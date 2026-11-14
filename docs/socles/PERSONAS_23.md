# Passe « personas » sur le socle 23 — Bulletins, conseils de classe et passage

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** (à relire par Florian) ; les recommandations des bloquants s'appliquent par défaut (D-031). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production ; l'application Next.js n'a pas été rouverte pour cette passe.
**Branche :** `ent-socle-22-23-conception` (base `origin/ent-mc0hoe` `7467f2d`).
**Éprouve :** `docs/socles/23_BULLETINS.md` (lu en entier), avec `21_EVALUATIONS.md` (§6.2, §14, §16, R-V1 à R-V47), `22_COMPETENCES.md`, `rapport-fiches-dialogue-personas-v2.md`, `TRANSITION_T0_INVENTAIRE.md` (M10), `DECISIONS.md` (D-022, D-023, D-027, D-031, D-032, D-034, D-035, D-036) ; forme reprise de `PERSONAS_21.md`.
**Ajouts :** faits **directement** dans `23_BULLETINS.md`, au **§28** (« ajout personas ») ; rien n'est supprimé ; quand un ajout contredit le texte d'origine, le §28 l'emporte et la contradiction est écrite au §7 ci-dessous.
**Numérotation :** manques **M23-001 à M23-088** (préfixe par socle, sans collision avec les sessions parallèles) ; actions **BUL-430 à BUL-466** ; règles **R-B23 à R-B35** ; cas limites **CB-47 à CB-62** ; questions **Q-BU14 à Q-BU21** ; réglages **S23-09 à S23-12** ; points pour Florian **F23-8 à F23-11**.

---

## 0. À lire en premier : les trois bloquants et leur recommandation (appliquée par défaut, D-031)

La conception tient sur l'essentiel : verrou à la tenue, décisions versionnées, conseil qui propose et direction qui décide, voies et délais obligatoires, fiche de dialogue reprise de Next.js. En la faisant vivre par dix-neuf personas, **trois points bloquants** apparaissent. Tous trois touchent la **validité juridique de la procédure d'orientation** : sans eux, une décision peut être annulée, ou un parent peut engager l'enfant à l'insu de l'autre.

| # | Point | Pourquoi c'est bloquant | Recommandation (appliquée par défaut, D-031) | Lots |
|---|---|---|---|---|
| **A** | **Quand une décision est-elle « notifiée » ?** `orn_decision.notifie_le` est unique et la date limite d'appel en découle (§21) ; or un document déposé au coffre n'est pas lu le jour même, un courrier simple n'a pas de date de présentation connue, et l'acceptation réputée (Q-BU5) court sur ce délai (P11, P16, M23-030, M23-061) | Une famille qui n'a jamais ouvert son coffre se voit opposer une « acceptation réputée » : la décision est contestable (CJA R. 421-5 : délai non opposable sans notification régulière [V R-B13]) | **La notification est par titulaire** (`orn_notification`) : elle vaut à la **première ouverture authentifiée** du document, ou à la **remise en main propre contre émargement**, ou à l'**accusé** d'un envoi recommandé ; le dépôt au coffre et le courrier simple **ne font pas courir le délai**. La date limite est calculée **par titulaire** ; **aucune acceptation réputée** tant qu'aucun titulaire n'a été valablement notifié ; à J+5 sans notification valable, tâche bloquante au secrétariat (entretien, remise en main propre, recommandé) (R-B23, R-B24, BUL-430 à BUL-433). Applique la règle du socle 7 (D-027 D : « chaque nature déclare les modes qui valent notification ») | B8, B9 |
| **B** | **Deux titulaires, une seule réponse.** S23-07 (« un titulaire suffit pour accepter ») et `orn_reponse` acceptent qu'un parent accepte pour les deux ; rien ne dit ce qui se passe si **l'un accepte et l'autre fait appel**, ni si l'élève mineur et ses parents divergent (P11, P13, M23-028, M23-029) | La décision devient « définitive » par l'acceptation d'un parent alors que l'autre, dans le délai, voulait faire appel : privation d'une voie de recours | **Tout titulaire peut faire appel** dans **son** délai ; une acceptation n'est **définitive** qu'à l'expiration du délai de **chaque** titulaire notifié sans appel ; un appel d'un titulaire suspend l'acceptation de l'autre, qui en est informé ; l'appel ne peut être retiré que par celui qui l'a formé. S23-07 devient : « l'acceptation **expresse** d'un titulaire suffit **pour lancer la montée provisoire** (constitution des classes), la décision n'est définitive qu'à la fin des délais » (R-B25, R-B26, BUL-434, BUL-435) | B8, B9, B10 |
| **C** | **Les actes qui engagent l'enfant passent par des liens sans compte.** La conception garde, pour les familles sans compte, le **lien nominatif** du socle 7 (BUL-306, BUL-314) ; un lien transféré (boîte partagée, ex-conjoint qui a gardé l'accès à l'adresse) permet de **déposer des vœux, confirmer, accepter ou retirer un appel** au nom d'un autre titulaire (P18, M23-070 à M23-073) | Accord tacite ou acceptation obtenus par usurpation ; procédure viciée, conflit familial aggravé | Les **actes engageants** (dépôt et correction de vœux, confirmation ou contradiction, acceptation, refus, appel, retrait d'appel, maintien) exigent une **session authentifiée** du titulaire, **passkey** (D-009) et **réauthentification de moins de 5 minutes** pour accepter, faire appel ou retirer un appel ; à défaut de compte, **papier signé remis au secrétariat** (BUL-314) ou **entretien** ; le lien nominatif ne sert qu'à **lire** (R-B27, BUL-436, BUL-437) | B8 |

**Quatre décisions importantes** (ne changent pas le schéma mais doivent être tranchées avant B8) : distinguer **placement ASE** et **délégation de l'autorité parentale** pour savoir qui demande (M23-034, Q-BU15) ; **compteur de redoublements** d'un arrivant inconnu (M23-041) ; **base légale et information** du mode doublon qui traite des données réelles en parallèle (M23-080) ; **vue séance** sur un écran projeté dans une salle où passent des tiers (M23-074).

---

## 1. Méthode

**Démarche.** Celle de `PERSONAS_21.md` §1 : dix-neuf personas réalistes, prises dans la consigne du pilote et dans le §3 de la conception ; collège (client futur) et lycée (pilote privé sous contrat) ; ordinaires et particuliers ; un **attaquant** et un **auditeur RGPD**. Chacune vit son année (planification des conseils, préparation, séance, bulletin, campagne de fiches, décision, appel, montée), puis ses **incidents**. Chaque fois qu'elle voulait faire ce que la conception ne prévoit pas, ou que c'était long, ambigu, risqué ou frustrant, un manque est noté avec le paragraphe visé.

**Grille transversale** : qui propose, qui décide, qui est notifié et **quand** ; ce que voit chacun (D-022, droits du lien, vue séance) ; ce que révèle un libellé, une absence, un ordre de passage ; ce que change un événement en cours de procédure (majorité, séparation, déménagement, placement) ; ce que l'IA ajoute **sans décider** ; ce que fait déjà Next.js (M10) et ce que font Charlemagne et Pronote (D-024).

**Gravité** (identique aux passes précédentes) : **bloquant** (contournement dangereux, décision contestable, cas réel impossible, conception contradictoire) ; **important** (cas fréquent sans solution propre, perte de temps forte, parité manquante) ; **confort** ; **différenciant** (peu de logiciels du marché le font [H]).

**Vérification réglementaire.** Aucune recherche nouvelle : les points qui dépendent d'un texte renvoient au §5 de la conception (vérifié le 2026-11-11). Deux points restent **[NV]** et sont signalés : la règle de délégation de l'autorité parentale pour l'orientation d'un enfant placé (Q-BU15) et la valeur d'une ouverture authentifiée comme notification d'une décision administrative (Q-BU14, à confirmer par un juriste, F23-6).

---

## 2. Les personas en un tableau

| # | Persona | Contexte | Ce qu'elle éprouve |
|---|---|---|---|
| P01 | **Mme Duval**, cheffe d'établissement du lycée privé pilote | 1 100 élèves, 36 classes, Charlemagne officiel l'an 1 | présidence, décisions, commission propre, doublon |
| P02 | **Florian**, secrétariat de scolarité du pilote | bulletins Charlemagne, mise sous pli, campagnes de fiches de dialogue (Next.js aujourd'hui) | an 1, distribution, campagnes, report dans Charlemagne |
| P03 | **M. Caron**, adjoint, président délégué | préside 18 conseils en deux semaines | délégation, séance, PV |
| P04 | **Mme Morel**, PP de 3ᵉ (collège client) | palier 3ᵉ, 28 élèves, Affelnet | vœux provisoires et définitifs, désaccords, entretien |
| P05 | **M. Haddad**, PP de 2ᵈᵉ | choix de spécialités, voie technologique ST2S | avis par spécialité, catalogue |
| P06 | **Mme Ferrand**, professeure de spécialité SES | élèves de six classes de 1ʳᵉ | conseils simultanés, fenêtre, notes au président |
| P07 | **Mme Benali**, CPE | vie scolaire, appréciation, cas sensibles | discipline, assiduité, délégués sortis |
| P08 | **Mme Leroy**, secrétaire d'un collège client | 650 élèves, bulletins ScolIA, 40 % de familles séparées | lot d'impression, non ouverts, duplicata |
| P09 | **Emma**, déléguée des élèves de 2ᵈᵉ | 15 ans | ce qu'elle voit, ce qu'elle dit, son propre cas |
| P10 | **Mme Laurent**, déléguée des parents | parent d'un élève de la classe | confidentialité, compte rendu |
| P11 | **M. et Mme Petit**, parents séparés en conflit | exercice conjoint, résidence alternée, désaccord sur la 2ᵈᵉ pro | conflit, appel d'un seul, notification |
| P12 | **Service ASE** et **M. Roux**, père sans exercice | enfant placé, mère titulaire injoignable | qui demande, qui reçoit |
| P13 | **Inès**, 15 ans, 3ᵉ | veut un CAP cuisine, parents veulent la 2ᵈᵉ générale | avis de l'élève, conflit avec les parents |
| P14 | **Hugo**, terminale, 18 ans le 10 décembre | majeur en cours d'année | demandeur, parents, LSL |
| P15 | **Léo**, 4ᵉ, arrivé d'un autre collège | redoublement envisagé, un redoublement antérieur ailleurs | compteur, dialogue, DASEN |
| P16 | **Famille Nguyễn**, allophone, sans compte | père lit peu le français, adresse postale changeante | notification, traduction, papier |
| P17 | **M. Garnier**, membre de la commission d'appel (privé) | chef d'un autre établissement du réseau | dossier, conflit d'intérêts |
| P18 | **« Kevin »**, attaquant (élève de 1ʳᵉ) et **son père**, en litige avec la mère | tente tout | vue séance, dossiers, bulletin d'un camarade, accord tacite, rejeu, IA |
| P19 | **Mme Aubert**, DPO du réseau (auditrice RGPD) | contrôle avant l'année de doublon | minimisation, durées, accès, journaux, base légale, AIPD, IA |

---

## 3. Parcours et manques, persona par persona

### P01 — Mme Duval, cheffe d'établissement du lycée pilote
Elle prépare la rentrée 2027 : Charlemagne reste officiel, mais elle veut éprouver la séance ScolIA (mode doublon) et passer la campagne d'orientation dans ScolIA.
| Id | Manque | Gravité | § | Recommandation |
|---|---|---|---|---|
| M23-001 | Aucun moyen de **comparer** en fin de trimestre les mentions et propositions notées en doublon avec celles saisies dans Charlemagne | important | §12 | rapport d'écart doublon (BUL-438), par classe, sans nominatif hors direction |
| M23-002 | Décisions conformes en lot (BUL-340) : aucune **liste de contrôle** avant signature (élèves sans avis de l'élève, sans confirmation du second titulaire) | important | §9.4 | pré-contrôle avant décision en lot (BUL-439) |
| M23-003 | Pas de **délégation de signature** des décisions d'orientation (elle est absente la semaine des décisions) | important | §15.1 | délégation `orientation.decider` à l'adjoint, datée, au PV (BUL-440) |
| M23-004 | La **motivation** d'une décision non conforme est un texte libre : risque de motivations stéréotypées ou contestables | important | §9.4 | trame en trois rubriques (connaissances, capacités, intérêts, R-B07) avec contrôle de longueur minimale ; jamais de diagnostic ni de situation familiale (R-B28) |
| M23-005 | Aucune vue « **toutes les décisions non conformes** » avec leur échéance et l'état des appels | confort | §20 | tableau des délais (BUL-441) |
| M23-006 | Le statut « privé sous contrat » ne précise pas l'**autorité de la commission d'appel** (réseau, diocèse) ni sa saisie | important | §9.7 | paramètre « commission » (nom, adresse, dates de séance) + convocation des familles (BUL-442) |
| M23-007 | Clôture d'année (socle 5) bloquée par un appel non tranché fin juillet | important | §9.8 | décision `en_attente_appel` acceptée par la montée **provisoire**, définitive à réception (R-B26) |

### P02 — Florian, secrétariat du lycée pilote
An 1 : il dépose les bulletins de Charlemagne, les distribue, mène la campagne de fiches (aujourd'hui dans Next.js) et reporte les décisions dans Charlemagne.
| Id | Manque | Gravité | § | Recommandation |
|---|---|---|---|---|
| M23-008 | Le **journal à reporter** des décisions (BUL-378) ne dit pas quoi reporter ni quand une ligne est « faite » | important | §9.9 | ligne par décision définitive : élève, destination, date ; case « reporté le … par … » (BUL-443) |
| M23-009 | Bulletins Charlemagne : aucun **contrôle avant publication** spécifique (pages manquantes, bulletin d'un autre élève dans le PDF découpé) | important | §12 | contrôles de dépôt : nombre de pages, nom extrait du PDF ≠ élève rapproché → bloquant (BUL-444) |
| M23-010 | Mise sous pli : pas d'**étiquettes** ni de page adresse séparée pour les enveloppes à fenêtre | confort | §10.4 | page adresse du lot d'impression (socle 7) ; trame « fenêtre » |
| M23-011 | Une campagne Next.js en cours lors de la bascule : la conception dit « finir dans l'outil », mais rien ne bloque une **seconde campagne** ScolIA pour les mêmes élèves | important | §9.9 | refus de créer une campagne pour un niveau qui a une campagne déclarée « en cours dans l'ancien outil » (BUL-445) |
| M23-012 | Réponses papier (BUL-314) : pas de **scan** joint obligatoire ni de signature vérifiée | important | §17.7 | pièce obligatoire (objet du socle 7) ; mention « saisi pour le titulaire par … » |
| M23-013 | Relances : pas de **modèle de message** par étape ni de choix du canal (SMS OVH) | confort | §16 | modèles prêts par étape (D-023) ; SMS selon Q-N5 du socle 8 |
| M23-014 | Pas d'export des **non-répondants** pour l'appel téléphonique de la vie scolaire | confort | §17.7 | liste imprimable minimale (nom, classe, titulaires, téléphone) |
| M23-015 | Le gabarit « 1ʳᵉ ST2S » existe, mais rien pour une **2ᵈᵉ pro** ou une classe de **CAP** si le réseau en ouvre | confort | §17.7 | gabarits voie pro (BUL-302 étendu) |

### P03 — M. Caron, adjoint et président délégué
| Id | Manque | Gravité | § | Recommandation |
|---|---|---|---|---|
| M23-016 | Deux conseils qui se **chevauchent** (retard du précédent) : la tablette ne permet pas de suspendre une séance et d'en ouvrir une autre | important | §7.4 | état `suspendu` (retour à `en_seance`), une seule séance ouverte par président (BUL-446) |
| M23-017 | Pas de **minuteur** ni d'ordre de passage partagé avec les professeurs qui attendent | confort | §7.4 | ordre et heure estimée visibles des participants |
| M23-018 | Le PV ne distingue pas les **décisions corrigées** pendant la réouverture | important | §7.6 | PV v2 automatique avec liste des corrections |
| M23-019 | La délégation (BUL-107) ne précise pas si le délégué peut **rouvrir** | important | §15.1 | la réouverture reste au chef sauf délégation explicite |
| M23-020 | Pas d'**émargement électronique** pour un participant à distance | confort | §7.4 | présence « distance » confirmée par la personne depuis sa session |

### P04 — Mme Morel, PP de 3ᵉ (palier)
| Id | Manque | Gravité | § | Recommandation |
|---|---|---|---|---|
| M23-021 | Les vœux de 3ᵉ alimentent aussi **Affelnet** ; aucun lien entre la fiche (23) et la saisie Affelnet (37) | important | §4.1 | port `orientation::voeux_definitifs` lu par le 37 ; même rang, même destination |
| M23-022 | Entretien non conforme : pas de **trame** de compte rendu ni de rappel des pièces à apporter | confort | §9.4 | trame courte (D-023) |
| M23-023 | La **remise à niveau** proposée (D. 331-32) n'a pas de champ distinct d'une proposition | important | §8.4 | proposition « avec dispositif » (destination + dispositif proposé) |
| M23-024 | Aucune alerte quand les vœux provisoires d'un élève sont **très éloignés** de ses résultats | confort | §13 | alerte déterministe pour le PP seulement, jamais vers la famille ni l'IA de décision |
| M23-025 | Élève qui n'a **aucun titulaire joignable** (deux parents sans canal) | important | §9.3 | procédure « titulaires injoignables » : entretien, courrier, constat motivé (CB-47) |

### P05 — M. Haddad, PP de 2ᵈᵉ (spécialités)
| Id | Manque | Gravité | § | Recommandation |
|---|---|---|---|---|
| M23-026 | Les spécialités demandées ne tiennent pas compte des **combinaisons impossibles** de l'emploi du temps (barrettes) | important | §9.4 | contrôle d'offre (socle 5) et de compatibilité de barrettes (socle 10) affiché à la famille **avant** le dépôt, sans bloquer le choix (D. 331-38) |
| M23-027 | L'avis du conseil sur une spécialité n'a pas de **motif** visible de la famille | confort | §8.4 | motif court facultatif, publié avec l'avis |

### P06 — Mme Ferrand, professeure de spécialité sur six classes
| Id | Manque | Gravité | § | Recommandation |
|---|---|---|---|---|
| M23-028 | Six conseils la même semaine : elle ne peut assister qu'à deux ; une **note au président par élève** est lourde pour 70 élèves | important | §7.3 | note **par groupe** (une note, élèves cochés) + note individuelle facultative (BUL-447) |
| M23-029 | Elle ne sait pas quand **ses** élèves passent au conseil d'une autre classe | confort | §7.4 | horaires estimés de ses élèves, par conseil |

### P07 — Mme Benali, CPE
| Id | Manque | Gravité | § | Recommandation |
|---|---|---|---|---|
| M23-030 | Un fait grave récent (exclusion en cours) n'est montré qu'en nombre : le président ne peut pas en tenir compte **sans le révéler** aux délégués | important | §7.5 | indicateur « point à voir hors délégués » sur la tablette du président seulement (R-B29) |
| M23-031 | Pas de **contrôle de cohérence** entre une mention « félicitations » et une sanction en cours | confort | §8.2 | avertissement au président seul (CB-13 étendu) |
| M23-032 | L'appréciation de vie scolaire peut **citer une sanction** (contraire à R-V32) | important | §8.1 | contrôle du 21 (CE-93) appliqué aussi à l'appréciation du conseil |

### P08 — Mme Leroy, secrétaire d'un collège client
| Id | Manque | Gravité | § | Recommandation |
|---|---|---|---|---|
| M23-033 | Fratrie dans trois classes publiées à des dates différentes : trois enveloppes au même parent | confort | §10.4 | regroupement différé « attendre la fratrie » (option du lot, S23-09) |
| M23-034 | Courrier revenu : aucun **compteur** d'adresses invalides par foyer ni blocage des envois suivants | important | §13.3 | marque « adresse à vérifier » (socle 4) ; envoi suivant suspendu, tâche |
| M23-035 | Duplicata demandé par **téléphone** par un « parent » : rien ne guide la vérification d'identité | important | §10.7 | parcours guichet : identité vérifiée ou envoi au coffre seulement (R-V39) |
| M23-036 | Un parent demande la **liste des destinataires** du bulletin de son enfant (soupçon d'envoi à l'autre parent) | important | §15.2 | réponse : « les titulaires selon la décision de l'établissement », sans révéler les canaux de l'autre (R-B20) ; trace |

### P09 — Emma, déléguée des élèves
| Id | Manque | Gravité | § | Recommandation |
|---|---|---|---|---|
| M23-037 | Les délégués ne peuvent pas **préparer** le conseil (remarques de la classe) dans ScolIA | confort | §7.4 | formulaire des délégués (texte libre, sans nominatif) versé au PV (BUL-448) |
| M23-038 | La vue séance montre les **moyennes** de chaque élève à ses camarades : légitime (R-B03) mais aucune règle sur l'**ordre** (élèves en difficulté regroupés = révélation) | important | §7.4 | ordre **alphabétique** imposé en présence des délégués ; « à discuter d'abord » seulement hors délégués (R-B30) |
| M23-039 | Quand son propre cas est examiné, rien ne l'invite à **sortir** automatiquement | important | §13.1 | CB-04 rendu automatique : la tablette propose « délégués sortis » quand l'élève projeté est un délégué ou l'enfant d'un délégué |

### P10 — Mme Laurent, déléguée des parents
| Id | Manque | Gravité | § | Recommandation |
|---|---|---|---|---|
| M23-040 | Rappel de confidentialité oral seulement : aucune **trace** de son acceptation | important | §7.4 | acceptation tracée à l'émargement (« j'ai pris connaissance de l'obligation de confidentialité », R-B03) |
| M23-041 | Délai de la synthèse anonyme non fixé ; elle doit faire son compte rendu sous 48 h | confort | §7.6 | synthèse validée dans les 24 h (rappel au président) |
| M23-042 | Elle n'a pas de compte : le lien nominatif de convocation ne doit **rien** ouvrir d'autre | important | §7.2 | lien de convocation = date, lieu, ordre du jour, aucune donnée d'élève |

### P11 — M. et Mme Petit, parents séparés en conflit
| Id | Manque | Gravité | § | Recommandation |
|---|---|---|---|---|
| M23-043 | Mme accepte la décision, M. veut faire appel le lendemain : rien ne le permet | **bloquant** | §9.7 | bloquant **B** (R-B25) |
| M23-044 | La notification est datée du dépôt au coffre ; M. ouvre deux jours plus tard : son délai est déjà presque échu | **bloquant** | §9.4 | bloquant **A** (R-B23) |
| M23-045 | Conflit résolu par la direction en `maintien_famille` : laquelle des deux demandes ? Le motif est libre | important | §9.3 | résolution nommant la demande retenue et le fondement (accord écrit, décision de justice) ; jamais « au premier arrivé » |
| M23-046 | Un parent produit une **décision de justice** qui lui confie la décision d'orientation | important | §9.3 | pièce au socle 4 ; le lien porte « décide seul de l'orientation » jusqu'à la date de fin ; procédure recalculée |
| M23-047 | La fiche commune montre le **commentaire** libre de l'autre parent (« son père ne s'en occupe pas ») | important | §9.3 | commentaire visible du personnel seulement ; la fiche commune ne montre que les vœux (R-B20 précisé) |

### P12 — Service ASE et M. Roux, père sans exercice
| Id | Manque | Gravité | § | Recommandation |
|---|---|---|---|---|
| M23-048 | L'enfant est **placé** ; le service gardien n'est pas titulaire de l'autorité parentale : qui demande ? | important | §9.3 | distinguer **placement** (la demande reste aux titulaires, le service est informé et peut accompagner) et **délégation d'autorité** (le délégataire demande) [NV, Q-BU15] |
| M23-049 | Mère titulaire injoignable, père sans exercice présent : la conception refuse son dépôt (CB-19) sans issue | important | §13.2 | procédure « titulaires injoignables » (CB-47) ; le père est informé et entendu à l'entretien |
| M23-050 | M. Roux reçoit la décision (R-B15) mais elle porte les **voies d'appel** qu'il ne peut pas exercer | confort | §9.6 | exemplaire « pour information » sans formulaire d'appel |

### P13 — Inès, 15 ans, 3ᵉ, veut un CAP
| Id | Manque | Gravité | § | Recommandation |
|---|---|---|---|---|
| M23-051 | Son avis (BUL-361) est **visible de ses parents** : elle n'ose pas écrire ce qu'elle veut | important | §9.3 | avis en deux parties : ce qui est transmis aux parents et au conseil ; un message au PP seul (confidentiel, jamais au dossier) (BUL-449) |
| M23-052 | Elle ne peut pas demander un **rendez-vous** avec la PsyEN depuis sa fiche | confort | §9.3 | lien vers le socle 19 / 37 |
| M23-053 | Rien ne montre au conseil que l'élève et les parents **divergent** | important | §7.5 | indicateur « avis de l'élève différent de la demande » en vue complète ; en vue séance, la demande seule (R-B31) |

### P14 — Hugo, terminale, majeur le 10 décembre
| Id | Manque | Gravité | § | Recommandation |
|---|---|---|---|---|
| M23-054 | Ses parents ont déposé des vœux en novembre ; à sa majorité, ils voient toujours la fiche | important | §9.3 | à la majorité, l'élève choisit ce que ses parents voient de la fiche (socle 4) ; information des parents du changement de demandeur |
| M23-055 | Bulletin du 2ᵉ trimestre : il veut que ses parents ne le reçoivent plus | important | §10.4 | déjà R-V14 ; afficher à Hugo la **date d'effet** (prochaine publication) |
| M23-056 | Il conteste l'avis « doit faire ses preuves » du LSL : aucune voie de **réexamen** | important | §11 | demande d'explication (BUL-407) + réexamen possible par le chef avant l'export LSL (BUL-450) |

### P15 — Léo, 4ᵉ, redoublement envisagé
| Id | Manque | Gravité | § | Recommandation |
|---|---|---|---|---|
| M23-057 | Arrivé d'un autre collège, son **compteur de redoublements** est inconnu | important | §9.5 | compteur « non connu » ≠ 0 : la décision exige une déclaration motivée de la direction après vérification du dossier (R-B32) |
| M23-058 | Le **dialogue** préalable est tracé, mais aucune règle ne fixe qu'il précède l'avis du conseil | important | §9.5 | dialogue avant la décision, avis du conseil avant la décision ; ordre contrôlé |
| M23-059 | Le dispositif d'accompagnement (socle 35) n'est **pas vérifié** à la rentrée suivante | confort | §9.5 | tâche de rentrée « accompagnement du redoublant » (socle 35) |
| M23-060 | Redoublement **refusé** à la famille qui le demandait : la notification ne dit pas qu'elle peut faire appel (D. 331-63) | important | §9.5 | trame dédiée « rejet de la demande de redoublement » avec voies et délais |

### P16 — Famille Nguyễn, allophone, sans compte
| Id | Manque | Gravité | § | Recommandation |
|---|---|---|---|---|
| M23-061 | Décision non conforme envoyée par courrier simple : le délai court sans qu'ils aient compris | **bloquant** | §9.4 | bloquant **A** ; entretien avec **interprète** proposé |
| M23-062 | Traduction automatique de la décision : aucune garantie que la **voie d'appel** soit bien traduite | important | §14 | les mentions de voies et délais sont des **textes fixes traduits à l'avance** (relus), pas une traduction automatique |
| M23-063 | Aucun moyen de noter qu'un **interprète** (ou un aîné) était présent à l'entretien | confort | §9.4 | champ participants de l'entretien ; jamais un élève mineur comme interprète d'une décision le concernant (recommandation) |
| M23-064 | Adresse postale changeante : le bulletin revient | important | §13.3 | M23-034 ; remise en main propre à l'élève **interdite** pour une décision d'orientation (elle s'adresse aux titulaires) |

### P17 — M. Garnier, membre de la commission d'appel (privé)
| Id | Manque | Gravité | § | Recommandation |
|---|---|---|---|---|
| M23-065 | Il reçoit le dossier par un lien nominatif : **durée d'accès** et téléchargement non bornés | important | §9.7 | accès jusqu'à la date de séance + 7 jours, consultations journalisées, téléchargement filigrané (R-B33) |
| M23-066 | Il est le chef de l'établissement **d'origine** d'un élève : le contrôle de conflit (BUL-354) est déclaratif | important | §9.7 | contrôle aussi automatique (établissement d'origine, lien familial connu dans le réseau [H]) |
| M23-067 | La commission veut **entendre** la famille : rien pour la convoquer | confort | §9.7 | convocation de la famille par la commission (BUL-442) |
| M23-068 | Dossier d'appel : les pièces médicales apportées par la famille arrivent avec le reste | important | §9.7 | pièce marquée « santé, transmise sous pli à la commission » ; jamais au dossier du conseil ni à l'IA |

### P18 — « Kevin », attaquant, et son père
Kevin (1ʳᵉ) veut voir le bulletin d'un camarade et les commentaires du conseil ; son père, en litige avec la mère, veut imposer ses vœux.
| Id | Manque | Gravité | § | Recommandation |
|---|---|---|---|---|
| M23-069 | Kevin photographie la **vue séance** projetée (il est délégué) | important | §7.5 | inévitable techniquement ; rappel de confidentialité aux délégués élèves aussi ; aucune note détaillée en vue séance (déjà) |
| M23-070 | Le père a gardé l'accès à la boîte mail commune : il ouvre le **lien nominatif** destiné à la mère et **confirme** ses propres vœux | **bloquant** | §9.3 | bloquant **C** (R-B27) |
| M23-071 | Le père retire l'**appel** formé par la mère | **bloquant** | §9.7 | bloquant **C** + B : seul l'auteur retire son appel |
| M23-072 | **Rejeu** d'une requête de dépôt après la clôture de l'étape (requête capturée) | important | §9.3 | clé d'idempotence, contrôle de l'étape **côté serveur** à la date de réception, version de la procédure (R-B34) |
| M23-073 | Kevin modifie l'identifiant d'élève dans l'URL du bulletin (`/bulletins/{id}`) | important | §15.3 | identifiants opaques + contrôle d'exemplaire (le lecteur doit être destinataire) ; test de séparation |
| M23-074 | Écran projeté dans une salle vitrée ; passage de tiers | important | §7.5 | « écran de veille » d'un geste ; vue séance masquée hors séance ouverte |
| M23-075 | Kevin demande à l'assistant IA « quelle mention a eue Léa ? » | important | §14 | capacité `bulletins.publie.lire` limitée à l'exemplaire du lecteur ; refus testé |
| M23-076 | Le père demande à l'IA de rédiger un **appel** au nom de la mère | confort | §14 | l'IA n'agit que pour l'utilisateur authentifié ; aucun acte au nom d'un tiers |
| M23-077 | Kevin crée un compte « famille » avec l'e-mail de sa mère | important | §9.3 | rattachement par le socle 4 (lien confirmé), jamais par e-mail (NXFD-1) ; test |

### P19 — Mme Aubert, DPO (auditrice RGPD)
| Id | Manque | Gravité | § | Recommandation |
|---|---|---|---|---|
| M23-078 | Aucune **mention d'information** (art. 13) propre à la procédure d'orientation (finalités, destinataires, durées, droits, commission) | important | §15 | texte d'information affiché sur la fiche et la décision (BUL-451) |
| M23-079 | Les **notes au président** et avis internes contiennent des appréciations sur des personnes : communicables au titre de l'art. 15 ? | important | §15.4 | oui, communicables (comme R-V41 du 21) ; l'écran le dit ; durée courte confirmée |
| M23-080 | **Mode doublon** : traitement de données réelles dans un second outil sans finalité propre déclarée | important | §12 | inscription au registre (socle 27), base légale = mission d'intérêt public / contrat d'association, durée limitée à l'année, purge des examens doublon à la fin (R-B35) |
| M23-081 | Les **alertes** « élèves à discuter » évaluent des personnes (21 §13.8) : aucune AIPD citée pour le 23 | important | §7.3 | AIPD commune 21–23 avant la mise en service (socle 27) |
| M23-082 | Journaux de consultation 1 an : pas de **revue** périodique ni d'alerte sur les consultations anormales | confort | §15.3 | revue trimestrielle par la direction ; alerte « plus de 50 dossiers hors de ses classes en un jour » |
| M23-083 | Durée des **brouillons** de PV et des notes du secrétaire de séance non fixée | confort | §15.4 | purge à la validation du PV |
| M23-084 | Le registre annuel (exemplaire établissement) contient la rubrique **accompagnements** | important | §10.7 | conforme (exemplaire famille = établissement) mais accès restreint (direction, secrétariat), jamais au dossier de transfert (R-V37) ; à écrire |
| M23-085 | Le **dossier d'appel** transmis à la commission contient des données de la fratrie (« son frère a redoublé ») | confort | §9.7 | contrôle de pièces : avertissement si un autre élève est nommé |
| M23-086 | Pas d'export d'**accès** pour la procédure d'orientation (toutes étapes, avis internes) | important | §17.13 | BUL-423 étendu (orientation, avis, journaux de consultation de son dossier) |
| M23-087 | La **traduction automatique** envoie le texte d'une décision à un fournisseur : base légale, transfert | important | §14 | fournisseur souverain (Mistral à l'étude, CLAUDE.md), désactivé par défaut ; textes fixes pré-traduits d'abord |
| M23-088 | Les **décisions définitives** sont lues par le socle 3 et le 37 : minimisation de ce qui est transmis | confort | §4.2 | port `passage::decision` sans motivation ni compte rendu d'entretien |

### Manques transverses : l'IA, les droits, la parité
- **IA** : rien de nouveau à ouvrir ; les refus (M23-075, M23-076) et la traduction (M23-062, M23-087) sont des garde-fous.
- **Droits** : trois précisions (délégation de signature, réouverture par un délégué, accès de la commission) ; aucune action nouvelle donnée aux professeurs.
- **Parité** : la séance et le bulletin couvrent NOT-12 à NOT-15 ; la fiche de dialogue dépasse le marché (NOT-17) ; les **recours** (NOT-18) sont désormais conçus.

---

## 4. Couverture du marché : Charlemagne, Pronote, Next.js

| Fonction | Charlemagne [H, expérience de Florian] | Pronote [H] | Next.js (M10) [F] | 23 après passe |
|---|---|---|---|---|
| Préparation automatique du conseil | partielle | oui | — | oui + « ce qui a changé » |
| Séance sur tablette, mentions | oui | oui (mode conseil, mini-fiche d'aménagements à ne pas reproduire) | — | oui, vue minimisée, ordre alphabétique en présence des délégués |
| PV | à la main [H] | partiel [H] | — | oui |
| Fiches de dialogue, deux parents, conflit | non | partiel [H] | **oui** | oui + notification par titulaire, actes authentifiés |
| Appel et commission | non [H] | non [H] | **oui** (dossier) | oui + délais par titulaire, commission privée |
| Redoublement (dialogue, accompagnement, compteur) | non [H] | non [H] | partiel (vœu « maintien ») | oui |
| Bulletin par destinataire calculé à la publication | non [H] | partiel [H] | — | oui |
| Lot d'impression par adresse | partiel | oui (prestataire postal) | — | oui (local) |

---

## 5. Manques qui relèvent d'autres socles

| Manque | Socle | Ce qu'il faut |
|---|---|---|
| Modes qui valent **notification** d'une décision d'orientation (bloquant A) | 7 | nature « décision d'orientation » : ouverture authentifiée, main propre émargée, recommandé avec accusé |
| Lien « décide seul de l'orientation » (décision de justice) ; placement vs délégation d'autorité | 4 | attribut daté du lien, pièce |
| Vœux définitifs lus par Affelnet | 37 | port `orientation::voeux_definitifs` |
| Compatibilité des spécialités avec les barrettes | 10, 5 | contrôle d'offre consultable par la famille |
| Interprète pour un entretien | 19 | participant « interprète » d'un rendez-vous |
| AIPD 21–23, registre du mode doublon | 27 | fiche de traitement |
| Réauthentification passkey pour un acte engageant | 1 | `ent_session.authentification_forte_le` (D-041) réutilisée |
| Accompagnement du redoublant vérifié à la rentrée | 35 | tâche de rentrée |
| Délégués élus (mandats), formulaire des délégués | 46 | mandats datés |

---

## 6. Synthèse

### 6.1 Manques par gravité
| Gravité | Nombre | Identifiants |
|---|---|---|
| **bloquant** | 5 (trois points A, B, C) | M23-043, M23-044, M23-061, M23-070, M23-071 |
| **important** | 58 | les autres « important » des tableaux du §3 |
| **confort** | 25 | idem « confort » |
| **différenciant** | (compté en « important ») | M23-051 (avis de l'élève en deux parties) — le marché ne le fait pas [H] |
Total : **88 manques** (M23-001 à M23-088), dix-neuf personas.

### 6.2 Ajouts dans `23_BULLETINS.md`
§28 : bloquants A à C ; actions **BUL-430 à BUL-466** ; règles **R-B23 à R-B35** ; cas limites **CB-47 à CB-62** ; questions **Q-BU14 à Q-BU21** ; réglages **S23-09 à S23-12** ; modèle (réserve **2474–2476**) ; lots B8, B9, B3, B6 complétés ; points **F23-8 à F23-11**.

### 6.3 Les dix ajouts les plus importants
1. Notification **par titulaire**, valable seulement par ouverture authentifiée, main propre émargée ou recommandé (A).
2. **Tout titulaire peut faire appel** dans son délai ; définitif à la fin de tous les délais (B).
3. **Actes engageants authentifiés** par passkey, lien nominatif en lecture seule (C).
4. Procédure « **titulaires injoignables** ».
5. **Ordre alphabétique** et sortie automatique des délégués concernés.
6. **Avis de l'élève en deux parties** (transmis / au PP seul).
7. Montée **provisoire** sur acceptation expresse, définitive après les délais.
8. Compteur de redoublements « **non connu** ».
9. Accès borné et journalisé de la **commission d'appel**.
10. Information art. 13, AIPD commune, registre du **mode doublon**.

### 6.4 Ce que la passe a montré sur la conception
Le circuit était juste dans son ordre, mais **pensé pour une famille unie et connectée** : les cas de parents séparés, de familles sans compte et d'élèves qui divergent de leurs parents exigent que **chaque titulaire** ait sa notification, son délai et sa voie d'appel, et que tout acte qui l'engage soit authentifié.

### 6.5 Réglages signalés (D-023)
S23-09 (attendre la fratrie pour le lot d'impression, désactivé), S23-10 (durée d'accès de la commission, 7 jours après la séance), S23-11 (seuil d'alerte de consultations anormales, 50 par jour), S23-12 (délai avant tâche « notification non valable », J+5).

---

## 7. Faits, hypothèses, contradictions

**Faits [F]** : ceux de la conception (§2, §27 de `23_BULLETINS.md`) ; rien de nouveau lu dans le code.
**Vérifié [V]** : renvois au §5 de la conception (R-B07, R-B09, R-B13 surtout).
**Non vérifié [NV]** : valeur d'une ouverture authentifiée comme notification d'une décision administrative (Q-BU14) ; règle d'orientation d'un enfant placé sans délégation d'autorité (Q-BU15) ; recevabilité d'un appel par un seul titulaire contre l'avis de l'autre [H : chaque titulaire exerce l'autorité ; à confirmer, F23-6].
**Hypothèses [H]** : pratiques de Charlemagne et Pronote (§4) ; contrôle automatique des conflits d'intérêts dans un réseau.
**Contradictions relevées (tranchées au §28)** :
- **C1** S23-07 / Q-BU6 (« un titulaire suffit pour accepter ») contre le droit d'appel de l'autre titulaire : **tranché** par R-B25 (acceptation expresse = montée provisoire ; définitive à la fin de tous les délais).
- **C2** Q-BU5 (acceptation réputée) et `orn_decision.notifie_le` unique contre CJA R. 421-5 : **tranché** par R-B23, R-B24 (notification par titulaire, aucun silence opposable sans notification valable).
- **C3** BUL-306, BUL-314 (lien nominatif pour les familles sans compte) contre la sécurité des actes engageants : **tranché** par R-B27 (lien en lecture seule).
- **C4** R-B20 (« ce que la fiche commune montre aux deux ») ne précisait pas le **commentaire** libre : précisé (M23-047, R-B20 complété au §28).
- **C5** BUL-361 (avis de l'élève visible du PP **et de la famille**) contre la liberté d'expression de l'élève mineur : complété (avis en deux parties, BUL-449).
