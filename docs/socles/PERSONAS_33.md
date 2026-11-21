# Passe « personas » sur le socle 33 — Visioconférence et continuité pédagogique

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** ; recommandations appliquées par défaut (D-031) sauf ce qui engage l'établissement à l'extérieur (§6.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production ; Next.js lu seulement.
**Branche :** `ent-socle-32-33-conception`.
**Éprouve :** `docs/socles/33_VISIO.md` (§0 à §29). **Ajouts** versés au **§30** de la conception, qui l'emporte en cas de contradiction.
**Numérotation :** manques **M-3301 à M-3328** ; actions **VIS-246 à VIS-275** ; règles **R-VI38 à R-VI42** ; cas limites **CVI-51 à CVI-70**.

---

## 0. À lire en premier : quatre bloquants (recommandations appliquées par défaut, D-031)

| # | Point | Pourquoi c'est bloquant | Recommandation appliquée | Où |
|---|---|---|---|---|
| **A** | L'**élève durablement à distance** (hospitalisé, à domicile, plusieurs semaines) n'existe que séance par séance (VIS-164) (M-3301) | Le professeur doit penser à l'ouvrir à chaque cours ; l'élève est oublié ; et la tentation est d'écrire le motif pour justifier | **Dispositif « suit à distance » daté, sans motif**, qui ouvre seul les salles des cours concernés (R-VI38, VIS-246 à VIS-248) | §30.1 A |
| **B** | Le **matin de la fermeture** (neige, 6 h 30), l'activation complète (périmètre, scénario, séances, annonces) ne tient pas sur un téléphone (M-3306) | La direction prévient alors par SMS personnels et réseaux sociaux, hors de tout cadre ; les familles apprennent la fermeture trop tard | **Activation en deux temps** : « Fermeture aujourd'hui » en un geste, organisation fine dans la journée (R-VI39, VIS-249, VIS-250) | §30.1 B |
| **C** | L'**attaquant** se fait passer pour un parent invité sans compte : le lien nominatif est transférable (M-3315) | Un ex-conjoint écarté (socle 4) ou un inconnu assiste à un rendez-vous sur un enfant | **Code** envoyé au moyen de contact connu du cœur, jamais à une adresse saisie sans vérification ; l'hôte voit la méthode de vérification (R-VI40, VIS-264) | §30.1 C |
| **D** | L'**auditeur RGPD** : conserver un an les entrées et sorties détaillées de chaque élève dans chaque visio est disproportionné (M-3320) | Profil de connexion d'un mineur, réutilisable à d'autres fins | **30 jours** pour le détail ; seul le résumé de présence passe au 12 ; le journal de sécurité ne garde que les anomalies (R-VI41, VIS-267) | §30.1 D |

**Rappel urgent pour Florian (hors ScolIA)** : les appels de la messagerie Next.js utilisent le STUN public de Google (ND33-01, Q-VI4) : l'IP des personnels qui appellent est transmise à Google. Le pilote ne touche pas Next.js.

---

## 1. Méthode
Celle de `PERSONAS_32.md` : quatorze personas, public et privé, collège et lycée, avec un **attaquant** et un **auditeur RGPD**, plus un persona « jour de crise ». Grille : ce qu'elle fait aujourd'hui, ce qu'elle retrouve, manques, peurs, cas limites, minimisation.

## 2. Les personas
| # | Persona | Établissement | Ce qu'elle éprouve surtout |
|---|---|---|---|
| P01 | **M. Garnier**, professeur de mathématiques, classes virtuelles pendant une fermeture | lycée pilote | animation, sous-groupes, appel |
| P02 | **Mme Okafor**, professeure d'allemand, option suivie à distance par 3 élèves d'un autre établissement | lycée public | extérieurs, élèves d'ailleurs |
| P03 | **Mme Duval**, cheffe d'établissement, le matin de la neige | lycée pilote | fermeture en un geste |
| P04 | **M. Haddad**, adjoint, réorganise les séances | lycée pilote | séances en lot, demi-groupes, capacité |
| P05 | **Mme Benali**, CPE, suit les élèves non joints | collège public | contact hebdomadaire, prioritaires |
| P06 | **Lucas**, 15 ans, hospitalisé 10 semaines | collège public | dispositif « à distance », discrétion |
| P07 | **Famille Nguyen**, allophone, rendez-vous parents-professeurs | collège public | interprète, sans compte |
| P08 | **M. Roussel**, père séparé ; l'autre parent a une adresse protégée | lycée pilote | rendez-vous séparés |
| P09 | **Mme Morel**, psychologue de l'éducation nationale | collège public | confidentialité, E2EE |
| P10 | **Délégués de classe** au conseil de classe hybride | lycée pilote | sortie pendant les cas individuels |
| P11 | **Clara**, 13 ans, ne veut pas montrer sa chambre | collège public | caméra, flou, pression des pairs |
| P12 | **M. Blanc**, DPO (auditeur RGPD) | lycée pilote | métadonnées, enregistrement, sous-traitants |
| P13 | **Attaquants** : élève perturbateur, ancien élève, ex-conjoint, saturation | — | intrusion, usurpation, déni de service |
| P14 | **Exploitation ScolIA** un lundi de fermeture régionale | plateforme | capacité, panne |

## 3. Parcours et manques

### P01 — M. Garnier
- Ses séances passent en `distance`, les salles sont dans « Aujourd'hui » ✔ ; appel pré-rempli ✔.
- **M-3302 (important)** : il teste son matériel la veille → salle d'essai personnelle VIS-252.
- **M-3303 (important)** : un élève lui écrit « je ne vous entends pas » → aide technique depuis la salle (VIS-253), indicateur de qualité (VIS-255).
- **M-3304 (confort)** : salle vide 15 min après le début → avertissement (VIS-274).
- Sous-groupes depuis les groupes de l'espace (32) ✔ ; il entre et sort, les élèves le voient (R-VI15) ✔.
- **M-3305 (important)** : cours hybride (demi-classe en salle, demi-classe à la maison) : la caméra de salle filme qui ? → R-VI42 (tableau et professeur par défaut, élèves informés) ; VIS-273.

### P02 — Mme Okafor, option à distance
- Les élèves d'un autre établissement ne peuvent pas avoir de compte chez elle (D-013) : ils entrent comme **extérieurs** dans une salle avec ses propres élèves → R-VI18 (adulte de l'établissement présent : elle) ✔ ; **M-3307 (important)** : leur présence doit être transmise à **leur** établissement : hors ScolIA en V1 (convention, Q-VI10) ; export simple de la présence des invités pour la séance (ajouté à VIS-162 : « présence des invités exportable »). Appliqué.

### P03 — Mme Duval, le matin de la neige
- **M-3306 (bloquant B)** ✔.
- **M-3308 (important)** : elle n'est pas joignable → délégation à l'adjoint (CVI-52) ✔.
- **M-3309 (important)** : fermeture décidée la veille au soir (CVI-53) ✔.
- **M-3310 (important)** : plan non validé (CVI-54) → trame par défaut ✔ ; état de préparation au tableau de bord de rentrée (VIS-251).

### P04 — M. Haddad, adjoint
- Séances en lot (VIS-211) ✔ ; demi-groupes (VIS-212) ✔.
- **M-3311 (important)** : 40 classes virtuelles à 8 h simultanées saturent le serveur → capacité visible avant d'activer (VIS-268), décalage d'horaires proposé (VIS-269), repère 1/3 synchrone rappelé ✔.
- **M-3312 (important)** : élèves sans équipement accueillis sur place : liste, salle, encadrant (VIS-270) et appel du 12 ✔.

### P05 — Mme Benali, CPE
- Tableau des non joints le jeudi ✔ ; prioritaires sans motif (R-VI28) ✔.
- **M-3313 (important)** : les familles signalent elles-mêmes l'absence d'équipement → VIS-271 vers les prêts (44).
- **M-3314 (important)** : un élève non joint trois semaines (CVI-31) : escalade ✔ ; le logiciel ne déclenche jamais une information préoccupante (36) ✔.

### P06 — Lucas, hospitalisé
- **M-3301 (bloquant A)** ✔.
- Il ne veut pas que ses camarades sachent : la salle d'une séance ordinaire, son nom parmi les autres, aucune mention « hospitalisé » ✔.
- **M-3316 (important)** : fatigue : il suit deux cours par jour → le dispositif porte sur des **cours choisis** ✔.
- **M-3317 (confort)** : agrandir le support partagé (VIS-256).

### P07 — Famille Nguyen, allophone
- Rendez-vous en visio sans compte, code par SMS ✔ ; interprète (VIS-184) ✔ ; vidéo de l'interprète épinglée (VIS-257).
- **M-3318 (important)** : réunion parents-professeurs : passer de salle en salle avec un temps d'attente estimé → VIS-258 à VIS-260.

### P08 — M. Roussel, séparé, adresse protégée de l'autre parent
- Rendez-vous séparés par défaut (CVI-18) ✔ ; aucune adresse ni numéro de l'autre parent visible dans la salle (nom affiché seulement) ✔.
- **M-3319 (important)** : la liste des participants d'un rendez-vous **commun** ne doit révéler aucune coordonnée → noms seulement, vérification par code masquée (R-VI40) ✔.

### P09 — Mme Morel, psychologue
- Permanence C3, E2EE, titre neutre, aucune IA, aucun enregistrement (R-VI34) ✔.
- **M-3321 (important)** : transférer un élève de sa salle vers celle de l'infirmière avec son accord → VIS-261 (transfert, contexte valide pour la cible) ; permanences à heures fixes (VIS-262).

### P10 — Délégués au conseil hybride
- Sortie pendant les cas individuels (VIS-187) ✔.
- **M-3322 (important)** : un délégué parent à distance enregistre avec son téléphone (CVI-21) : non empêchable ; rappel à l'ouverture, charte (VIS-275).

### P11 — Clara, 13 ans
- Caméra éteinte permise, jamais imposée (R-VI7) ✔ ; flou local ✔.
- **M-3323 (important)** : pression des camarades (« allume ta caméra ») dans le chat → chat sans messages entre élèves… mais le chat de salle est public : le professeur modère, et la charte (VIS-275) le dit ; aucune mise en avant de « qui a sa caméra » dans l'interface (ajout : pas d'indicateur collectif de caméras allumées). Appliqué.
- **M-3324 (confort)** : elle veut savoir si elle est enregistrée → indicateur permanent ✔.

### P12 — M. Blanc, DPO
- **M-3320 (bloquant D)** ✔.
- **M-3325 (important)** : sous-traitants : l'hébergeur seul ; le serveur média est exploité par ScolIA ; **aucun** STUN tiers (R-VI30) ✔ ; registre (CVI-67).
- **M-3326 (important)** : droit d'accès aux données de connexion (VIS-266) ✔.
- **M-3327 (important)** : base légale de l'enregistrement : la conception l'adosse à l'accord du 18 ; il rappelle qu'un « consentement » dans une relation d'autorité (établissement–élève) est fragile : il faut une **alternative réelle** (participer sans être capté) → R-VI23 la garantit ✔ ; Q-VI6 maintenue.
- Filigrane (Q-VI8) : il est nominatif mais visible seulement par l'utilisateur lui-même : pas de diffusion de donnée ✔.

### P13 — Attaquants
- Élève perturbateur : micro et caméra coupés à l'entrée, exclusion, verrouillage (VIS-124 à VIS-128) ✔.
- Ancien élève avec compte encore actif (CVI-11) ✔.
- **M-3315 (bloquant C)** ✔ ; code volé (CVI-59) ✔.
- Liens piégés dans le chat (CVI-60) ✔ ; saturation d'une salle d'attente (CVI-61) ✔ ; déni de service TURN (CVI-62) ✔ ; jeton volé et double connexion (CVI-45, VIS-265) ✔.
- **M-3328 (important)** : le client JavaScript tiers est une **dépendance de chaîne d'approvisionnement** : version figée par empreinte, servie par nous, mises à jour après revue (R-VI32) ✔ ; audit de licences des dépendances au lot VI0.

### P14 — Exploitation, lundi de fermeture régionale
- Capacité réservée (VIS-244), gel des mises à jour pendant une activation (CVI-68) ✔.
- Panne du serveur média (CVI-35) : bascule vers l'autonomie ✔ ; pas de service de secours externe sans accord.

## 4. Manques qui relèvent d'autres socles
| Manque | Socle | Proposition |
|---|---|---|
| Dispositif « suit à distance » : organisation et motif | 34, 35 | le motif reste chez eux ; le 33 n'en reçoit que les dates et les cours |
| Présence d'élèves extérieurs | 9 / conventions | export simple ; interface entre établissements plus tard |
| Accueil sur place pendant la fermeture | 12, 14 | appel ordinaire, présents en cas d'alerte (14) |
| Prêts de continuité | 44 | liste et retours |
| Annonce de fermeture en un geste | 17, 8 | trame « fermeture » prête, envoi N3 simplifié |

## 5. Vérification réglementaire
Relue : guide de continuité du 2/10/2026, CNIL (caméra, outils, télésurveillance), Code civil 9 et 372-1, offres de l'État. Aucun changement à la conception, sauf la mention de l'alternative réelle à l'enregistrement (P12).

## 6. Décisions
### 6.1 Appliquées par défaut (D-031)
Bloquants A à D ; salle d'essai ; aide technique ; cours hybride (R-VI42) ; présence des invités exportable ; parcours famille de salle en salle ; transfert entre salles ; pas d'indicateur collectif de caméras ; charte visio annuelle.
### 6.2 Soumis à Florian
Q-VI1 (brique), Q-VI2 (client JavaScript isolé), Q-VI3 (coût), **Q-VI4 (STUN Google dans Next.js)**, Q-VI6 (enregistrement, DPO), Q-VI10 (conventions pour les élèves extérieurs).

## 7. Synthèse
28 manques (4 bloquants, 20 importants, 4 confort) ; 30 actions, 5 règles, 20 cas limites ajoutés (§30 de la conception) ; lots VI3, VI7 et VI12 élargis.
