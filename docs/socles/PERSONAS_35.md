# Passe « personas » sur le socle 35 — Besoins éducatifs particuliers et inclusion

**Date :** 2026-11-11 — **Statut :** proposition, non décidée ; recommandations appliquées par défaut (D-031), sauf ce qui engage l'établissement à l'extérieur (§6.2). Les ajouts sont versés au **§30 de `35_INCLUSION.md`**, qui l'emporte sur le texte d'origine.
**Branche :** `ent-socle-35-36-conception`. Aucun code écrit ; aucun accès à une base, à un secret ni à Next.js (le module d'inclusion n'existe pas dans Next.js, `35_INCLUSION.md` §2.4).
**Personas fictifs**, composés à partir des acteurs du §3 ; aucune personne réelle.

---

## 0. À lire en premier : quatre bloquants et leur recommandation (appliquée par défaut)

1. **L'AESH sans compte est le premier manque fonctionnel** (DINC-06) : sans lien à un agent du socle 6, il n'y a ni planning, ni alerte de changement, ni convocation. **Recommandation** : IN4 fait partie du parcours minimal de la V1 ; la reprise des noms en texte passe par un écran de correspondance, jamais automatique (MIG-5). *Appliqué.*
2. **Le remplaçant de professeur et la remplaçante d'AESH** n'ont rien tant que l'accès est calculé sur les classes « à moi » : **R-INC37** et **R-INC32** calculent l'accès sur la **séance** et la **plage**. *Appliqué.*
3. **L'export de session d'examen** est le seul export nominatif hors du référent ; sans règle, il devient la liste « tous les élèves handicapés » qui circule par clé USB. **R-INC33** (filigrane, journal, sans dispositif). *Appliqué.*
4. **Le public tient le document dans le LPI** : saisir deux fois ruinerait l'adoption. **R-INC22** et S-2700 ; l'interface LPI est une démarche officielle (**soumis à Florian**, Q-INC3).

---

## 1. Méthode
Pour chaque persona : un parcours réel sur une année, confronté à la conception (§9, §13, §15 de `35_INCLUSION.md`), les **manques** (M-xxx), les **contradictions** et les **cas limites** ; chaque manque reçoit une action (INC-300 et suivantes), une règle (R-INC31 et suivantes) ou un renvoi à un autre socle. Deux personas sont hostiles (attaquants) et un est auditeur RGPD.

## 2. Les personas en un tableau
| # | Persona | Situation | Ce qu'il éprouve |
|---|---|---|---|
| P01 | Mme Morel, référente inclusion (CPE) d'un collège privé sous contrat | 41 élèves à dispositif, aucune infirmière à plein temps | rentrée sous les tableurs |
| P02 | M. Faure, enseignant référent (Éducation nationale), 14 établissements | ESS à caler, GEVA-Sco | « je n'ai jamais les bilans à temps » |
| P03 | Mme Diallo, AESH mutualisée sur trois élèves et deux établissements | changements de salle non annoncés | « je l'apprends par l'élève » |
| P04 | M. Haddad, professeur de mathématiques, 160 élèves | ne sait pas qui a quoi | peur d'être accusé |
| P05 | Mme Roche, professeur d'EPS | confusion inaptitude / aménagement | sécurité |
| P06 | Mme Petit, coordonnatrice ULIS | inclusions refaites à chaque EDT | professeurs d'accueil oubliés |
| P07 | M. Nguyen, enseignant d'UPE2A | élèves arrivant en cours d'année | familles non francophones |
| P08 | M. et Mme Garnier, séparés, père sans autorité parentale depuis le jugement | Lucas, 13 ans, PAP | conflit ; la mère redoute que le père soit informé |
| P09 | Inès, 17 ans puis 18 ans, PPS, terminale | bac, Parcoursup | « je veux décider seule » |
| P10 | Mme Duval, cheffe d'établissement | signatures de rentrée, examens | recours d'une famille |
| P11 | Mme Lambert, secrétaire | dossiers d'examen | erreur de date limite |
| P12 | Yanis, remplaçant contractuel arrivé lundi | trois classes avec élèves aménagés | aucun accès |
| P13 | M. Blanc, délégué à la protection des données | AIPD, registre | liste exportable |
| P14 | Attaquant 1 : un AED curieux | compte vie scolaire | veut savoir « qui est handicapé » |
| P15 | Attaquant 2 : compte parent compromis | session volée | veut lire le dossier d'un autre enfant |
| P16 | Florian, administrateur, avec l'IA | démonstration | ne veut rien montrer de réel |

---

## 3. Parcours et manques, persona par persona

### P01 — Mme Morel, référente inclusion
**Parcours** : fin août, elle reçoit les affectations d'AESH par courriel du service départemental ; 1re semaine, elle diffuse les mesures ; septembre, PAP à reconduire et campagne du DNB ; janvier, sept ESS ; juin, liaison avec trois écoles.
**Ce qui marche** : tableau de bord (INC-260, INC-261), reconduction (INC-127), campagne (INC-215 à INC-222).
**Manques** : M-01 elle part en congé maladie trois semaines en novembre → **suppléant du référent** (INC-313) ; M-02 un élève quitte l'établissement en février, elle doit clore, proposer la liaison, retirer les mesures : **un geste** (INC-312) ; M-03 un nouveau professeur arrive en janvier, ne lit pas les mesures → **alerte après sa première séance** (INC-329) ; M-04 deux élèves ont besoin d'une salle au rez-de-chaussée : l'EDT l'ignore → **contrainte de salle sans motif** (INC-330, R-INC31) ; M-05 elle crée par erreur un dossier pour un homonyme → **retrait complet** à double validation (INC-331, CINC-48) ; M-06 sans infirmière, l'avis médical du PAP vient du médecin de secteur par papier → l'avis est saisi « reçu sur papier, date, médecin » par la référente, pièce **vue non stockée** (aligné mode sans infirmière du 34, R-S43).
**Contradiction** : la conception donne `inclusion.signer_chef` à la direction seule ; un établissement à deux sites a un **chef d'établissement adjoint** qui signe : la portée « établissement » couvre l'adjoint par la fonction du 6 — aucune modification.

### P02 — M. Faure, enseignant référent
**Parcours** : 14 établissements, 160 élèves ; il convoque, réunit, rédige le GEVA-Sco de réexamen dans le LPI (public) ou sur papier (privé).
**Manques** : M-07 il travaille dans **deux organisations clientes** de ScolIA : une identité, une casquette d'invité par établissement (CINC-38, aligné CS-59 du 34) ; M-08 il veut **demander les bilans** des professeurs trois semaines avant : INC-202 existe ; ajouter la **relance** automatique à J-7 (inclus dans INC-202) ; M-09 visio pour les familles éloignées (INC-303) ; M-10 son invitation expire en juin alors que l'ESS de réexamen est en juin : invitation **à l'année scolaire + 1 mois** par défaut et revue mensuelle des invités (INC-321).
**Cas limite** : il ne doit **pas** voir les élèves non désignés du même établissement (R-INC14) ; test du piège 9.

### P03 — Mme Diallo, AESH mutualisée
**Parcours** : lundi, Léo (6e A, maths et français) et Sarah (5e C, EPS) ; mardi, Tom au collège voisin ; pause méridienne avec Léo.
**Ce qui marche** : planning (INC-183), changements (INC-184), cahier de textes (INC-185).
**Manques** : M-11 la **remplaçante** d'AESH, mardi, n'a aucun accès → accès temporaire sur les plages remplacées (INC-306, R-INC32) ; M-12 une **sortie** au musée avec Léo : planning et consignes hors ligne (INC-307, lien 42) ; M-13 elle a une **formation** jeudi : indisponibilité de plage (INC-308) ; M-14 elle veut savoir si ses heures planifiées dépassent celles de son contrat (INC-309, pour elle seule) ; M-15 elle repère une **porte trop lourde** pour un fauteuil : signalement d'accessibilité (INC-310) ; M-16 la **cantine** doit savoir qu'elle est à table avec Léo, sans savoir pourquoi (INC-325, R-INC36) ; M-17 son arrivée en novembre : **fiche d'accueil** (INC-322).
**Cas limites** : CINC-37 (détresse d'un élève → socle 36, rien au 35) ; CINC-40 (lien familial avec l'élève).
**Contradiction** : `06_PERSONNELS.md` §775 donne à l'AESH « `dossier.lire` réduit » ; la conception du 35 ne lui donne **pas** la fiche élève au-delà de l'identité, de la photo et de la classe ; **le 35 l'emporte** sur ce point (aucune section scolarité, ni famille) — à reporter au 6.

### P04 — M. Haddad, professeur de mathématiques
**Parcours** : 1re semaine, il ouvre « Mesures de ma classe » ; il donne un devoir (délai majoré appliqué à Léo) ; contrôle chronométré : impossible de donner le tiers-temps à la fin de l'heure.
**Manques** : M-18 il voudrait recopier dans son carnet → **impression filigranée** seulement (CINC-44) ; M-19 il ne sait pas si la mesure « calculatrice » vaut au **brevet blanc** : la portée « examen » répond (INC-223) ; M-20 il craint d'être noté sur S-2706 : agrégat seulement, jamais transmis à la direction comme évaluation (R-INC23, confirmé).
**Cas limite** : CINC-07 (impossible aujourd'hui) — la famille voit le signalement sans son nom (R-INC23).

### P05 — Mme Roche, professeur d'EPS
**Parcours** : Sarah a « consignes démontrées » (35) et « pas d'activité en hauteur » (consigne du 34).
**Manques** : M-21 les deux sources doivent apparaître **dans la même liste** de la séance, sans dire laquelle vient de la santé : l'écran de la séance agrège `inclusion::mesures_seance` et `sante::consignes(audience professeurs)` — **chacun garde sa source cachée** (règle à reporter aux deux socles, déjà R-S3 au 34).
**Contradiction** : le catalogue du 35 ne doit **pas** contenir d'interdits physiques (domaine du 34) : test croisé des deux catalogues (aucun code commun).

### P06 — Mme Petit, coordonnatrice ULIS
**Parcours** : 12 élèves, inclusions dans 9 classes ; changement d'EDT en octobre.
**Manques** : M-22 l'emploi du temps d'inclusion doit être recalculé et les **professeurs d'accueil** prévenus (INC-231, INC-232, CINC-13 — confirmés) ; M-23 un élève d'ULIS part en **voyage avec sa classe de référence** (CINC-45) ; M-24 elle veut **copier les mesures** d'un PAP vers le PPS qui le remplace (INC-311).

### P07 — M. Nguyen, enseignant d'UPE2A
**Manques** : M-25 la famille lit l'ourdou : **langue des convocations** (INC-302), modèles validés, jamais de traduction automatique d'un document signé (R-INC24) ; M-26 l'UPE2A **ferme** en juin (CINC-46).
**Point RGPD** : aucune nationalité, aucun pays d'origine (R-INC25) ; la **langue** des convocations est un choix de la famille, pas une donnée d'origine.

### P08 — Famille Garnier
**Parcours** : la mère a l'autorité parentale exclusive depuis le jugement ; le père a un compte parent hérité.
**Ce qui marche** : R-INC8 (titulaires seulement), CINC-11.
**Manques** : M-27 la mère veut qu'une **grand-mère** reçoive les convocations : **personne de confiance** en lecture seule, sans signature ni notification MDPH (INC-304, R-INC35) ; M-28 la mère veut voir **qui a lu** le dossier de Lucas (INC-300 : par catégorie de lecteurs, sans noms de professeurs) ; M-29 si le juge rend l'autorité au père (CINC-39), accès ouvert **à la date**, signature proposée à la révision.

### P09 — Inès, PPS, 17 puis 18 ans
**Parcours** : aménagements du bac demandés en 2de, reconduits ; elle a 18 ans en février.
**Manques** : M-30 elle veut **reconduire** ses aménagements pour le BTS (INC-328) ; M-31 elle veut voir **les mesures qu'elle peut demander** elle-même (INC-326) ; M-32 à 18 ans, elle rouvre l'accès « convocations » à sa mère seulement (INC-274, confirmé).
**Cas limite** : CINC-12, CINC-33 confirmés.

### P10 — Mme Duval, cheffe d'établissement
**Manques** : M-33 elle veut signer **en série** les PAP de rentrée, chacun ouvert au moins une fois (aligné SAN-314 du 34) : ajouté à INC-123 (« signature en série, confirmation par dossier ») ; M-34 un recours d'une famille : **relevé de conformité** (INC-263) et **historique des mesures** (INC-327).
**Point de vigilance** : elle **ne lit pas** les comptes rendus d'ESS auxquels elle n'a pas participé (§8.1) ; elle peut se faire inscrire participante ; c'est un changement d'usage par rapport à l'existant (DINC-01) : **signalé**.

### P11 — Mme Lambert, secrétaire
**Manques** : M-35 la plateforme académique demande une **liste** : export de session (INC-315, R-INC33) ; M-36 elle ne doit voir **ni la nature** du dispositif **ni** les élèves hors session (confirmé R-INC7).

### P12 — Yanis, remplaçant
**Manques** : M-37 désigné à 7 h 45 pour 8 h : mesures de la séance **dès la désignation**, retirées après (R-INC37) ; sans compte : impression par la vie scolaire (INC-171, CINC-14).

### P13 — M. Blanc, DPO (auditeur RGPD)
**Questions qu'il pose** et réponses :
1. *Base légale* : mission d'intérêt public (6.1.e) et obligations légales (L112-1, D311-13, D351-*) ; art. 9.2.g pour les mesures imposées par la loi, 9.2.h pour l'avis médical [H] → **à écrire au registre** (soumis au DPO, Q-INC4).
2. *AIPD* : oui, commune avec le 34 (délibération 2018-327 : personnes vulnérables) → **dossier d'avis** (INC-318).
3. *Minimisation* : professeur sans nature de dispositif, secrétariat sans liste, AESH sans notes (§15) → **tests** demandés en recette.
4. *Droits* : accès (INC-271), rectification (nouvelle version), opposition (INC-280) ; **droit d'accès aux notes d'AESH** : communication sous réserve des droits d'autrui (CINC-43).
5. *Durées* : §16, à valider ; **effet des sauvegardes** : destruction rejouée (INC-273), mention dans la notice.
6. *Sous-traitants* : aucun prestataire ne reçoit une donnée du 35 ; e-mails en enveloppe neutre (R-INC12).
7. *Fuite* : périmètre calculable (INC-319) ; lecture massive détectée (INC-320, R-INC34).
8. *Transferts hors UE* : aucun (hébergement Scaleway France, D-034).
**Manques relevés** : M-38 dossier d'avis du DPO (INC-318) ; M-39 calcul de fuite (INC-319) ; M-40 détection de lecture massive (INC-320).

### P14 — Attaquant 1 : un AED curieux
**Tentatives** : (a) ouvrir `/eleves/{id}/dossier` d'un élève qu'il soupçonne : la section dispositif n'existe plus hors cercle (DINC-04 corrigé) ; (b) lire les mesures via l'appel d'une séance qu'il surveille en permanence : il voit les mesures **de la séance de permanence** seulement si une mesure a la portée « cours » — **décision** : la permanence n'est pas un cours, aucune mesure n'y est servie sauf « aide à l'évacuation » et « accompagnant présent » ; (c) faire des captures d'écran : écran sécurisé mobile (§19) ; (d) demander à l'IA « quels élèves ont un tiers-temps » : refus (R-INC19) ; (e) lister par des requêtes rapides : lecture massive (R-INC34).
**Manque** : M-41 la portée « séance » doit exclure les **permanences et études** : ajouté à R-INC37.

### P15 — Attaquant 2 : compte parent compromis
**Tentatives** : (a) changer l'identifiant d'élève dans l'URL : contrôle de lien avec autorité côté service (R-INC8), 403 sans indiquer si l'élève existe ; (b) rejouer une signature : version figée et empreinte (R-INC9) ; (c) se déclarer personne de confiance d'un autre enfant : seule une titulaire peut déléguer (INC-304) ; (d) exfiltrer par l'export RGPD : export du seul enfant lié, journalisé, réauthentification par passkey (D-009) ; (e) faire envoyer des convocations à une autre adresse : adresses du socle 4, modification soumise à confirmation.
**Manque** : M-42 aucune alerte à la titulaire quand une **délégation** est créée depuis un nouvel appareil → notification « une personne de confiance a été ajoutée » à chaque titulaire (ajouté à INC-304).

### P16 — Florian, démonstration de décembre 2026
**Manques** : M-43 aucune donnée réaliste : seed avec mesures et dispositifs **fictifs**, sans trouble nommé (R-INC28, CINC-47) ; M-44 montrer le parcours AESH sur le labo sans données réelles.

---

## 4. Manques transverses
- **IA** : aucune capacité sur le 35 en dehors du PPRE et des supports C0 ; test « IA sans 35 » à IN0 (confirmé).
- **Mobile** : AESH en premier (planning, changements, sortie hors ligne) ; écran sécurisé.
- **Autres socles** : 6 (AESH : pas de `dossier.lire` au-delà de l'identité, P03) ; 10 (contraintes de salle sans motif, INC-330 ; permanence exclue, P14) ; 14 (aide à l'évacuation) ; 15 (remplaçante d'AESH) ; 21 (corriger « 35 possède le PAI ») ; 34 (catalogues sans code commun ; liste fusionnée de la séance sans source, P05) ; 40 (« accompagnant présent ») ; 42 (besoins de sortie).

## 5. Vérification réglementaire (2026-11-11)
Reprise de `35_INCLUSION.md` §5 ; points de vigilance ajoutés : (a) la **généralisation des PAS** a été rejetée le 11/05/2026 mais l'expérimentation continue : neutralité (Q-INC14) ; (b) la **pause méridienne** est prise en charge par l'État (loi 2024-475) — l'AESH à la cantine est un accompagnement, pas une surveillance de la restauration ; (c) le **droit d'accès** aux notes d'AESH suit l'article 15 RGPD sous réserve des droits d'autrui [H, DPO].
Sources consultées le 2026-11-11 (extraits) : https://www.banquedesterritoires.fr/ecole-inclusive-lassemblee-nationale-cree-les-poles-dappui-la-scolarite · https://www.weka.fr/actualite/?p=214106 · https://www.senat.fr/questions/base/2026/qSEQ260508943.html · https://www.banquedesterritoires.fr/la-loi-sur-la-prise-en-charge-par-letat-des-aesh-sur-le-temps-meridien-ete-publiee · https://questions.assemblee-nationale.fr/dyn/17/questions/QANR5L17QE15832.pdf · https://www.education.gouv.fr/bo/20/Hebdo47/MENE2034197C.htm · https://eduscol.education.gouv.fr/5535/reglementation-concernant-les-amenagements-d-examens-pour-les-eleves-en-situation-de-handicap · https://eduscol.education.gouv.fr/5436/le-livret-de-parcours-inclusif-lpi · https://www.ih2ef.gouv.fr/deploiement-du-livret-de-parcours-inclusif-lpi.

## 6. Questions
### 6.1 Appliquées par défaut (D-031)
| # | Question | Recommandation |
|---|---|---|
| Q-INC23 | Permanences et études exclues de la portée « séance » | **Oui** |
| Q-INC24 | AESH sans `dossier.lire` au-delà de l'identité (contre `06` §775) | **Oui** ; à reporter au 6 |
| Q-INC25 | Direction : comptes rendus d'ESS seulement si participante | **Oui** ; changement d'usage signalé |
| Q-INC26 | Personne de confiance en lecture seule, alerte aux titulaires | **Oui** |
### 6.2 Soumises à Florian
Q-INC2 (référents réels, dont une AESH et un enseignant référent), Q-INC3 (LPI), Q-INC4 et Q-INC16 (DPO : AIPD, durées), Q-INC5 (avis médical du PAP dans le privé : juriste).

## 7. Bilan
44 manques (M-01 à M-44) → 32 actions (INC-300 à INC-331), 7 règles (R-INC31 à R-INC37), 12 cas limites (CINC-37 à CINC-48), 2 réglages (S-2709, S-2710), 4 questions (Q-INC23 à Q-INC26), 3 contradictions (06 §775, 21 §6, direction et ESS).
