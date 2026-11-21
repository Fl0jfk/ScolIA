# Passe « personas » sur le socle 38 — Examens et certifications

**Date :** 2026-11-11 — **Branche :** `ent-socle-37-38-conception`. **Objet :** éprouver `docs/socles/38_EXAMENS.md` par des personnes variées (fictives, aucune donnée réelle), relever manques, contradictions et cas limites, et **reporter les ajouts dans la conception** (§28 du 38, « ajout personas »). Conventions de la conception.

---

## 0. À lire en premier : trois bloquants et leur recommandation (appliquée par défaut, D-031)

| # | Bloquant | Recommandation appliquée |
|---|---|---|
| **B38-1** | Le **surveillant remplaçant** désigné le matin ne voyait pas les mesures de ses candidats (elles n'étaient données qu'aux surveillants prévus), ou les aurait gardées ensuite | mesures transmises à la désignation, **bornées** à la salle et à la durée de la mission, journalisées, retirées à la clôture (R-EX31) |
| **B38-2** | Une salle à part **nommée** « salle tiers-temps » ou « salle ULIS » sur l'affichage révèle la situation d'un élève à toute la classe | libellés **neutres** partout (R-EX32) |
| **B38-3** | Un sujet du bac blanc **déposé ailleurs** (cahier de textes, espace de classe, banque de sujets) avant l'épreuve | un sujet d'épreuve non tenue n'existe que dans le 38 ; banque limitée aux épreuves tenues (R-EX33) |

---

## 1. Méthode
1. Lecture de la conception, du 21 (évaluation commune), du 34 (dispenses, consignes minimales), du 10 (neutralisation), du 12 (`en_examen`), du 19 (R-AG21), de M17 (Next.js).
2. **Quinze personas** (§2) dont un **attaquant** et un **auditeur RGPD** ; chacun vit une semaine d'examen ou une campagne d'inscription, action par action.
3. Manques numérotés **M38-xx**, gravité (B, I, m), correction.
4. Aucune donnée réelle ; Next.js non exécuté.

---

## 2. Les personas en un tableau

| # | Persona | Contexte | Ce qu'il éprouve |
|---|---|---|---|
| P01 | **M. Caron**, proviseur adjoint, responsable des examens | brevet blanc (180 élèves) et bac blanc la même semaine de février | session, placement, surveillances, contrôle |
| P02 | **Mme Nguyen**, secrétaire des examens | inscriptions Cyclades, 210 confirmations à signer | préparation, signatures, résultats, diplômes |
| P03 | **M. Arnaud**, professeur de philosophie | sujet du bac blanc, 70 copies | dépôt, gel, impression, correction |
| P04 | **Mme Lopez**, professeure d'EPS | CCF de terminale, 3 inaptitudes | situations, inaptitudes, PV |
| P05 | **M. Ferry**, professeur de lycée pro | chef-d'œuvre de CAP, PFMP | grilles, oral, calcul |
| P06 | **Léo**, 15 ans, 3ᵉ, ULIS, fauteuil, AESH (Mme Diallo) | tiers-temps, secrétaire, salle accessible | aménagement, convocation, jour J |
| P07 | **Sarah**, 17 ans, 1ʳᵉ, PAI allergie, dyslexie (PAP) | EAM blanche, demande d'aménagement en cours | mesures provisoires, consignes |
| P08 | **Famille Martin**, séparée | fils en terminale, deux adresses | confirmation d'inscription, résultats |
| P09 | **Hugo**, 18 ans, terminale | ne veut pas que ses parents voient ses résultats | majorité, publication |
| P10 | **M. Petit**, AED, surveillant remplaçant à 7 h 45 | salle 12, deux tiers-temps | mesures, émargement |
| P11 | **Mme Roux**, CPE | absents à l'épreuve, retards, téléphone qui sonne | absents, incidents, fraude |
| P12 | **Mme Duval**, cheffe d'établissement | fraude présumée, cérémonie de juin | suites, cérémonie, oppositions |
| P13 | **Florian**, administrateur, l'année de doublon | Charlemagne officiel, ScolIA en parallèle | mode doublon |
| P14 | **« Max »**, attaquant (élève de terminale) | veut le sujet de philosophie et la liste des tiers-temps | sujets, plans, liens, IA |
| P15 | **M. Blanc**, DPO (auditeur RGPD) | contrôle avant le doublon | registre, durées, décisions d'aménagement |

---

## 3. Parcours et manques, persona par persona

### P01 — M. Caron
Il crée « brevet blanc de février » depuis le gabarit DNB 2027 et « bac blanc » depuis le gabarit, mêmes jours. Le contrôle affiche 7 problèmes.
- **M38-01 (I)** Deux sessions concurrentes se disputent les mêmes salles et surveillants : le contrôle doit porter **sur l'ensemble des sessions** de la période. → précisé : R-EX8 vérifié **par créneau tous sessions confondues** (EXA-133).
- **M38-02 (I)** Une salle inondée le matin : réaffecter places et surveillants en un geste. → EXA-470, CEX-14.
- **M38-03 (m)** Qui a déplacé tel élève de salle ? → EXA-474.
- **M38-04 (m)** Le restaurant et l'internat ne savent pas que les 3ᵉ sortent à 12 h 40. → EXA-495.

### P02 — Mme Nguyen
- **M38-05 (I)** 12 familles sans compte : il faut imprimer convocations et confirmations. → EXA-473.
- **M38-06 (I)** Un élève arrive d'un autre établissement avec une **décision d'aménagement** déjà rendue : la saisir sans redemander. → EXA-489 (document de la famille, mesures seulement, R-EX11).
- **M38-07 (m)** Étiquettes de copies et bordereaux. → EXA-491, EXA-492.

### P03 — M. Arnaud
- **M38-08 (B)** Il a déposé par erreur le sujet dans le cahier de textes de la classe en « brouillon » : un élève l'aurait vu si publié. → **B38-3**, EXA-463, R-EX33, CEX-58.
- **M38-09 (I)** Un collègue ouvre son sujet vingt fois la veille au soir : alerte. → EXA-486.

### P04 — Mme Lopez
- **M38-10 (I)** Inaptitude partielle : elle ne doit voir que « inapte partiel du … au … », jamais le motif ; la conséquence vient du 34. → EXA-301 confirmé ; CEX-50.
- **M38-11 (m)** Règles d'épreuve adaptée : [NV] national → saisies par l'établissement (inchangé).

### P05 — M. Ferry
- **M38-12 (m)** Il veut voir le calcul 50/50 du chef-d'œuvre avant validation : EXA-305 déjà « affiché ». Pas de manque.
- **M38-13 (m)** Un élève conteste la note d'oral. → EXA-480, EXA-481, modèle 2777.

### P06 — Léo et Mme Diallo
- **M38-14 (I)** Salle accessible et table adaptée : contrainte dure du placement. → EXA-466.
- **M38-15 (I)** L'AESH est employée par un tiers (public) ou par l'établissement : prévenir des horaires décalés. → EXA-467.
- **M38-16 (B)** La salle à part s'appelait « Salle tiers-temps » sur la liste affichée. → **B38-2**, EXA-462, R-EX32.

### P07 — Sarah
- **M38-17 (I)** Demande en cours : mesures **provisoires** appliquées à l'EAM blanche (R-EX13) ; elle doit savoir que c'est provisoire. → la convocation porte « provisoire » (EXA-220).
- **M38-18 (I)** PAI allergie : pause et collation autorisées, consigne minimale au responsable de salle. → EXA-464, EXA-465, CEX-56.

### P08 — Famille Martin
- **M38-19 (I)** Le père ne reçoit pas l'information de l'épreuve du samedi. → EXA-469.
- **M38-20 (m)** La mère a signé la confirmation ; le père veut changer l'option : CEX-38, Q-EX10 (direction arbitre selon le 4).

### P09 — Hugo, 18 ans
- **M38-21 (I)** Les convocations partent en copie à ses parents par défaut (S38-05) : il peut retirer ; les **résultats** ne partent qu'à lui (CEX-39). Cohérent. Rappel annuel du partage (repris du 37, ORI-364) **à généraliser** au 4 (socle 4).

### P10 — M. Petit, AED remplaçant
- **M38-22 (B)** **B38-1** : il ne voyait pas les deux tiers-temps. → EXA-460, EXA-461, R-EX31, CEX-55, modèle 2776.
- **M38-23 (I)** Pas de réseau dans la salle : émargement hors ligne (EX6). Confirmé.

### P11 — Mme Roux, CPE
- **M38-24 (I)** Signal des élèves **souvent absents** aux épreuves (déterministe) pour anticiper. → EXA-477 (sans score).
- **M38-25 (m)** Un candidat part avec sa copie. → EXA-479.
- **M38-26 (I)** Téléphone qui sonne à l'interne : la règle de l'établissement (sanction ou non) relève du 13 ; jamais automatique (R-EX24). Confirmé.

### P12 — Mme Duval
- **M38-27 (I)** Cérémonie : une famille s'oppose ; une autre demande une place de plus. → EXA-450 déjà ; EXA-441 (max par élève) avec dérogation de la direction (EXA-443).
- **M38-28 (I)** Elle découvre ND-01 en relisant la recette. → **F38-1**.

### P13 — Florian, année de doublon
- **M38-29 (B→I)** Les familles reçoivent deux convocations (Charlemagne ou papier, et ScolIA). → mode doublon EXA-488, R-EX34, modèle 2777.

### P14 — « Max », attaquant
Il essaie : (a) deviner l'URL du sujet ; (b) l'IA (« résume le sujet de philosophie du bac blanc ») ; (c) le plan de salle partagé pour repérer les tiers-temps ; (d) le lien de réponse à la cérémonie d'un camarade ; (e) se faire désigner « co-auteur ».
- **M38-30 (I)** (a) objets non devinables et contrôle des droits par le 7 ; confirmé ; (b) R-EX14 : l'IA n'a aucun accès avant la fin ; cas ajouté au jeu d'évaluation ; (c) R-EX32 ; (d) R-EX30, EXA-487, modèle 2776 ; (e) co-auteurs = **personnels** seulement (contrainte ajoutée à EXA-172).
- **M38-31 (m)** Ouvertures répétées d'un sujet par un compte compromis : EXA-486.

### P15 — M. Blanc, DPO
- **M38-32 (I)** Registre des traitements du socle. → EXA-482.
- **M38-33 (I)** Droit d'accès. → EXA-483.
- **M38-34 (I)** Purge (copies, candidats extérieurs, cérémonie). → EXA-484.
- **M38-35 (I)** Qui lit les décisions d'aménagement ? → EXA-485.
- **M38-36 (I)** La liste « à risque d'absence » est-elle un profilage ? → déterministe, expliquée, accessible à la vie scolaire seulement, sans score (EXA-477) ; validation DPO (F38-5).
- **M38-37 (m)** Usage religieux (jeûne, fête) déclaré par la famille : donnée sensible (art. 9 RGPD) → **information non conservée** au-delà de la session, facultative (EXA-468, CEX-57) ; DPO (F38-5).

### Manques transverses
- **M38-38 (m)** « Aujourd'hui » (29) : épreuves de la semaine. → EXA-494.
- **M38-39 (m)** Traduction des convocations. → EXA-472.
- **M38-40 (m)** « Ce que je dois apporter ». → EXA-471.
- **M38-41 (m)** Réouvrir une salle close par erreur. → EXA-478.
- **M38-42 (m)** Élève allophone et dictionnaire bilingue [NV national]. → EXA-490.

---

## 4. Vérification réglementaire (2026-11-11)
- Usage d'un dictionnaire bilingue pour les allophones au DNB : **[NV]** → aucune règle codée, autorisation par épreuve.
- AESH comme secrétaire aux examens nationaux : **[NV]** → décision de l'autorité (Q-EX11).
- Données religieuses : RGPD art. 9 (catégorie particulière) [V, texte connu] → minimisation (M38-37).

---

## 5. Manques qui relèvent d'autres socles
| Manque | Socle | Proposition |
|---|---|---|
| Rappel annuel du partage pour l'élève majeur (généraliser) | 4 | règle commune |
| Consigne minimale « collation, pause » pour un PAI | 34 | conséquence partagée à l'examen |
| Prévenir l'employeur d'une AESH | 35, 6 | port d'information |
| Horaires décalés pour cantine et internat | 40, 41 | consommer `examens.calendrier_publie` |
| Refus de sujets d'épreuve non tenue dans le cahier de textes | 20 | consommer le port du 38 |
| Décision d'aménagement d'examen au 38, plan au 35 | 35 | C1 du 38, à reporter à sa conception |

---

## 6. Synthèse

### 6.1 Manques par gravité
- **Bloquants : 4** (M38-08, M38-16, M38-22, M38-29 ramené à important après le mode doublon) → B38-1 à B38-3 et mode doublon.
- **Importants : 25.**
- **Mineurs : 13.**
Total : **42 manques**.

### 6.2 Ajouts dans la conception
§28 du 38 : **36 actions** (EXA-460 à EXA-495), **4 règles** (R-EX31 à R-EX34), **4 cas limites** (CEX-55 à CEX-58), **2 questions** (Q-EX15, Q-EX16), **2 migrations** dans la réserve (2776, 2777).

### 6.3 Les dix ajouts les plus importants
1. Mesures bornées au surveillant de la salle, y compris le remplaçant (R-EX31).
2. Libellés neutres partout (R-EX32).
3. Sujet d'épreuve non tenue introuvable hors du 38 (R-EX33).
4. Mode doublon (R-EX34).
5. Contrôle du plan sur toutes les sessions d'un créneau (M38-01).
6. Décision d'aménagement reprise d'un autre établissement sans pièce médicale (EXA-489).
7. Consignes minimales du PAI pendant l'épreuve (EXA-464, 465).
8. Alerte d'accès anormal à un sujet (EXA-486).
9. Contestation d'une note d'oral tracée (EXA-480, 481).
10. Registre, droit d'accès, purge, journal des décisions (EXA-482 à 485).

### 6.4 Ce que la passe a montré
Le moteur tenait ; les fuites possibles étaient **dans les à-côtés** (libellés, remplaçants, autres espaces de dépôt) et **dans le doublon**.

---

## 7. Décisions

### 7.1 Appliquées par défaut (D-031)
B38-1, B38-2, B38-3 ; mode doublon ; Q-EX15 (pas d'export par le surveillant) ; Q-EX16 (correcteurs masqués avant levée) ; co-auteurs de sujet limités aux personnels ; information religieuse facultative et non conservée.

### 7.2 Soumis à Florian
F38-1 (ND-01, urgent) ; F38-5 (DPO : liste « à risque d'absence », information religieuse) ; F38-2 (centre d'examen).

---

## 8. Faits, hypothèses, contradictions
**Faits** : ND-01 relu (lecture seule). **Hypothèses** : volumes, pratiques. **Contradictions** : aucune nouvelle ; C1 à C5 du 38 inchangées.
