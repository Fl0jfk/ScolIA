# Passe « personas » sur le socle 37 — Orientation et parcours

**Date :** 2026-11-11 — **Branche :** `ent-socle-37-38-conception`. **Objet :** éprouver `docs/socles/37_ORIENTATION.md` par des personnes réelles et variées (fictives, aucune donnée réelle), relever manques, contradictions et cas limites, et **reporter les ajouts dans la conception** (§28 du 37, marqué « ajout personas »). Conventions de la conception ([F], [V], [V-r], [NV], [H], [R]).

---

## 0. À lire en premier : trois bloquants et leur recommandation (appliquée par défaut, D-031)

| # | Bloquant | Recommandation appliquée |
|---|---|---|
| **B37-1** | Des adultes qui vivent avec l'élève **sans être titulaires** (beau-parent, famille d'accueil, ASE) : la conception ne disait pas ce qu'ils voient ni s'ils peuvent déclarer un vœu | ils **reçoivent** ce que le socle 4 leur ouvre, **n'écrivent rien** ; le tuteur ASE agit comme titulaire selon son mandat (R-OR29) |
| **B37-2** | La famille connaît le **résultat d'affectation avant l'établissement** (téléservice) ; la conception ne prévoyait que la saisie du secrétariat | la famille **déclare** son résultat ; marqué « déclaré par la famille » jusqu'à confirmation (R-OR30) |
| **B37-3** | La page publique de Next.js (ND-01) montre qu'un simple lien ou formulaire peut **énumérer** des élèves et révéler un résultat | aucune recherche d'élève par un tiers non authentifié ; liens nominatifs muets avant authentification, tentatives limitées (R-OR31) |

---

## 1. Méthode
1. Lecture de la conception, du 23 (§9, §28), du 21 (R-31), du 13 (M-1344) et de la ligne M17 de T0.
2. **Quatorze personas** (tableau §2) dont un **attaquant** et un **auditeur RGPD** ; chacun parcourt l'année d'orientation qui le concerne, action par action, en cherchant ce qui manque, ce qui contredit, ce qui fuit.
3. Chaque manque reçoit un numéro **M37-xx**, une gravité (B bloquant, I important, m mineur) et sa correction (action, règle, cas limite, modèle).
4. Aucune donnée réelle ; Next.js non exécuté.

---

## 2. Les personas en un tableau

| # | Persona | Contexte | Ce qu'il éprouve |
|---|---|---|---|
| P01 | **Mme Garcia**, PP de 3ᵉ B (28 élèves), collège privé sous contrat | première année avec ScolIA, Affelnet en mai | préparation des vœux, liste de saisie, relances |
| P02 | **M. Leroy**, secrétaire de scolarité, collège et lycée | saisit pour 40 familles, reçoit les résultats | liste de saisie, résultats, non-inscrits |
| P03 | **Mme Benoît** et **M. Benoît**, séparés, garde alternée | fils Noah en 3ᵉ, désaccord sur la voie | vœux, information, résultats |
| P04 | **Famille Yilmaz**, allophone, sans ordinateur | fille Elif arrivée en janvier en 3ᵉ | circuit papier, traduction, saisie accompagnée |
| P05 | **Lucas**, 17 ans, terminale, puis 18 ans en avril | Parcoursup, veut garder ses vœux pour lui | majorité, partage, fiche Avenir |
| P06 | **M. Fontaine** et **Mme Petit**, deux PP de terminale | 35 fiches Avenir en mars | co-rédaction, pré-remplissage, absence |
| P07 | **Mme Duval**, cheffe d'établissement | 900 avis par vœu en une semaine | avis préparés, cohérence, relecture |
| P08 | **Mme Rochat**, référente orientation | plan, forum des métiers, intervenants | plan, activités, présence, bilan |
| P09 | **Mme Kerbrat**, PsyEN (public) puis conseillère d'orientation (privé) | entretiens, secret | demandes, partage minimal |
| P10 | **Inès**, 15 ans, ULIS, fauteuil | cherche un lycée accessible | entretien avec le référent, information sans diagnostic |
| P11 | **Yanis**, 16 ans, affecté nulle part fin juin | non affecté, puis liste supplémentaire | sans-solution, suivi, clôture |
| P12 | **Famille d'accueil Morel** (ASE) et le service de l'ASE | Théo, 15 ans, parents titulaires injoignables | titulaires, information, vœux |
| P13 | **« Kévin »**, attaquant (ancien élève malveillant) | cherche l'e-mail de la mère d'une camarade et son lycée actuel | liens, questionnaires, énumération |
| P14 | **M. Blanc**, délégué à la protection des données (auditeur RGPD) | contrôle avant l'année de doublon | registre, durées, profilage, IA, accès |

---

## 3. Parcours et manques, persona par persona

### P01 — Mme Garcia, PP de 3ᵉ
Elle ouvre le tableau du PP en avril : 28 élèves, vœux définitifs du 23 arrivés, colonne « formation cible » vide pour 19. Elle associe les formations, voit 2 incohérences (vœu 2ᵈᵉ GT, décision 2ᵈᵉ pro). Elle relance 6 familles.
- **M37-01 (I)** Deux élèves ont un vœu vers un **lycée privé** : rien ne lui dit si la démarche directe est faite. → ORI-378, ORI-392.
- **M37-02 (I)** Un élève fait appel : ses vœux Affelnet sont-ils préparés quand même ? → « sous réserve de la commission » ORI-376, recalcul ORI-377, COR-04.
- **M37-03 (m)** Elle voudrait savoir combien d'élèves de sa classe visent le même lycée, sans fiche par fiche. → ORI-369 (agrégé).
- Contradiction relevée : **aucune**.

### P02 — M. Leroy, secrétariat
Il imprime la liste de saisie, saisit 40 familles dans l'application nationale, coche « saisi ». Le 30 juin, il reçoit les résultats.
- **M37-04 (B)** Les familles voient leur résultat sur le téléservice **avant** qu'il ait importé quoi que ce soit ; elles appellent : ScolIA affiche encore « saisi ». → **B37-2**, ORI-370, ORI-371, R-OR30.
- **M37-05 (I)** Une famille sans e-mail ni compte : il n'y a pas de courrier papier de rappel. → ORI-390, ORI-391.
- **M37-06 (m)** Reprendre la campagne de l'an passé (formations, structure des dates). → ORI-389.

### P03 — Les Benoît, séparés
Le père veut la voie pro, la mère la 2ᵈᵉ GT. Le conflit est résolu au 23 (direction : « avis du conseil »).
- **M37-07 (I)** Le père ne reçoit pas le résultat d'affectation : la notification n'était envoyée qu'au titulaire qui avait saisi. → ORI-365 (second titulaire), RG-O03.
- **M37-08 (m)** Ils n'ont pas le même calendrier personnel (garde alternée) : le calendrier est par élève, visible des deux. Pas de manque.
- Aucune contradiction : le 37 lit les vœux **définitifs** (COR-01).

### P04 — Famille Yilmaz
Elif arrive en janvier ; la famille ne lit pas le français écrit, pas d'ordinateur.
- **M37-09 (I)** Calendrier et rappels non traduits. → ORI-373 (socle 8).
- **M37-10 (I)** La saisie Affelnet par la famille suppose ÉduConnect : aucun rendez-vous de saisie accompagnée. → ORI-375 ; liste papier de ce qu'il faut apporter ORI-374.
- **M37-11 (m)** Procédures académiques pour allophones arrivés tard [NV] : COR-11, gardé [NV].

### P05 — Lucas, 17 puis 18 ans
Il déclare 3 vœux à son PP, pas les autres. Il devient majeur le 12 avril.
- **M37-12 (I)** À la majorité, ce qu'il avait partagé avec sa famille reste partagé sans qu'il le sache. → rappel annuel ORI-364 ; R-OR23 précisée (rien de privé ne devient visible ; le partagé reste révocable).
- **M37-13 (I)** Il consulte sa fiche Avenir le 1ᵉʳ juin et trouve une appréciation injuste : aucune voie de demande. → ORI-387 (demande de correction), lien Q-OR9.
- **M37-14 (m)** Il veut savoir pourquoi il reçoit un rappel « confirmez vos vœux » : → ORI-396 (« pourquoi je vois cette alerte »).

### P06 — M. Fontaine et Mme Petit, deux PP de terminale
Ils se partagent 35 fiches. Mme Petit est arrêtée deux semaines en mars.
- **M37-15 (B→I)** Délégation de la fiche d'un PP absent : prévue par COR-17 mais pas d'action. → ORI-381.
- **M37-16 (I)** Pré-remplissage : ils ne savent pas **d'où** vient chaque phrase (risque de citer une appréciation corrigée ensuite) ; ils veulent refuser une phrase sans qu'elle revienne. → ORI-382, ORI-383, modèle 2754.
- **M37-17 (I)** Vue commune des deux PP. → ORI-366.
- **M37-18 (m)** Une appréciation du bulletin est corrigée après validation : alerte. → ORI-384, COR-18.

### P07 — Mme Duval, cheffe d'établissement
Elle doit donner un avis pour chaque vœu (≈ 900).
- **M37-19 (I)** Les avis proposés depuis l'avis du conseil sont utiles, mais elle veut **appliquer un avis par type de formation** pour un élève (dix licences de droit), avec relecture des réservés. → ORI-385.
- **M37-20 (I)** Le conseil du 2ᵉ trimestre a été corrigé (avis réservé devenu favorable) après la préparation : → COR-50.
- **M37-21 (m)** Elle veut savoir qui a ouvert une fiche. → ORI-386.
- **Contradiction** : aucune avec le 23 (le 37 lit l'avis du conseil, ne le modifie pas).

### P08 — Mme Rochat, référente orientation
Elle construit le plan, organise le forum des métiers (20 intervenants, 300 élèves).
- **M37-22 (I)** Un intervenant se comporte mal : pas de moyen de **révoquer** son lien. → ORI-397.
- **M37-23 (m)** Elle veut archiver la campagne et produire une synthèse annuelle pour le conseil pédagogique. → ORI-388.
- **M37-24 (m)** Signaler une formation erronée du catalogue. → ORI-368.

### P09 — Mme Kerbrat, PsyEN puis conseillère du privé
- **M37-25 (I)** Dans le privé, la conseillère d'orientation n'est pas PsyEN (statut, secret [NV]) : la conception doit traiter le rôle « conseiller d'orientation de l'établissement » comme la psychologue pour le partage, ou comme un personnel ordinaire ? → **F37-4** (Florian) ; en attendant, **même règle de partage minimal** (R-OR21 étendue au conseiller).
- **M37-26 (m)** Elle voudrait joindre au suivi d'un sans-solution le compte rendu d'un échange avec le CIO. → ORI-367.

### P10 — Inès, ULIS, fauteuil
- **M37-27 (I)** Le référent handicap (35) doit pouvoir être convié à l'entretien d'orientation **avec l'accord de la famille**, sans que le 37 stocke le diagnostic. → ORI-379.
- **M37-28 (m)** Indiquer qu'une formation offre un accueil adapté (information générale, sans élève). → ORI-380.

### P11 — Yanis, non affecté
- Le 1ᵉʳ juillet, il apparaît dans les sans-solution ; responsable : son PP, en vacances.
- **M37-29 (I)** Le responsable désigné peut être absent en juillet : la conception doit proposer **un responsable de permanence** par défaut (direction ou secrétariat) sur la période estivale. → ajouté à ORI-301 (responsable par défaut paramétrable, S37-04 étendu).
- **M37-30 (m)** Place libérée en septembre : COR-08, déjà couvert.

### P12 — Famille d'accueil Morel et l'ASE
- **M37-31 (B)** **B37-1** : la famille d'accueil voulait déclarer les vœux. → R-OR29, ORI-372, COR-47.

### P13 — « Kévin », attaquant
Il essaie : (a) le questionnaire de devenir par un lien d'une camarade transféré ; (b) un formulaire de recherche ; (c) les liens d'invitation d'intervenant ; (d) l'IA (« dis-moi où est partie Léa »).
- **M37-32 (B)** (a) un lien nominatif ouvert par un tiers ne doit rien afficher avant authentification ; (b) aucun formulaire de recherche d'élève sans compte ; répétitions limitées. → **B37-3**, R-OR31, ORI-398, COR-52, modèle 2753.
- **M37-33 (I)** (c) le lien d'intervenant ne montre que l'activité, jamais la liste des élèves : R-OR25 confirmée.
- **M37-34 (I)** (d) l'IA a exactement ses droits (aucun) : refus ; jeu d'évaluation enrichi d'un cas « devenir d'un tiers ».
- **M37-35 (I)** Il signale ND-01 de Next.js lui-même : **F37-1** (en production aujourd'hui).

### P14 — M. Blanc, auditeur RGPD
- **M37-36 (I)** Registre : il manque l'export du registre des traitements du socle (finalités, bases légales, durées, destinataires). → ORI-393.
- **M37-37 (I)** Droit d'accès d'une personne à ses données d'orientation. → ORI-394.
- **M37-38 (I)** Purge effective selon §17.4 avec rapport. → ORI-395.
- **M37-39 (I)** Profilage : les alertes (rien suivi, sans-solution) sont-elles un profilage ? → signaux déterministes expliqués à l'élève (ORI-396), aucun score (Q-OR8) ; à valider par le DPO (F37-5).
- **M37-40 (I)** Année de doublon : des notifications partant de ScolIA en parallèle de Charlemagne dupliquent l'information et brouillent la source officielle. → mode doublon ORI-399, R-OR32, COR-51.
- **M37-41 (m)** Reprise de M17 : base légale de la conservation des situations reprises ? → Q-OR10 : seulement l'année en cours ; DPO (F37-5).

### Manques transverses : mobile, IA, tests
- **M37-42 (m)** Mobile : pointer la présence d'un forum de 300 élèves sans réseau → pointage hors ligne (socle 11), synchronisation, conflits signalés.
- **M37-43 (m)** Tests : ajouter à la recette le cas « résultat déclaré par la famille puis contredit par le fichier ».

---

## 4. Vérification réglementaire (2026-11-11)
Recherche reprise sur les points soulevés :
- Rôle du conseiller d'orientation dans le privé et secret : **[NV]** (aucune source officielle lisible) → F37-4.
- Accès de l'élève à la fiche Avenir avant juin : **[NV]** (seule la date de consultation dans Parcoursup est [V-r]) → F37-6.
- Statut de l'ASE et du tuteur : renvoi aux règles du socle 4 (non revérifié ici).

---

## 5. Manques qui relèvent d'autres socles
| Manque | Socle | Proposition |
|---|---|---|
| Mode « fiche tenue dans le téléservice » (public) | 23 | report des vœux par l'établissement (C2 du 37) |
| Pointage hors ligne de grandes activités | 11 | file locale, synchronisation |
| Traduction des avis | 8 | langue préférée du titulaire |
| Référent handicap convié avec accord | 35 | port « convier le référent » |
| Mise à jour de T0 l. 207 (fiches de dialogue au 23) | T0 | à faire par la session qui maintient T0 |

---

## 6. Synthèse

### 6.1 Manques par gravité
- **Bloquants : 3** (M37-04, M37-31, M37-32 → B37-2, B37-1, B37-3), tranchés.
- **Importants : 27.**
- **Mineurs : 13.**
Total : **43 manques**.

### 6.2 Ajouts dans la conception
§28 du 37 : **36 actions** (ORI-364 à ORI-399), **4 règles** (R-OR29 à R-OR32), **6 cas limites** (COR-47 à COR-52), **2 questions** (Q-OR15, Q-OR16), **2 migrations** dans la réserve (2753, 2754).

### 6.3 Les dix ajouts les plus importants
1. Résultat déclaré par la famille, confirmé par le secrétariat (R-OR30).
2. Liens muets avant authentification, tentatives limitées (R-OR31) — leçon de ND-01.
3. Adulte hébergeant non titulaire : reçoit, n'écrit pas (R-OR29).
4. Mode doublon : rien n'est envoyé aux familles sans décision (R-OR32).
5. Délégation de la fiche Avenir d'un PP absent (ORI-381).
6. Origine de chaque phrase pré-remplie, refus définitif (ORI-382, 383).
7. Avis du chef par type de formation, réservés relus (ORI-385).
8. Second titulaire notifié du résultat d'affectation (ORI-365).
9. Circuit papier et saisie accompagnée (ORI-374, 375, 390, 391).
10. Registre, droit d'accès, purge (ORI-393 à 395).

### 6.4 Ce que la passe a montré
La conception tenait bien la frontière avec le 23 (aucune contradiction trouvée) ; ses faiblesses étaient **les personnes en marge** (familles sans numérique, adultes non titulaires, PP absents, l'été) et **l'exposition** (liens, recherche, doublon).

---

## 7. Décisions

### 7.1 Appliquées par défaut (D-031)
B37-1, B37-2, B37-3 ; Q-OR15 (pas de catalogue partagé entre établissements) ; Q-OR16 (carnet partagé limité au PP et aux professeurs de l'élève) ; responsable de permanence estivale (M37-29) ; conseiller d'orientation du privé traité comme la psychologue pour le partage, en attendant F37-4.

### 7.2 Soumis à Florian
F37-1 (ND-01, **urgent**, production) ; F37-4 (conseiller d'orientation du privé) ; F37-5 (DPO : profilage, reprise M17) ; F37-6 (juriste : accès anticipé à la fiche Avenir).

---

## 8. Faits, hypothèses, contradictions
**Faits** : ND-01 relu dans `app/lib/invitation-db.ts:645-750` (lecture seule). **Hypothèses** : comportements des familles, volumes (900 avis). **Contradictions** : aucune nouvelle ; C1 à C5 du 37 inchangées.
