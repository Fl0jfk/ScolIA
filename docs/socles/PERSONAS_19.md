# Passe « personas » sur le socle 19 — Agenda, réunions et rendez-vous

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** (à relire par Florian) ; les recommandations s'appliquent par défaut (D-031), sauf ce qui engage l'établissement à l'extérieur (§8.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production. Le code Next.js a été **lu seulement** (copie de `origin/main` par `git archive`, rien exécuté).
**Branche :** `ent-socle-19-conception`, partie de `origin/ent-mc0hoe` (`7467f2d`).
**Éprouve :** `docs/socles/19_AGENDA_RDV.md` (§0 à §32), avec `CLAUDE.md`, `docs/DECISIONS.md` (D-031, D-034, D-035, D-036), `docs/TRANSITION_T0_INVENTAIRE.md` (M02, M03, M16, M17) ; forme reprise de `PERSONAS_10.md` et `PERSONAS_42.md`.
**Ajouts :** faits **directement** dans `19_AGENDA_RDV.md`, au **§33** (« ajout personas »), qui l'emporte en cas de contradiction ; rien n'est supprimé.
**Numérotation :** manques **M19-001 à M19-099** (préfixe propre au socle, pour éviter les collisions avec les sessions parallèles du 2026-11-11 : le plus haut numéro global pris sur les branches est M-2591) ; actions **AGD-320 à AGD-339** ; règles **R-AG35 à R-AG45** (R-AG46 à R-AG48 viennent de la vérification réglementaire) ; cas limites **CAG-61 à CAG-80** ; questions **Q-AG17 et suivantes** ; réglages **S19-14 et suivants**.

---

## 0. À lire en premier : quatre bloquants et leur recommandation (appliquée par défaut, D-031)

| # | Point | Pourquoi c'est bloquant | Recommandation (appliquée par défaut) | Lots |
|---|---|---|---|---|
| **A** | **Une interdiction de contact ne protège pas le couloir.** R-AG12 empêche l'information croisée et l'enchaînement, mais deux parents sous interdiction peuvent avoir des rendez-vous à 18 h 10 et 18 h 20 avec deux professeurs **du même couloir** le même soir (P04). | Risque physique pour une victime de violences ; l'établissement a l'information (socle 4) et l'agenda la contourne. | **Séparation dans le temps et l'espace** (R-AG35) : sous interdiction ou adresse protégée, les rendez-vous des deux parents d'une même campagne sont tenus à **au moins 45 minutes** d'écart sur toute la soirée et, si possible, dans des bâtiments différents ; à défaut, un **rendez-vous individuel hors soirée** est proposé à l'un d'eux par le PP ; l'accueil voit « arrivées à séparer » sans motif. Le calcul est serveur, invisible des parents. | AG5, AG9 |
| **B** | **Le « cours manqué » révèle un rendez-vous sensible.** AGD-179 prévient le professeur que l'élève manquera son cours pour un rendez-vous ; AGD-230 fait de même pour une convocation de santé. Un élève qui sort chaque mardi à 10 h « pour rendez-vous » pendant le cours de Mme X, c'est un suivi PsyEN deviné (P07). | Contredit R-AG5 et R-AG7 par recoupement ; risque de stigmatisation. | **Sortie de cours neutre** (R-AG36) : pour un rendez-vous restreint, le professeur et l'appel (socle 12) ne voient que « absence autorisée par la vie scolaire », jamais « rendez-vous », jamais l'hôte ; l'élève peut demander un créneau **hors cours** en priorité ; un rendez-vous restreint récurrent est placé, si possible, sur des cours **différents** (rotation proposée à l'hôte). | AG4 (+12) |
| **C** | **L'occupation des personnels de santé et du social révèle les élèves.** Le libre / occupé (R-AG6) est ouvert à tout le personnel enseignant ; croisé avec l'EDT nominatif des élèves (que le professeur voit pour ses élèves), l'« occupé » de la PsyEN ou de l'infirmière à 10 h et l'absence de Léa à 10 h désignent Léa (P16, attaquant « professeur curieux »). | Inférence sur une donnée de santé ou de suivi psychologique. | **Occupation protégée** (R-AG37) : l'occupation de la PsyEN, des infirmiers, des assistants sociaux et du médecin n'est visible que d'eux-mêmes et de la direction ; les autres voient seulement leurs **plages ouvertes** à la réservation ; aucune réunion ne peut être placée par un tiers sur leurs plages sans passer par eux. | AG2 |
| **D** | **Les familles sans numérique sont exclues.** Toute la conception suppose que la famille réserve en ligne. Or le secrétariat et le PP prennent encore beaucoup de rendez-vous par téléphone ou au portail (P05, P08) ; AGD-159 crée un rendez-vous « convenu » hors campagne, mais rien ne permet de **réserver un créneau de campagne pour une famille** ni d'envoyer un **courrier** de convocation. | Inégalité d'accès ; l'établissement retomberait sur le tableur. | **Réservation pour le compte d'une famille** (AGD-320, traçée « pris par le secrétariat au téléphone ») dans toutes les offres, avec les mêmes contrôles ; **courrier** de convocation ou d'invitation (socle 7) pour les personnes sans compte ou qui le demandent (AGD-321) ; ligne « rendez-vous par téléphone » à l'accueil. | AG4, AG5 |

**Défaut Next.js nouveau** (à signaler à Florian, Next.js reste intouchable) : aucun, au-delà de ND-01 à ND-32 du §2.3 de la conception ; la passe confirme la gravité de ND-20 et ND-30 (pages publiques qui renvoient l'e-mail d'un parent).

---

## 1. Méthode

**Démarche** (celle de `PERSONAS_10.md` §1 et `PERSONAS_42.md` §1) : dix-huit personas réalistes, prises dans le tableau des acteurs (§4 de la conception), couvrant le privé sous contrat (lycée-collège pilote de Florian) et le public (EPLE), les familles ordinaires et les cas particuliers, un **attaquant** (cinq variantes), une **déléguée à la protection des données** et un **persona de bascule** (Florian, administrateur). Pour chacune : son année ou sa journée, ses irritants, peurs et ce qui la ravirait, les **manques** et **contradictions** de la conception, les **cas limites**.

**Grille transversale** : (1) **parité** — que fait-elle aujourd'hui dans Next.js (salles, rendez-vous d'inscription, portes ouvertes) et le retrouve-t-elle ? (2) **minimisation** — qu'apprend-elle qu'elle ne devrait pas, y compris par recoupement ? (3) **D-036** — ce qu'elle saisit est-il une colonne ou une table ? (4) le **mobile** ; (5) ce que l'**IA** prépare sans décider.

**Gravité** : **bloquant** · **important** · **confort** · **différenciant**.

**Conventions** : [V] lu sur source officielle ; [NV] non vérifié ; [H] hypothèse ; [R] recommandation ; [F] fait vérifié dans le code.

---

## 2. Les personas en un tableau

| # | Persona | Contexte | Ce qu'elle éprouve |
|---|---|---|---|
| P01 | Mme Martin, adjointe | lycée-collège privé, 1 100 élèves ; monte les soirées de 4ᵉ-3ᵉ et de 2ᵈᵉ | campagnes, plages, salles, vagues |
| P02 | M. Leroy, professeur de mathématiques | 7 classes, 180 élèves, service partagé avec un collège **hors ScolIA**, temps partiel le mercredi | hôte calculé, capacité, disponibilités |
| P03 | Mme Benali, CPE | assiduité, convocations, commission éducative | rendez-vous obligatoires, non-venues |
| P04 | Mme Garnier et M. Garnier | séparés ; interdiction de contact, adresse protégée de la mère ; deux enfants (5ᵉ, 2ᵈᵉ) | R-AG12, couloir |
| P05 | Famille Nguyen | trois enfants (6ᵉ, 4ᵉ, 2ᵈᵉ) ; mère allophone, père en horaires décalés ; pas d'ordinateur | parcours, langue, téléphone |
| P06 | Hugo, 18 ans | terminale, majeur, orientation | autonomie, information des parents |
| P07 | Inès, 13 ans | suivie par la PsyEN chaque semaine | rendez-vous restreint, cours manqué |
| P08 | Mme Petit, secrétaire de direction | agenda du chef, rendez-vous d'inscription, courriers | délégation, téléphone, courrier |
| P09 | M. Durand, principal de collège **public** | CA, commission permanente, conseil pédagogique, conseil de discipline | délais, quorum, instances |
| P10 | Mme Roux, AESH | deux élèves accompagnés ; ESS | convocations, temps de service |
| P11 | M. Faure, professeur d'EPS | réserve chaque semaine le gymnase, la salle informatique ; utilisateur assidu de M02 | parité salles, séries |
| P12 | Mme Lopez, accueil | portail, standard | liste du jour, visiteurs, interdictions |
| P13 | M. Bernard, agent | mise en place des salles | planning des salles |
| P14 | Mme Caron, infirmière | bilan de 6ᵉ, campagne de vaccination | convocations de santé |
| P15 | Mme Morel, responsable EVARS | programme d'interventions (M03) | refonte de M03 |
| P16 | Les attaquants | parent harceleur, élève curieux, professeur curieux, script d'énumération, intrus d'un autre établissement | sécurité |
| P17 | Mme Blanc, déléguée à la protection des données | registre, durées, minimisation | RGPD |
| P18 | Florian, administrateur, veille de bascule des salles | février 2027 | reprise |

---

## 3. Audit de parité avec Next.js (fait de code, inventaires de la conception §2.2)

Les 19 actions de M02, 12 de M03, 15 de M16, 8 de M17 et 8 des invitations et partenariats ont chacune une cible (colonne « Repris en » du §2.2). **Trois reculs** relevés par la passe et corrigés au §33 :
1. **Couleurs par matière** et **saisie niveau → classe** de M02 (NX-17, NX-06) n'ont pas d'équivalent dans le modèle (`ent_agenda_reservation` n'a ni matière ni classe) → colonnes `matiere_code` et `classe_id` (facultatives) ajoutées (M19-061).
2. **Personne présente au rendez-vous** (« madame, monsieur, les deux », M16) : couvert par les participants, mais pas pour un rendez-vous pris par un externe sans compte → champ `presents_annonces` (texte court) (M19-062).
3. **Équipe d'accueil des portes ouvertes** (NX-65) : AGD-311 la reprend, mais le modèle n'a pas de table → `ent_agenda_session_equipe` (M19-063).

---

## 4. Parcours et manques, persona par persona

### P01 — Mme Martin, adjointe : la soirée de 4ᵉ-3ᵉ du 3 décembre

**Son année.** Septembre : réunions de rentrée par niveau, salles du soir. Novembre : elle monte deux soirées (4ᵉ-3ᵉ le 3 décembre, 2ᵈᵉ le 10). Mars : seconde série. Elle faisait un tableur : 38 professeurs, 14 salles, 260 familles.
**Irritants** : les professeurs à deux établissements ; les salles du bâtiment B fermées le soir (alarme) ; les familles qui prennent tout la première heure.
**Peurs** : un professeur qui n'a pas été prévenu ; 40 familles dans un couloir à 18 h.
**Ce qui la ravirait** : « Générer », regarder l'aperçu, corriger trois choses, ouvrir.

| N° | Manque | Gravité | Proposition |
|---|---|---|---|
| M19-001 | Les **horaires d'ouverture des bâtiments** le soir (alarme, gardiennage) ne sont pas une contrainte : la génération peut placer un professeur en salle B à 19 h | important | contrainte « bâtiment ouvert » lue au socle 5 (horaires de site) ; à défaut, plage de la campagne par bâtiment (AGD-322) |
| M19-002 | Pas de **lissage de la demande** : les familles rapides prennent tous les créneaux de 17 h 30 | important | ouverture par **vagues horaires** et plafond de rendez-vous par famille **dans la première vague** (AGD-323) |
| M19-003 | Pas de **simulation de charge** avant ouverture : « avec 10 min, Mme X aura-t-elle assez de créneaux ? » | important | l'aperçu (AGD-201) affiche, par hôte, élèves concernés / créneaux / demande estimée (taux de l'an passé, à défaut 40 %) |
| M19-004 | Rien pour la **salle unique** (gymnase avec 30 tables numérotées) | confort | lieu « table n » : `lieu_texte` par hôte et plan imprimable (AGD-216) |
| M19-005 | Les professeurs voient leur planning trop tard | important | publication aux hôtes **avant** l'ouverture aux familles, avec 48 h pour ajuster (AGD-324) |
| M19-006 | L'IA propose une soirée mais la conception ne dit pas ce qu'elle optimise | différenciant | critères explicites : couverture des familles « à voir », équité entre familles, temps d'attente, pauses des hôtes (§18.1 enrichi) |
| M19-007 | Fermer une soirée **par erreur** n'a pas de retour arrière | important | « rouvrir » une campagne close depuis moins de 24 h (AGD-325) |

**Cas limites** : CAG-61 (un professeur arrive dans l'établissement la veille de la soirée : hôte ajouté, familles de ses classes prévenues d'un créneau nouveau) ; CAG-62 (deux soirées le même soir dans deux bâtiments, même professeur : conflit signalé).

### P02 — M. Leroy, professeur de mathématiques, 180 élèves, deux établissements

**Sa journée de soirée** : 17 h–20 h, 18 rendez-vous de 10 min, sans pause ; trois familles en retard décalent tout.
**Irritants** : l'autre collège (hors ScolIA) le veut le même soir ; les familles qui demandent un rendez-vous « en plus » par message.
**Ce qui le ravirait** : sa liste du soir avec, pour chaque élève, ses trois dernières notes et la dernière appréciation, sur sa tablette.

| N° | Manque | Gravité | Proposition |
|---|---|---|---|
| M19-008 | Aucune **pause imposée** pour un hôte très demandé, ni plafond de rendez-vous par hôte et par soirée | important | pause de 10 min toutes les 90 min **non réservable** et plafond par hôte (S19-14, défaut 18) |
| M19-009 | Un engagement dans un établissement **hors ScolIA** n'existe que comme « plage occupée » déclarée à la main | confort | déjà AGD-186 ; ajout d'un **import ponctuel .ics** de ses contraintes (AGD-187 avancé à AG2) |
| M19-010 | Le retard d'une famille décale les suivantes sans le leur dire | important | « je suis en retard » (AGD-283) et retard de l'hôte (AGD-211) décalent **l'affichage** des suivantes, jamais leur réservation |
| M19-011 | La fiche du soir n'est pas définie précisément | différenciant | fiche : notes de **ses** matières (21), dernière appréciation, assiduité dans **ses** cours (12), PAP « oui » sans détail (35) — R-AG9 |
| M19-012 | Un professeur à **temps partiel** absent le jour de la soirée n'a pas d'alternative offerte aux familles | important | « recevoir à une autre date » : l'hôte ouvre ses disponibilités dans la campagne (créneaux hors soirée) (AGD-326) |

### P03 — Mme Benali, CPE : convocation après douze demi-journées d'absence

**Parcours** : elle convoque la famille ; la famille ne vient pas ; elle reconvoque ; elle doit pouvoir prouver la convocation (socle 12, assiduité).

| N° | Manque | Gravité | Proposition |
|---|---|---|---|
| M19-013 | La **preuve de convocation** (date d'envoi, canal, accusé, non-venue) n'est pas exportable vers le dossier d'assiduité | important | le fait `convoque` et `non_venu` est lu par le socle 12 (port `agenda::rdv_faits(eleve, nature)`), et un courrier peut être déposé au 7 (AGD-321) |
| M19-014 | « Non venu » ne propose rien | important | après « non venu » d'une convocation : proposition automatique d'un second rendez-vous et d'un courrier (AGD-327) |
| M19-015 | Commission éducative : la conception la donne au 13, mais pas qui convoque ni dans quel délai | important | c'est une réunion demandée par le 13 (AGD-141) ; délai et composition fournis par le 13 (§6 de la conception) |
| M19-016 | Le CPE reçoit souvent **sans rendez-vous** (famille au portail) | confort | « rendez-vous immédiat » créé et honoré en un geste, pour la trace (AGD-328) |

### P04 — Mme Garnier et M. Garnier : séparés, interdiction de contact, adresse protégée

**Situation** : ordonnance de protection ; la mère a une adresse protégée (socle 4) ; chacun a l'autorité parentale ; deux enfants.
**Peur** (mère) : croiser le père le soir de la rencontre. **Peur** (père) : être écarté de la scolarité.

| N° | Manque | Gravité | Proposition |
|---|---|---|---|
| M19-017 | **Couloir** (bloquant A) | **bloquant** | R-AG35 |
| M19-018 | Les **réunions collectives** (réunion de rentrée de la classe, remise des diplômes) réunissent les deux parents sans aucune alerte | important | l'organisateur et l'accueil voient « deux participants à ne pas placer ensemble » sans nom ni motif quand les deux ont répondu oui ; placement et accueil séparés (R-AG38) |
| M19-019 | L'**information de l'existence** d'un rendez-vous à l'autre parent (foyers distincts) peut, à elle seule, servir à guetter la sortie de l'établissement | important | sous interdiction : aucune information, même sans heure (déjà R-AG12) ; **en outre**, sans interdiction mais avec signalement d'un parent (« je ne souhaite pas que l'autre parent soit informé ») : examen par la direction, jamais automatique (AGD-329) |
| M19-020 | L'enfant en garde alternée a deux **calendriers** de garde : la convocation d'élève tombe la semaine du père | confort | aucune règle propre (hors périmètre) ; avis aux deux parents selon R-AG12 |
| M19-021 | Une **interdiction levée** ne rétablit rien | confort | à la levée, les rendez-vous futurs repassent sous la règle normale, sans rétroactivité d'information |

**Cas limites** : CAG-63 (le père réserve, la mère réserve ensuite le même professeur : le second rendez-vous est proposé à au moins 45 min ou un autre jour) ; CAG-64 (le professeur lui-même ne doit pas dire « votre ex-femme sort d'ici » : la fiche lui affiche « recevoir séparément », sans nom de l'autre parent).

### P05 — Famille Nguyen : trois enfants, mère allophone, sans ordinateur

| N° | Manque | Gravité | Proposition |
|---|---|---|---|
| M19-022 | **Réservation pour le compte de la famille** (bloquant D) | **bloquant** | AGD-320 |
| M19-023 | Le parcours en **tout ou rien** (R-AG32) échoue souvent pour une fratrie à 15 professeurs | important | « meilleur parcours possible » : ce qui tient est réservé **après confirmation**, le reste en liste d'attente (R-AG39 remplace R-AG32) |
| M19-024 | Interprète : la conception dit « demander un interprète » mais pas qui le fournit ni comment l'hôte le sait | important | marque sur le rendez-vous, langue, interprète désigné (personnel, bénévole, service externe) ou « enfant aîné majeur accepté pour une information, jamais pour une décision » (INS-61) ; durée doublée (AGD-330) |
| M19-025 | Les avis par SMS sont la seule voie utile, mais R-AG27 les réserve à l'annulation du jour | important | SMS de **confirmation** pour les personnes qui n'ont que ce canal (préférence du socle 8), sous Q-N22 |
| M19-026 | Le père en horaires décalés ne peut venir qu'à 20 h 30 | confort | modalité téléphone ou visio proposée en tête quand la soirée est saturée |

### P06 — Hugo, 18 ans, terminale

| N° | Manque | Gravité | Proposition |
|---|---|---|---|
| M19-027 | À la majorité, les **invitations des parents** à la soirée continuent par défaut | important | conforme au socle 4 : invitations aux parents seulement si Hugo l'a accepté ; la campagne affiche à l'organisateur « 12 élèves majeurs : invitation à eux » |
| M19-028 | Hugo veut que sa mère **vienne avec lui** sans qu'elle reçoive tout | confort | AGD-165 (ajouter un participant) par Hugo lui-même pour **un** rendez-vous |

### P07 — Inès, 13 ans, suivie par la PsyEN

| N° | Manque | Gravité | Proposition |
|---|---|---|---|
| M19-029 | **Cours manqué** (bloquant B) | **bloquant** | R-AG36 |
| M19-030 | Le **rappel push** sur le téléphone d'Inès affiche « Rendez-vous avec Mme X (psychologue) » sur l'écran verrouillé | important | texte neutre « Rendez-vous à 10 h » pour les natures restreintes (R-AG40) |
| M19-031 | Si Inès est mineure et que ses parents ont un compte : l'agenda **de l'enfant** côté parent montrerait-il le rendez-vous ? | important | non pour une nature restreinte prise par l'élève (§15 de la conception) ; règle à vérifier au socle 36 (Q-AG17) |

### P08 — Mme Petit, secrétaire de direction

| N° | Manque | Gravité | Proposition |
|---|---|---|---|
| M19-032 | Délégation de l'agenda du chef : AGD-181 existe, mais pas « qui voit le motif » | important | le délégué voit les motifs des rendez-vous **ordinaires** du chef ; jamais les natures restreintes ; journal |
| M19-033 | Les **courriers** (convocation d'une famille sans compte, convocation du CA aux externes) | important | AGD-321 |
| M19-034 | Rendez-vous d'inscription (M16) : aujourd'hui elle travaille dans Google Agenda ; demain dans ScolIA, mais **le chef consulte son agenda sur son téléphone** | important | abonnement iCal (AG10) **avancé** pour les hôtes du 19 dès AG4, contenu minimisé |

### P09 — M. Durand, principal d'un collège public

| N° | Manque | Gravité | Proposition |
|---|---|---|---|
| M19-035 | Le **calcul du délai** de convocation doit compter en jours **francs** ou calendaires selon le texte | important | règle portée par type d'instance (§6 de la conception) ; test par type |
| M19-036 | Convocation du CA : les **documents** doivent être joints selon l'ordre du jour | important | déjà AGD-127 ; alerte « point sans pièce » |
| M19-037 | Le privé sous contrat n'a pas de CA mais un conseil d'établissement et les instances de l'OGEC | important | types d'instances par **régime** (public / privé) (§6) |

### P10 — Mme Roux, AESH

| N° | Manque | Gravité | Proposition |
|---|---|---|---|
| M19-038 | L'ESS est convoquée par l'**enseignant référent** (extérieur) : la conception suppose que l'établissement convoque | important | une ESS peut être **reçue** (organisateur externe, lien nominatif) et l'établissement y **inscrit** ses participants internes (AGD-331) |
| M19-039 | L'AESH n'apparaît dans aucun calcul de participants | important | participant calculé « AESH de l'élève » (port 35/6) |

### P11 — M. Faure, professeur d'EPS, utilisateur assidu de la réservation de salles

| N° | Manque | Gravité | Proposition |
|---|---|---|---|
| M19-040 | Couleurs et matière (recul de parité) | important | §3, M19-061 |
| M19-041 | Ses séries de septembre à juin **traversent le changement d'EDT** de janvier | important | une modification d'EDT qui met une occurrence en conflit la signale (AGD-259 étendu aux réservations) |
| M19-042 | « Copier la semaine » : il réserve semaine par semaine, pas en série | confort | AGD-332 dupliquer mes réservations d'une semaine |
| M19-043 | Il ne reçoit aucun avis à la **modification** (comme Next.js) | confort | avis de modification si l'auteur n'est pas le bénéficiaire |

### P12 — Mme Lopez, accueil

| N° | Manque | Gravité | Proposition |
|---|---|---|---|
| M19-044 | La **liste du jour imprimée** reste sur le comptoir | important | impression avec mention « document interne », heures et noms seulement ; écran par défaut ; purge à J+1 (R-AG41) |
| M19-045 | Visiteur qui donne un **autre nom** | confort | identification par la personne attendue (rendez-vous), alerte si une interdiction concerne un nom proche (socle 14) |
| M19-046 | Le standard prend un rendez-vous : elle n'a pas le droit `agenda.rdv.proposer` | important | AGD-320 ouvert à l'accueil **pour les offres publiques** (inscriptions, portes ouvertes) |

### P13 — M. Bernard, agent

| N° | Manque | Gravité | Proposition |
|---|---|---|---|
| M19-047 | La **mise en place** est du texte libre | confort | liste courte typée (tables en U, rangs, vidéoprojecteur, sono, chaises supplémentaires) + texte (table `ent_agenda_reservation_besoin`) |
| M19-048 | Il veut son planning **la veille à 16 h** | confort | avis quotidien (socle 8) des mises en place du lendemain |

### P14 — Mme Caron, infirmière : bilan de 6ᵉ

| N° | Manque | Gravité | Proposition |
|---|---|---|---|
| M19-049 | 200 convocations dans les heures de cours, une salle, deux semaines | important | AGD-228 (répartition automatique) avec contrainte « pas pendant une évaluation annoncée » (socle 20) et sortie neutre (R-AG36) |
| M19-050 | Les familles doivent être **informées** sans que l'agenda porte la nature médicale | important | avis neutre « rendez-vous à l'infirmerie » (R-AG30) ; contenu au socle 34 |

### P15 — Mme Morel, responsable EVARS

| N° | Manque | Gravité | Proposition |
|---|---|---|---|
| M19-051 | Les **intervenants extérieurs** (association) n'ont pas de compte | important | intervenant externe = personne « contact » du socle 4 avec convention (7) ; positionné par la responsable |
| M19-052 | La séance réalisée doit pouvoir **remplacer un cours** (séance du 10) avec appel | important | AGD-306 demande au 10 une séance « intervention » ; appel par l'enseignant présent (12) |
| M19-053 | Le bilan annuel par classe est la preuve attendue par la direction | important | AGD-309 (déjà) ; ajout du nombre de séances réalisées par élève absent ou présent, **sans** liste nominative hors direction |

### P16 — Les attaquants

| Variante | Tentative | Défense | Manque |
|---|---|---|---|
| Parent harceleur (ex-conjoint) | réserve juste avant l'autre parent pour l'attendre ; s'abonne à l'iCal de l'enfant ; appelle l'accueil « pour savoir si mon ex est là » | R-AG35 ; iCal de l'enfant sans rendez-vous des parents ; accueil sans information sur un autre visiteur (R-AG42) | M19-054 **important** : script d'accueil « nous ne donnons pas d'information sur les visiteurs » |
| Élève curieux | change l'identifiant d'élève dans l'URL ; crée un « rendez-vous » à un camarade | 404 indiscernable ; seuls les personnels invitent (CAG-51) | — |
| Professeur curieux | libre / occupé de la PsyEN croisé avec l'EDT | **bloquant C**, R-AG37 | M19-055 **bloquant** |
| Script d'énumération | parcourt `/rdv/{jeton}` et l'inscription publique | jetons 128 bits, limitation de débit du socle 1, réponse indiscernable | M19-056 **important** : **injection** dans l'ICS (CRLF dans un titre saisi) et dans les **exports tableur** (formule commençant par `=`) → R-AG43 |
| Intrus d'un autre établissement | rejoue une requête avec l'identifiant d'un autre établissement | RLS stricte, clés composites | — |
| Hameçonnage | publie un événement avec un lien piégé dans la description | M19-057 **important** : description rendue en texte, liens seulement vers des domaines autorisés (R-AG44) |

### P17 — Mme Blanc, déléguée à la protection des données

| N° | Manque | Gravité | Proposition |
|---|---|---|---|
| M19-058 | « Familles jamais venues sur l'année » (AGD-215, §11.4) est un **profilage** des familles | important | indicateur **par élève**, visible du seul PP et du CPE pour leurs élèves, sans conservation au-delà de l'année, jamais exporté ni agrégé par famille (R-AG45) |
| M19-059 | La **mention d'information** (art. 13 RGPD) n'apparaît nulle part dans les parcours externes | important | mention courte sur toute page de rendez-vous public et dans les avis (finalité, durée, contact du DPO), fournie par le socle 27 |
| M19-060 | Import d'agenda personnel : données de **tiers** (rendez-vous médicaux du personnel) | confort | seules les plages sont lues, titre et participants jamais stockés (déjà Q-AG7), traitement local dans la transaction d'import |
| — | Registre des traitements : quatre lignes (agenda et rendez-vous ; réservations de lieux ; campagnes ; programmes d'interventions) | — | fiches à fournir au socle 27 |

### P18 — Florian, administrateur, veille de la bascule des salles (février 2027)

| N° | Manque | Gravité | Proposition |
|---|---|---|---|
| M19-064 | Les réservations **créées dans Next.js après l'export** sont perdues | important | gel de M02 la veille (action de Florian) **et** second export différentiel le matin ; comparaison des comptes |
| M19-065 | Les **doublons historiques** (ND-01) bloquent l'import par la contrainte d'exclusion | important | file humaine des chevauchements à l'import, avant la bascule, avec proposition (garder le plus ancien) |
| M19-066 | Les **séries** Next.js sont des lignes indépendantes reliées par `group_id` : une série partiellement annulée | confort | série reconstituée si au moins deux occurrences futures ; sinon réservations unitaires |
| M19-067 | Les professeurs ne savent pas que l'outil change | important | annonce (socle 17) à J-7, lien depuis l'ancien `/prof-room` (action de Florian) |

### Manques transverses : mobile, IA, tests

| N° | Manque | Gravité | Proposition |
|---|---|---|---|
| M19-068 | L'émargement sur mobile **hors réseau** (gymnase, sous-sol) | important | émargement local puis synchronisation, horodatage serveur à la réception, conflits signalés |
| M19-069 | L'IA qui propose un créneau commun ne doit pas apprendre l'occupation protégée (C) | important | elle lit la même vue (R-AG37) |
| M19-070 | Aucun test de **charge** de l'ouverture d'une campagne | important | 300 familles en 5 minutes (déjà §28 AG5) + 1 000 lectures de « ma soirée » le jour J |

---

## 5. Vérification réglementaire (2026-11-11)

La recherche (sous-agent, Légifrance accessible, BO et eduscol en 403, copies académiques) est versée au **§6 de la conception**, avec URL et marques. Points qui confirment ou changent ce que les personas ont relevé :

| Persona | Point | Source | Effet |
|---|---|---|---|
| P09 | Délai de convocation du CA : **8 jours** (et non 10, ancienne version), **1 jour en urgence** ; quorum : majorité, sinon nouvelle réunion sous 5 à 8 jours | R421-25 [V] | M19-035 tranché : jours calendaires, forçage « urgence » seulement là où le texte le prévoit |
| P09 | Conseil pédagogique 8 jours / 3 jours ; CVL avant chaque CA ordinaire | R421-41-4, R421-43 [V] | AGD-340 (tableau de conformité) |
| P03 | Conseil de discipline : convocation de l'élève et du représentant légal **5 jours avant, pli recommandé ou remise contre signature** | D511-31 [V] | courrier AGD-321 avec accusé, porté par le socle 13 |
| P10 | L'ESS est **réunie par l'enseignant référent** | D351-12 [V] | M19-038 confirmé : AGD-331 (réunion reçue) |
| P04, P05 | Relations avec **les deux parents**, courriers aux deux adresses ; parent sans autorité informé des choix importants | circulaire 2006-137 [V] ; C. civ. 373-2-1 [V] ; F21287 [V] | R-AG12 confirmée ; R-AG47 ajoutée |
| P05 | Faciliter les échanges avec les parents qui ne maîtrisent pas le français ; accompagnement par une personne de leur choix ; oral privilégié | circulaires 2006-137 [V], 2013-142 [V-copie] | accompagnant **de droit** (CAG-16 corrigé) ; AGD-320 et AGD-330 confirmés |
| P01 | Au moins deux rencontres par classe et par an ; réunion des nouveaux parents avant la fin de la 3ᵉ semaine | D111-1, D111-2 [V] ; circulaire 2006-137 [V] | AGD-340 |
| P08 | Réponse motivée à toute demande d'entrevue refusée | D111-4 [V] | R-AG46 |
| P17 | Journaux **12 mois au maximum** ; suppression sous 3 mois d'un compte sans vocation | kit juridique SDET 2025 [V] ; arrêté du 30 nov. 2006 [V] | §21.3 corrigé |
| P16 | Pas de données personnelles dans une URL | guide CNIL 2024, fiche 10 [V] | R-AG23 confirmée (jetons opaques) |
| Tous | Accessibilité des services numériques des EPLE | loi 2005-102 art. 47 [V] | R-AG48 (vue liste de toute grille) |
| P06 | Information des parents d'un élève majeur | aucun texte national trouvé [NT] | règle du socle 4 inchangée |

---

## 6. Manques qui relèvent d'autres socles

| Socle | Manque |
|---|---|
| 4 | Signalement « ne pas informer l'autre parent » sans interdiction judiciaire (M19-019) : examen par la direction, à porter au dossier |
| 5 | Horaires d'ouverture des bâtiments le soir (M19-001) |
| 8 | SMS de confirmation pour les personnes « SMS seulement » (M19-025, Q-N22) ; texte neutre des push sur écran verrouillé pour toute nature restreinte (M19-030) |
| 12 | Absence autorisée neutre pour un rendez-vous restreint (R-AG36) ; lecture des convocations et non-venues (M19-013) |
| 14 | Script d'accueil sur les visiteurs (M19-054) ; alerte d'interdiction |
| 27 | Mention d'information et fiches de registre (M19-059) |
| 35 | ESS convoquée par l'enseignant référent ; AESH de l'élève (M19-038, M19-039) |
| 36 | Visibilité parentale d'un rendez-vous PsyEN pris par un mineur (Q-AG17) |

---

## 7. Synthèse

### 7.1 Manques par gravité

| Gravité | Nombre | Numéros |
|---|---|---|
| Bloquant | 4 (en 4 points A–D) | M19-017, M19-022, M19-029, M19-055 |
| Important | 50 | voir tableaux (dont M19-061 à M19-063, reculs de parité) |
| Confort | 14 | |
| Différenciant | 2 | M19-006, M19-011 |
| **Total** | **70** | M19-001 à M19-070 |

### 7.2 Ajouts dans la conception (`19_AGENDA_RDV.md` §33)

Actions **AGD-320 à AGD-339** (20), règles **R-AG35 à R-AG45** (11), cas limites **CAG-61 à CAG-70**, réglages **S19-14 à S19-16**, tables `ent_agenda_session_equipe`, `ent_agenda_reservation_besoin`, colonnes `matiere_code`, `classe_id`, `presents_annonces` ; lots révisés (AG2 avance l'import .ics ponctuel ; AG4 avance l'abonnement iCal des hôtes ; AG5 reçoit R-AG35 et R-AG39).

### 7.3 Les dix ajouts les plus importants
1. Séparation dans le temps et l'espace sous interdiction (R-AG35).
2. Sortie de cours neutre pour un rendez-vous restreint (R-AG36).
3. Occupation protégée des personnels de santé et du social (R-AG37).
4. Réservation pour le compte d'une famille et courrier (AGD-320, AGD-321).
5. Meilleur parcours possible au lieu du tout ou rien (R-AG39).
6. Publication aux hôtes avant les familles (AGD-324).
7. ESS reçue d'un organisateur externe (AGD-331).
8. Indicateur de non-venue par élève, sans profilage des familles (R-AG45).
9. Injections ICS et tableur neutralisées (R-AG43).
10. Bascule des salles avec export différentiel et file des doublons (M19-064, M19-065).

### 7.4 Ce que la passe a montré sur la conception
La conception tient sur la mécanique (moteur unique, calcul plutôt que copie, minimisation par construction). Ses angles morts étaient tous des **recoupements** : une information qu'aucun écran ne montre mais que deux écrans ensemble révèlent (couloir, cours manqué, occupation de la PsyEN). Et elle supposait une famille connectée.

---

## 8. Décisions : appliquées par défaut (D-031) et soumises à Florian

### 8.1 Appliquées par défaut (D-031)
- A à D du §0 (R-AG35, R-AG36, R-AG37, AGD-320 / AGD-321).
- R-AG39 remplace R-AG32 (meilleur parcours possible, confirmation explicite).
- Pause et plafond par hôte (S19-14).
- Écart minimal sous interdiction : 45 minutes (S19-15, non réglable en dessous de 30).
- Liste du jour imprimée purgée à J+1 et marquée « document interne ».
- Indicateur de non-venue par élève seulement (R-AG45).

### 8.2 Soumis à Florian
- Q-AG17 : visibilité parentale d'un rendez-vous PsyEN pris par un mineur (à trancher avec le socle 36 et, si besoin, un juriste).
- Q-N22 (rappel) : SMS de confirmation pour les familles « SMS seulement » (coût).
- Q-AG10 (rappel) : date de bascule des salles, gel et export différentiel de M02 (actions de Florian sur la production).
- Q-AG13 (rappel) : failles Next.js ND-20, ND-30, ND-21, ND-22, ND-13.

## 8 bis. Réglages signalés (D-023)

| N° | Réglage | Défaut |
|---|---|---|
| S19-14 | Plafond de rendez-vous par hôte et par soirée ; pause toutes les 90 min | 18 ; oui |
| S19-15 | Écart minimal entre deux parents sous interdiction | 45 min (plancher 30) |
| S19-16 | Plafond de rendez-vous par famille dans la première vague horaire | 4 |

---

## 9. Faits, hypothèses, contradictions

**Faits [F]** : ceux du §32 de la conception ; aucun nouveau fait de code (la passe n'a relu que les inventaires).
**Hypothèses [H]** : taux de demande par défaut de 40 % ; durées d'écart sous interdiction (45 min) ; habitudes des familles sans numérique.
**Contradictions** : R-AG32 (tout ou rien) contre M19-023 → R-AG39 l'emporte ; R-AG6 (libre / occupé ouvert au personnel enseignant) contre C → R-AG37 l'emporte pour les personnels de santé et du social ; AGD-179 et AGD-230 (avis du cours manqué) contre R-AG7 → R-AG36 l'emporte.
