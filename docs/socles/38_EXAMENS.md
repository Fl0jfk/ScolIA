# Socle 38 — Examens et certifications

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian) ; recommandations appliquées par défaut (D-031), listées au §26. Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production. Code Next.js **lu seulement**, de façon ciblée (CLAUDE.md).
**Branche :** `ent-socle-37-38-conception`, partie de `origin/ent-mc0hoe`.
**Passe personas :** `docs/socles/PERSONAS_38.md` ; ajouts faits **ici**, marqués **« ajout personas »** (§28) ; en cas de contradiction, le §28 l'emporte.
**À lire avec :** `CLAUDE.md` ; `DECISIONS.md` D-017, D-021 à D-036, D-037 à D-046 (E1–E14) ; `SOCLES.md` §38 ; `10_EMPLOI_DU_TEMPS.md` (l. 347 et CL-15 : neutralisation) ; `12_ASSIDUITE.md` (`en_examen`, l. 220, 303) ; `19_AGENDA_RDV.md` (R-AG21 : l'examen national prime) ; `20_CAHIER_DE_TEXTES.md` (CC-56, CDT-254) ; `21_EVALUATIONS.md` (R-05, §4 point 4, CE-46, CE-64, EVA-216, `eva_evaluation_commune`) ; `22_COMPETENCES.md` (R-C13, CCF) ; `23_BULLETINS.md` (LSL, avis pour l'examen) ; `34_SANTE.md` (dispenses, N-0/N-1/N-2) ; `09_INTEROP.md` (FX-09, FX-34, FX-37, FX-38, FX-64, X0, X7, D-2) ; `13_DISCIPLINE.md` ; `15_ABSENCES_PERSONNELS.md` ; `TRANSITION_T0_INVENTAIRE.md` (M17 invitations, M08 HSE).
**Conventions** : comme au 37 ([F], [V], [V-r] résumé de moteur pointant vers une page officielle — les sites education.gouv.fr et eduscol ont renvoyé **403** le 2026-11-11 —, [V*] repris d'une conception qui l'a vérifié, [NV] interdit de coder dessus, [H], [R]).
**Numérotation (préfixes vérifiés libres) :** actions **EXA-100 à EXA-499** (EXA-01 à 07 de `SOCLE2_DOMAINES.md` repris par renvoi) ; règles **R-EX1…** ; réglementaire **RG-X01…** ; cas limites **CEX-01…** ; défauts Rust **XD1…** ; existant Next.js **NXI-01…**, défauts **NXID-01…** ; questions **Q-EX1…** ; réglages **S38-01…** ; points pour Florian **F38-1…** ; lots **EX0…EX18** ; migrations **2760–2779** (plage imposée, vérifiée libre sur les 134 branches distantes).

---

## 0. En bref

1. **Thèse** : un examen est **une organisation éphémère et sans droit à l'erreur** — des centaines d'élèves, des salles, des surveillants, des sujets confidentiels, des aménagements à respecter **à la minute**, puis des copies, des notes et des résultats. Ce que l'établissement redoute n'est pas la saisie, c'est **l'oubli** : un tiers-temps non appliqué, un surveillant absent, un sujet ouvert trop tôt, une convocation à la mauvaise salle. Le socle fait de chaque épreuve un **plan vérifiable** avant le jour J (§1).
2. **Trois familles** dans un seul moteur : (a) **épreuves internes** (brevet blanc, bac blanc, oraux blancs, épreuves communes, épreuve anticipée de mathématiques blanche) que l'établissement organise de A à Z ; (b) **examens nationaux** (DNB, baccalauréats général, technologique et professionnel, CAP) dont l'établissement **prépare** les inscriptions (Cyclades), **organise** ce qui lui revient (oral du DNB, grand oral blanc, **CCF** d'EPS et de la voie professionnelle, centre d'examen s'il l'est) et **rapatrie** les résultats ; (c) **certifications et attestations** (Pix, ASSR 1 et 2, attestation de savoir-nager, PSC1) (§7 à §15).
3. **Le réglementaire** [§5] : **DNB à compter de la session 2027** : contrôle continu **40 %** (moyennes annuelles des disciplines obligatoires de 3ᵉ, coefficient 1 chacune), épreuves **60 %** (français 2, mathématiques 2, histoire-géographie 1,5 + EMC 0,5, sciences 2, **oral 2**) [V-r BO spécial n° 4 de 2026] ; moyennes transmises **par le LSU** à Cyclades [V-r] ; **épreuve anticipée de mathématiques** en 1ʳᵉ (le 21 juin 2027, 2 h, coefficient 2) [V-r] ; LSL déposé sur Cyclades au plus tard le 15 juin 2027 [V-r] ; **inscriptions par l'établissement dans Cyclades**, confirmation **signée** par le candidat et son représentant légal s'il est mineur [V-r] ; **CCF** pour le public et le privé sous contrat en voie professionnelle [V-r] ; **aménagements** : demande au **médecin désigné par la CDAPH**, avis, **décision de l'autorité organisatrice** notifiée avec voies de recours, tiers-temps au plus [V-r C. éduc. D. 351-27 et suivants, circulaire de 2020] ; **fraude** : procès-verbal, saisine du recteur, commission de discipline [V-r D. 334-25 et suivants] ; **résultats à la presse seulement avec consentement** [V-r CNIL 2012-177, à reconfirmer] ; **aucune interface éditeur** trouvée pour Cyclades ni Notanet [NV] (D-029 : échanges nationaux réservés aux éditeurs inscrits).
4. **Existant Rust** [F] : un seul fichier, `notanet_export.rs`, qui exporte **toutes les notes individuelles** d'une période par élève et par matière sous le nom « Notanet / notes bac » — **ni le format ni le contenu** du DNB (qui veut des moyennes annuelles). **Lot EX0** : retrait de cet export de l'interface ou renommage strict « relevé de notes de travail » (déjà demandé par le lot X0 du 9), **exclusion des notes non publiées** si elles y entrent [à vérifier, XD2], journal de l'export.
5. **Existant Next.js** : rien sur l'organisation des examens ; le module **invitations (M17)** fait la **cérémonie de remise des diplômes** (bac et brevet) : liste d'éligibles importée, réponses des familles, nombre de personnes, situation de l'ancien élève — **repris** (EXA-440 à 452) avec un **défaut grave en production** (ND-01 du 37 : la page publique renvoie e-mail du parent, situation et confirmation qu'un élève est lauréat à qui connaît son nom ; F38-1 = F37-1).
6. **Les aménagements, cœur du risque** (§10) : le 38 **possède la décision d'aménagement d'examen** (mesures accordées, session, autorité, date, recours) — jamais le diagnostic, jamais un certificat médical (ils vont au médecin désigné, pas à l'établissement) ; le 35 possède le plan (PAP, PPS) et les aménagements pédagogiques ; les mesures **pilotent** automatiquement salles, durées, secrétaires, sujets adaptés et convocations ; aux épreuves internes, les mesures décidées **s'appliquent par défaut**, les mesures demandées et non encore décidées s'appliquent **en provisoire** (Q-EX3).
7. **Sujets confidentiels** (§9) : dépôt par l'auteur, accès réservé, **chaque ouverture journalisée**, impression en lot par le reprographe désigné, **aucune diffusion avant l'heure de l'épreuve** (pas même à l'IA) ; sujets adaptés (agrandis, braille) liés aux mesures.
8. **IA** (§16) : propose un placement, une répartition des surveillances, un planning d'oraux, vérifie la cohérence (« 3 élèves en tiers-temps sans salle »), rédige un brouillon de convocation ; **jamais** une note, une correction, une décision d'aménagement, ni une lecture de sujet avant l'heure.
9. **Chiffres** : **216 actions** (180 dans la conception, EXA-100 à EXA-452, et 36 ajoutées par la passe personas, EXA-460 à EXA-495), **58 cas limites**, **34 règles** (dont 4 de la passe personas), **19 lots** (EX0 à EX18), **19 migrations** planifiées dans 2760–2778, 2779 pour les retraits.
10. **Pour Florian** (§26.2) : ND-01 (production) ; l'établissement est-il **centre d'examen** (DNB, bac, épreuves anticipées) ? démarche éditeur et question à la DNE sur Cyclades ; rémunération des surveillances d'examens blancs dans le privé (HSE ?) ; DPO (copies, résultats, aménagements) ; juriste (publication des résultats, cérémonie).

---

## 1. Thèse : une organisation éphémère sans droit à l'erreur

**Ce que vivent les établissements aujourd'hui [H, expérience de terrain]** :
- le **brevet blanc** se prépare sur un tableur : liste des élèves par salle, tableau des surveillances recopié trois fois, convocations faites à la main ; une semaine avant, un professeur découvre qu'il surveille pendant son propre cours ;
- les **tiers-temps** sont listés de mémoire ; au jour J, un élève en tiers-temps est placé dans une salle qui ferme à l'heure normale ; le secrétaire-lecteur n'a pas été prévenu ;
- les **sujets** circulent par e-mail entre collègues, s'impriment la veille sur la photocopieuse commune ;
- les **cours** des classes non concernées tombent dans des salles réquisitionnées ; personne n'a prévenu les élèves ;
- pour les **examens nationaux**, le secrétariat vérifie les inscriptions Cyclades ligne à ligne, court après les confirmations signées, imprime les convocations ; en juillet, il recopie les résultats dans le logiciel pour le dossier de l'élève ;
- le **CCF** d'EPS ou de la voie professionnelle laisse des grilles papier, des PV signés à retrouver, des inaptitudes mal tracées.

**Ce que redoutent les acteurs [H]** : un aménagement **non appliqué** (recours, épreuve à refaire, injustice) ; un sujet **divulgué** ; une épreuve sans surveillant ; un élève **oublié** dans le placement ; une confirmation d'inscription non signée découverte trop tard ; une fraude mal procédurée ; une note de CCF perdue ; un résultat publié contre l'avis d'une famille.

**Ce qui les ravirait [H]** : un **plan d'épreuve contrôlé** (« tout est prêt » ou « 4 problèmes ») ; placement et surveillances **proposés** en un geste, équitables, respectant les emplois du temps ; convocations **personnelles** sur l'application (élève, famille, surveillant) ; les cours touchés **neutralisés** et annoncés automatiquement ; une **feuille d'émargement** sur tablette ; les copies anonymisées et réparties aux correcteurs ; les notes qui **arrivent dans le carnet de notes** (21) sans ressaisie ; les résultats officiels **dans le dossier** ; la cérémonie organisée en une après-midi.

**Principes [R]** :
1. **Un seul moteur d'épreuve** pour l'interne, le national organisé localement, et les certifications : session → épreuves → candidats → salles → places → surveillances → convocations → émargement → copies → notes → résultats.
2. **Les aménagements gouvernent le plan** : ils ne sont pas une colonne d'information, ils **contraignent** salle, durée, personnes, sujet ; un plan qui ne les respecte pas n'est pas publiable (R-EX8).
3. **Le national reste national** : Cyclades, les sujets nationaux, les jurys, les décisions d'aménagement et de fraude appartiennent à l'autorité ; ScolIA **prépare, trace, organise ce qui revient à l'établissement, rapatrie**.
4. **Les notes appartiennent au 21** : une épreuve interne crée une **évaluation commune** (21) ; le 38 organise et anonymise, il ne calcule pas de moyenne.
5. **Minimisation** : un surveillant voit « tiers-temps » ou « salle à part » pour **son** candidat, jamais pourquoi ; un professeur voit les aménagements de ses élèves (35) ; personne ne voit un diagnostic.
6. **Confidentialité des sujets** par conception : moins de personnes, chaque accès journalisé, rien avant l'heure.

---

## 2. Évaluation franche de l'existant

### 2.1 Rust [F]
| Élément | Où | Constat |
|---|---|---|
| Export « Notanet » | `crates/domain-eleve/src/notanet_export.rs` (en-tête « Export Notanet / notes bac (P2 stub CSV) ») ; `ent-db/src/service.rs:15157-15209` ; route `/exports/notanet` (`ent-api/src/routes.rs:595`) | CSV `ine;nom;prenom;matiere;note` : **une ligne par note** de la période (`notes.list_for_eleve` filtré sur `periode_id`), convertie sur 20 ; code matière « ? » si inconnu ; droit `exports.officiels` (professeur refusé, `tests_d022.rs:368`) |
| Examens | — | **aucune** table, aucune route, aucun écran |
| Reflets ailleurs | 12 : état `en_examen` prévu (`12_ASSIDUITE.md` l. 303) ; 10 : exception « absence établissement » (`377_edt_exception_absence_etab.sql`) ; 21 : `eva_evaluation_commune` **conçue** (`21_EVALUATIONS.md` l. 1171) ; 19 : priorité des examens nationaux (R-AG21) | points d'accroche, rien de codé pour le 38 |

### 2.2 Défauts (lot EX0)
| # | Défaut | Conséquence | Où [F] | Gravité | Correction (EX0) |
|---|---|---|---|---|---|
| **XD1** | L'export porte un **nom officiel** (« Notanet / notes bac ») pour un contenu qui n'est **pas** celui du DNB (notes individuelles, pas de moyennes annuelles, pas d'oral, pas d'épreuves) | un secrétariat croit tenir un export DNB ; risque de dépôt erroné ou de décision sur de faux chiffres | `notanet_export.rs:1-30` | **important** | avec le lot **X0 du 9** : libellé « relevé de notes de travail (non officiel) », jamais « Notanet » ni « bac » à l'écran ; retiré des menus « exports nationaux » |
| **XD2** | L'export prend **toutes les notes** de la période par `list_for_eleve`, sans filtre visible de **publication** ni de statut (absent, dispensé, non noté) [à vérifier dans `notes.list_for_eleve`] | des notes **non publiées** à l'élève peuvent sortir dans un fichier ; un « absent » devient 0 ou disparaît | `service.rs:15180-15205` | important | filtrer sur les notes **publiées** et **comptées** ; colonnes de statut ; test |
| **XD3** | Export **non journalisé** (aucun `ent_journal_modif` visible dans la fonction) | sortie de données sans trace (E7) | `service.rs:15157-15209` | moyen | journal de l'export (qui, quoi, combien de lignes) |
| XD4 | Conversion sur 20 **recalculée** à l'export (`valeur * 20 / sur`) | une règle de calcul hors du 21 (propriétaire des notes) | `service.rs:15193-15197` | mineur | lecture des valeurs du 21 telles quelles ; supprimé avec XD1 |

### 2.3 Next.js [F, lecture seule et ciblée]
Aucun module d'examens (recherche de `brevet blanc`, `bac blanc`, `examen`, `tiers-temps`, `grand oral`, `cyclades`, `notanet` dans `app/` : seules occurrences « brevet » dans **invitations**).

**Module « invitations » (M17), cérémonie de remise des diplômes** (`drizzle/0054_invitation_ceremonie.sql`, `0057_invitation_eligible_situation.sql`, `app/lib/invitation-db.ts`, `app/api/invitation/**`, `app/components/invitation/*`) :
| Id | Action Next.js | Reprise |
|---|---|---|
| NXI-01 | Créer une page d'invitation par événement (titre, introduction, thème « remise de diplôme », lieu, début et fin, **mode diplôme** : bac, brevet ou les deux) | EXA-440 |
| NXI-02 | Capacité totale et **maximum de personnes par élève** | EXA-441 |
| NXI-03 | Importer la **liste des éligibles** (prénom, nom, date de naissance, diplôme) par copier-coller ou Excel, en ajout ou remplacement | EXA-442 (depuis les **résultats officiels** du 38, plus d'import) |
| NXI-04 | Exiger ou non d'être dans la liste des éligibles | EXA-442 |
| NXI-05 | Réponse des familles : présence, nombre de personnes, e-mail, diplôme | EXA-444 (dans le compte ou par lien nominatif) |
| NXI-06 | Date limite de réponse | EXA-441 |
| NXI-07 | **Situation de l'ancien élève** demandée (`off`, `always`, `bac_only`) : études, emploi, autre, détail, établissement | **socle 37** (devenir, ORI-332) |
| NXI-08 | Tableau de bord des réponses, export | EXA-446 |
| NXI-09 | Détection et résolution des **doublons** de réponses | sans objet (identité ScolIA) |
| NXI-10 | Notification par e-mail d'une adresse de l'établissement | EXA-445 (socle 8) |

| # | Défaut | Où | Gravité | Dans ScolIA |
|---|---|---|---|---|
| **NXID-01** | = **ND-01 du 37** : page publique, correspondance sur 2 critères sur 3 (prénom et nom suffisent), renvoie e-mail du parent, situation, établissement fréquenté, nom exact ; **révèle qu'un élève est lauréat** | `invitation-db.ts:645-750` ; `lookup/route.ts:21-37` | **grave** (production) | **F38-1** (Florian corrige Next.js) ; ScolIA : réponse dans le compte ou lien nominatif muet avant authentification (R-EX30) |
| NXID-02 | Éligibles **recopiés** à la main depuis les résultats (aucun lien avec la base élèves) | NXI-03 | moyen | éligibles = **lauréats** issus des résultats officiels rapatriés (EXA-442) |
| NXID-03 | Le résultat d'un élève (lauréat) est utilisé pour une invitation **sans** que la famille ait été informée de cet usage | — | moyen | information au moment du rapatriement ; opposition possible (R-EX29) |
| NXID-04 | Aucune durée de conservation (T0 : « perdable après l'événement ») | — | mineur | 3 mois après l'événement (§17.4) |

**À ne pas perdre (D-035)** : la cérémonie elle-même (moment fort, attendu des familles [H]) ; la limite de personnes par élève ; la date limite ; le tableau des réponses.

### 2.4 Charlemagne et Pronote [H, `MATRICE_CHARLEMAGNE.md`]
Les outils du marché gèrent des **devoirs communs** et parfois des **plannings d'examens blancs** ; la gestion des aménagements, des sujets confidentiels, du CCF et des résultats officiels reste **sur papier ou tableur** [H, à confirmer par Florian (F38-6)].

### 2.5 Verdict
**Tout est à construire**, sauf un export trompeur à retirer (EX0) et une cérémonie à reprendre proprement.

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Chef d'établissement** (chef de centre le cas échéant) | arrête le calendrier, préside, responsable des sujets, des aménagements appliqués, de la fraude, signe | tout vérifier soi-même la veille | aménagement oublié ; sujet divulgué ; fraude mal procédurée | plan contrôlé, « tout est prêt » |
| **Adjoint, responsable des examens, secrétariat des examens** | organise : sessions, salles, placements, surveillances, convocations ; inscriptions Cyclades ; résultats | tableurs, recopies, relances de signatures | une inscription non confirmée ; une salle double | propositions en un geste, listes de manques |
| **Professeurs** | conçoivent les sujets, surveillent, corrigent, font passer les oraux, évaluent en CCF | surveillances mal réparties, copies mal distribuées, grilles papier | surveiller pendant leur cours ; perdre une copie | leurs surveillances et copies dans leur agenda ; grilles en ligne |
| **Professeurs d'EPS** | CCF : protocoles, situations d'évaluation, inaptitudes | inaptitudes au dernier moment, PV | une note manquante | épreuve adaptée liée à la dispense (34) |
| **Professeurs de la voie pro** | CCF, chef-d'œuvre, PFMP (39) | grilles du référentiel recopiées | perdre un PV | situations et PV tracés |
| **CPE, AED, vie scolaire** | surveillances, appel des candidats, retardataires, incidents | élèves absents à l'épreuve | élève perdu dans l'établissement | émargement en direct, absents signalés |
| **Infirmière** | soins pendant l'épreuve ; trousse PAI | — | malaise sans information | consignes minimales (34) |
| **AESH, secrétaires-lecteurs** | assistent un candidat | convocation tardive | mauvaise salle | convocation personnelle avec la mesure |
| **Agents, reprographie** | préparent salles, impriment les sujets | sujets reçus la veille | fuite | lot d'impression sécurisé, liste de salles |
| **Référent handicap, coordonnateur ULIS** (35) | prépare les demandes d'aménagement | dossiers incomplets, délais | demande hors délai | liste des élèves concernés, échéances |
| **Médecin scolaire** | (public) avis | — | — | rien dans ScolIA (secret médical) |
| **Élève mineur, élève majeur** | passe l'épreuve, signe sa confirmation d'inscription | ne pas savoir où aller | se tromper de salle ; aménagement oublié | convocation sur l'application, rappel la veille |
| **Famille** (titulaires, séparés) | signe la confirmation d'inscription (mineur), demande l'aménagement | papiers à rendre | rater la demande d'aménagement | échéances claires, signature en ligne |
| **Candidats extérieurs** (si centre d'examen) | passent l'épreuve dans l'établissement | — | — | — |
| **Surveillants convoqués par l'autorité** (centre d'examen) | surveillent | — | — | — |
| **Autorité organisatrice** (rectorat, SIEC) | inscriptions, sujets, aménagements, jurys, résultats | — | — | — |
| **Auditeur RGPD / DPO** | contrôle | — | fuite de données d'aménagement | registre, durées, journal |

---

## 4. Frontières

| Sujet | 38 | Autres |
|---|---|---|
| Sessions, épreuves, candidats, salles, places, surveillances, convocations, émargement, incidents, copies | **possède** | 5 (salles), 6 (personnels), 19 (réservations, priorité R-AG21), 10 (neutralisation), 12 (`en_examen`) |
| Notes des épreuves internes | **organise** (anonymat, répartition) | **21 possède** les notes (évaluation commune) ; publication au 21 |
| Notes et résultats des **épreuves organisées localement pour un examen national** (oral du DNB, CCF) | **possède** jusqu'à la transmission | 9 transporte (bloqué D-2) |
| Moyennes de contrôle continu du DNB et du bac | **lit** | **21** calcule, **22** (LSU) et **23** (LSL) transmettent |
| Aménagements d'examen (décision, mesures) | **possède** | **35** possède PAP, PPS, aménagements pédagogiques ; **34** les dispenses d'EPS et consignes minimales |
| Inscriptions aux examens nationaux (préparation, confirmations signées) | **possède** la préparation et la trace | Cyclades (national) ; **7** signatures et pièces |
| Résultats officiels, relevés, diplômes, remise | **possède** | 7 (attestations, registre de remise), 2 (dossier) |
| Cérémonie de remise des diplômes | **possède** (reprise M17) | 19 (événement), 8 (invitations) |
| Devenir de l'ancien élève | — | **37** |
| Certifications (Pix, ASSR, ASNS, PSC1) : sessions et résultats | **possède** | 22 et 23 (attestations au LSU et au LSL) |
| Fraude : procès-verbal, transmission | **possède** la trace | autorité (commission) ; **13** pour une procédure disciplinaire interne |
| Absence d'un personnel convoqué par l'autorité (surveillance, correction, jury) | déclare | **15** remplace, **10** adapte |
| Rémunération des surveillances (privé, HSE ?) | — | 49 [F38-4] |

### 4.1 Ports exposés [R]
- `examens::calendrier(tx, perimetre) -> [Epreuve]` (dates, publics) — 10, 12, 19, 20 (charge), 21, 29, familles.
- `examens::candidat_occupe(tx, eleve, instant) -> Option<Occupation>` — 12 (`en_examen`), 14, 42.
- `examens::surveillance_de(tx, personne, instant) -> Option<Surveillance>` — 10, 15.
- `amenagements::mesures(tx, eleve, session) -> [Mesure]` — 21 (évaluations ordinaires si l'établissement l'étend, Q-EX12), 35 (lecture), 42.
- `resultats::officiels(tx, eleve) -> [Resultat]` — 2, 23, 37 (admission, devenir), 47.
- `certifications::obtenues(tx, eleve) -> [Certification]` — 22, 23, 37.

### 4.2 Ports consommés [R]
5 (salles et capacités, niveaux, classes, groupes, calendrier, fermetures) ; 6 (personnels, services, quotités) ; 10 (séances, `seances_touchees`, création d'exceptions) ; 19 (réservations) ; 12 (appel, absence) ; 21 (création d'une évaluation commune et de ses évaluations par cours, statuts) ; 35 (PAP, PPS, dispositifs : **drapeau** et mesures pédagogiques, jamais le diagnostic) ; 34 (dispense d'EPS : **conséquence** seulement, consignes minimales « trousse d'urgence ») ; 4 (titulaires, majorité) ; 7 (signature, objets, documents émis, liens nominatifs) ; 8 (avis) ; 9 (fichiers de résultats, X7) ; 15 (remplacement) ; 18 (consentements : publication) ; 39 (PFMP pour la voie pro).

---

## 5. Ce qui est réglementaire (vérifié le 2026-11-11)

| # | Règle | Source (consultée le 2026-11-11) | Statut | Conséquence pour ScolIA |
|---|---|---|---|---|
| RG-X01 | **DNB, session 2027** : contrôle continu **40 %** (moyennes annuelles de toutes les disciplines obligatoires de 3ᵉ, coefficient 1 chacune) ; épreuves terminales **60 %** | BO spécial n° 4 de 2026 (MENE2623194N) — https://www.education.gouv.fr/bo/2026/Special4/MENE2623194N ; eduscol 5604 — https://eduscol.education.gouv.fr/5604/modalites-d-attribution-du-diplome-national-du-brevet | [V-r] ; cohérent avec 21 R-05 [V] | simulation interne seulement (R-EX20) ; moyennes du 21 |
| RG-X02 | **Épreuves DNB 2027** : français 2, mathématiques 2, histoire-géographie 1,5 + EMC 0,5, sciences 2 (deux disciplines sur trois, deux exercices sur 10), **oral de soutenance 2** ; candidats individuels : langue vivante coefficient 2 ; notes d'épreuves arrondies au point supérieur | BO n° 4 de 2026 (MENE2623228N) — https://www.education.gouv.fr/bo/2026/Hebdo4/MENE2623228N | [V-r] | brevet blanc : **gabarit DNB 2027** préréglé (EXA-112) ; oral organisé par l'établissement (§12.1) |
| RG-X03 | « Tous les élèves scolarisés sont aptes à se présenter au DNB ; l'exemption ne peut être envisagée qu'exceptionnellement » | mêmes sources | [V-r] | candidats d'un brevet blanc = tous les 3ᵉ par défaut ; exemption = cas tracé |
| RG-X04 | **Brevet condition d'accès en 2ᵈᵉ**, place du socle dans le calcul 2027 | — | [NV] (non confirmé) | **ne rien coder** |
| RG-X05 | **Flux des notes du DNB** : moyennes validées saisies dans le **LSU**, transmises à **Cyclades** ; « Notanet » absent des sources trouvées | eduscol 5604 (résumé) | [V-r] partiel ; format [NV] | l'export « Notanet » existant est trompeur (XD1) ; transmission par le LSU au **22** |
| RG-X06 | **Baccalauréat 2027** : calendrier au BO spécial n° 2 de 2026 ; épreuves de remplacement le 10 septembre 2027 | https://www.education.gouv.fr/bo/2026/Special2/MENE2622686N | [V-r] | dates = paramètres |
| RG-X07 | **LSL** : version finalisée déposée sur Cyclades **au plus tard le 15 juin 2027** | BO n° 36 de 2026 (MENE2624207N) ; mémento eduscol 2025 | [V-r] | échéance au **23** ; le 38 l'affiche au calendrier |
| RG-X08 | **Épreuve anticipée de mathématiques** (arrêté du 10 juin 2025) : en 1ʳᵉ, première passation juin 2026 pour le bac 2027 ; session 2027 : **21 juin 2027 matin, 2 h, 20 points, coefficient 2, sans calculatrice**, automatismes en QCM (6 points) + 2 à 3 exercices (14 points) ; trois sujets selon le parcours ; résultats transmis à Parcoursup | https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000051714285 ; https://www.education.gouv.fr/reussir-au-lycee/epreuve-anticipee-de-mathematiques-en-classe-de-premiere-450607 | [V-r] ; incohérence « depuis la session 2026 » sur une page (C3) | gabarit d'épreuve blanche « EAM » (3 parcours, QCM) ; parcours de l'élève lu au 5 |
| RG-X09 | **Inscriptions** : pour un élève scolarisé (public, privé sous contrat ou hors contrat), l'établissement **inscrit dans Cyclades** ; la **confirmation d'inscription** est éditée par l'établissement, **signée par le candidat et son représentant légal s'il est mineur**, puis déposée ; sans signature, candidature non validée (échéance 2026 : 6 janvier ; 2027 [NV]) | SIEC — https://siec.education.fr/candidats/examens/BP/inscriptions/inscriptions-19 ; eduscol 5682 — https://eduscol.education.gouv.fr/5682/inscriptions-au-baccalaureat-general-et-technologique | [V-r] | **circuit de confirmation signée** (§13.2) ; échéance paramétrée ; liste des non signées |
| RG-X10 | **Convocations nationales** disponibles dans Cyclades environ trois semaines avant chaque épreuve ; l'établissement y accède pour ses candidats | mêmes sources | [V-r] | ScolIA n'émet **pas** la convocation nationale ; il rappelle et peut joindre le PDF reçu (§13.4) |
| RG-X11 | **Voie professionnelle** : élèves du public, du privé sous contrat et des CFA habilités évalués en **CCF** pour les épreuves générales ; hors contrat et CFA non habilités en ponctuel ; **chef-d'œuvre** (CAP) et projet (bac pro) : 50 % notes consignées durant l'élaboration + 50 % oral devant 2 enseignants (général et professionnel), à partir de mai, 10 min (CAP), 15 min (bac pro) | arrêté du 30 août 2019 (CAP) — https://www.legifrance.gouv.fr/loda/id/JORFTEXT000039034347 ; arrêté du 17 juin 2020 (bac pro) — https://www.legifrance.gouv.fr/loda/id/JORFTEXT000042080678 ; BO n° 41 de 2020 | [V-r] | CCF (§12.3) ; gabarit d'oral du chef-d'œuvre |
| RG-X12 | **EPS en CCF** | BO n° 4 de 2021 (MENE2037057C) — https://www.education.gouv.fr/bo/21/Hebdo4/MENE2037057C.htm | [V-r] (existence) ; protocoles, inaptitude [NV] | situations d'évaluation, inaptitude liée à la dispense (34) ; règles de note [NV] → saisies par l'établissement |
| RG-X13 | Détail du **contrôle continu du bac** (40 %), grand oral, langues vivantes, options | mémento eduscol 2025 (non relu en détail) | [NV] | moyennes au 21 et LSL au 23 ; grand oral **blanc** organisé (§12.2) sans règle nationale codée |
| RG-X14 | **Aménagements pour candidats en situation de handicap** : C. éduc. D. 351-27 à D. 351-31 (plage à confirmer) ; **majoration de temps au plus d'un tiers** du temps de chaque épreuve ; adaptation ou dispense **seulement si le règlement de l'examen le prévoit** ; demande au **médecin désigné par la CDAPH**, au plus tard à la **date limite d'inscription** (sauf handicap révélé après) ; avis motivé, puis **décision de l'autorité organisatrice** notifiée avec voies de recours ; candidats individuels et CNED : médecin désigné du département | https://www.legifrance.gouv.fr/codes/id/LEGISCTA000006166867 ; circulaire BO n° 47 du 10 décembre 2020 — https://www.education.gouv.fr/media/73400/download ; eduscol 5535 | [V-r] | **décision** au 38 (§10) ; contrôle « tiers-temps ≤ 1/3 » (R-EX9) ; échéance de demande = date d'inscription (R-EX10) ; aucun certificat médical dans ScolIA (R-EX11) |
| RG-X15 | Procédure simplifiée pour élèves en PAP ou PPS, formulaires de l'année, liste des mesures courantes | — | [NV] | liste des mesures = **référentiel** chargé (secrétaire, lecteur, salle à part, ordinateur, sujet agrandi, braille, pauses…) [H], modifiable |
| RG-X16 | **Fraude** (bac général, D. 334-25 et suivants) : en flagrant délit, le surveillant **fait cesser** sans interrompre l'épreuve, **saisit les pièces**, dresse un **procès-verbal** contresigné par les autres surveillants et l'auteur (refus mentionné) ; le chef de centre **saisit le recteur sans délai** ; fraude découverte à la correction : rapport du correcteur ; aucune pièce de résultat avant la décision de la commission (délai indicatif deux mois) ; procédure renforcée par un décret du 27 février 2026 [NV] | https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000025818132/ ; https://www.education.gouv.fr/reussir-au-lycee/fraudes-aux-examens-341483 | [V-r] ; décret 2026 [NV] | **trame de PV** (§11.6) pour les examens nationaux tenus dans l'établissement ; pour les épreuves internes, incident vers la direction et éventuellement le **13** (jamais de sanction automatique) |
| RG-X17 | Téléphones éteints et rangés ; détecteurs dans toutes les académies | même page | [V-r] | consigne dans la convocation (trame) |
| RG-X18 | Transport et ouverture des sujets nationaux, convocation des enseignants aux surveillances, centre d'examen privé | — | [NV] | procédures de **centre** en lot tardif (EX14), sur les consignes reçues de l'autorité |
| RG-X19 | **Pix** : certification en fin de cycle 4 et en fin de cycle terminal ; calendrier lycées 2026-2027 du **2 novembre 2026 au 5 mars 2027**, rattrapage du 8 au 12 mars ; **5 compétences au niveau 1 minimum** pour s'inscrire ; résultats des terminales importés dans Parcoursup ; sans incidence sur DNB ni bac | eduscol 5520 — https://eduscol.education.gouv.fr/5520/evaluer-developper-et-certifier-les-competences-numeriques | [V-r] | sessions de certification (§14) ; prérequis déclaré ; résultat au LSU/LSL (22, 23) |
| RG-X20 | **ASSR 1** (5ᵉ) et **ASSR 2** (3ᵉ) : 20 questions, 10/20 minimum ; l'ASSR 2 ou l'ASR est exigée pour le permis (nés depuis 1988, moins de 21 ans) | https://www.education.gouv.fr/l-education-la-securite-routiere-en-milieu-scolaire-2699 ; eduscol 4995 | [V-r] | session, résultats, **attestation conservée** et remise (§14) |
| RG-X21 | **Attestation scolaire « savoir nager en sécurité »** (ASNS, arrêté du 28 février 2022), délivrée par le chef d'établissement, versée au livret scolaire | https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000045269090/ ; eduscol 5709 | [V-r] | attestation (7) et livret (22) |
| RG-X22 | **PSC1** listé parmi les attestations du livret scolaire (arrêté du 31 décembre 2015) ; niveaux [NV] | https://www.legifrance.gouv.fr/loda/id/JORFTEXT000031742317/2018-08-23/ | [V-r] partiel | attestation ; niveau paramétré |
| RG-X23 | Cambridge, DELF scolaire, Voltaire, « gestes qui sauvent » | — | [NV] | certifications **libres** de l'établissement (référentiel), aucune règle codée |
| RG-X24 | **Résultats** : transmission à la presse ou à des organismes privés **seulement avec consentement exprès** (à l'inscription) ; opposition possible à la publication ; conservation jusqu'aux résultats de la session suivante (traitement national) | avis CNIL 2012-177 — https://www.legifrance.gouv.fr/cnil/id/CNILTEXT000027400981/ ; arrêté de 2013 (BO n° 23) — https://www.education.gouv.fr/bo/13/Hebdo23/MENE1303017A.htm | [V-r], **anciens**, à reconfirmer | ScolIA **ne publie aucun résultat** à l'extérieur ; liste interne de lauréats (cérémonie) avec **information et opposition** (R-EX29) |
| RG-X25 | Examens blancs, neutralisation des cours | — | [NV] (aucun texte) | **décision de l'établissement** (conseil pédagogique) ; aucun contrôle réglementaire codé |
| RG-X26 | Durée de conservation des copies, relevés, diplômes non retirés | — | [NV] | durées proposées au DPO (§17.4) |
| RG-X27 | **Données de santé** : l'établissement connaît la **décision** (mesures), pas le diagnostic (secret médical de l'avis) | déduit de RG-X14 | [R] (exigence de conception cohérente avec la procédure, pas un texte cité) | R-EX11, R-EX12 |
| RG-X28 | Règlement IA : **évaluer les acquis** = haut risque | 23 R-B16 | [V*] | l'IA ne corrige ni ne note (R-EX27) |

**Non vérifié et interdit de coder dessus** : RG-X04, X13, X15, X18, X23, X26 ; format Notanet ou Cyclades ; échéances 2027 d'inscription et de demande d'aménagement ; détail du décret fraude de 2026.

---

## 6. Concepts et vocabulaire [R]

| Terme | Sens |
|---|---|
| **Session** | ensemble d'épreuves sur une période : « brevet blanc de février », « DNB 2027 (organisation locale) », « Pix terminales 2026-2027 » ; nature `interne`, `nationale_locale`, `centre_national`, `certification` |
| **Gabarit** | modèle prêt d'une session (D-023) : brevet blanc DNB 2027, bac blanc (philosophie, spécialités), EAM blanche, grand oral blanc, oral du DNB, CCF EPS, épreuves communes, Pix, ASSR |
| **Épreuve** | une matière ou partie (écrit, oral, pratique) ; date, heure, durée **de base**, public, coefficients **informatifs** (interne) |
| **Candidat** | élève inscrit à une épreuve (ou candidat extérieur au centre) ; statut `inscrit`, `exempte`, `dispense`, `absent`, `present`, `abandon`, `exclu` |
| **Mesure d'aménagement** | une mesure accordée (tiers-temps, salle à part, secrétaire, lecteur, ordinateur, sujet adapté, pause, accès…) ; **source** (`decision_autorite`, `demande_en_cours`, `pap_pps`, `etablissement`) |
| **Salle d'épreuve** | salle (socle 5) affectée à une épreuve, avec capacité utile (une place sur deux, par exemple) et **heure de fin** propre (tiers-temps) |
| **Place** | candidat × salle × numéro de table |
| **Surveillance** | personne × salle × créneau ; rôle `responsable_salle`, `surveillant`, `secretaire`, `lecteur`, `appariteur`, `jury` |
| **Convocation** | document émis (7) et avis (8) à un candidat, à ses titulaires, à un surveillant |
| **Émargement** | présence du candidat constatée dans la salle (heure d'entrée, de sortie) |
| **Copie** | rendue, numérotée (anonymat), attribuée à un correcteur |
| **Sujet** | fichier confidentiel (7) avec version, auteur, variantes adaptées, heure d'ouverture |
| **Incident** | retard, malaise, fraude présumée, sortie, problème matériel ; procès-verbal |
| **Inscription nationale** | préparation de l'inscription d'un élève à un examen national (série, spécialités, options, langues), état dans Cyclades (déclaré), confirmation signée |
| **Résultat officiel** | décision du jury rapatriée (admis, ajourné, mention, notes si communiquées) |
| **Certification** | Pix, ASSR, ASNS, PSC1, autres ; passage et obtention, attestation |

---

## 7. Sessions et épreuves

### 7.1 Créer une session [R]
- Depuis un **gabarit** (§6) : la session est préremplie (épreuves, durées, coefficients, public type) ; la direction ajuste dates, salles disponibles, publics (niveaux, classes, groupes, élèves).
- Une session porte : nature, titre, période, responsable, **règle de neutralisation** des cours (§7.3), **règle de notes** (épreuve interne : crée-t-elle une évaluation commune au 21 ? compte-t-elle dans la moyenne ? Q-EX6), mode des convocations, salles mobilisables.
- États : `brouillon` → `planifiee` (dates fixées) → `organisee` (plan complet contrôlé) → `publiee` (convocations parties) → `en_cours` → `terminee` → `corrigee` → `close` (+ `annulee`).

### 7.2 Épreuves [R]
Chaque épreuve : matière (5), nature (écrit, oral, pratique, QCM, numérique), date et heure de début, **durée de base**, matériel autorisé (texte encadré, préréglé par gabarit : « calculatrice interdite » pour l'EAM), public, variantes de sujet (parcours pour l'EAM), **ordre de passage** pour les oraux.

### 7.3 Neutralisation des cours (socle 10) [R]
- Le 38 demande au 10 les **séances touchées** : séances des candidats (cours **neutralisés** pour le public concerné), séances des **surveillants** (cours non assurés → remplacement ou annulation, socle 15), séances dans les **salles réquisitionnées** (déplacées).
- Proposition présentée à la direction (« 14 cours neutralisés, 6 cours à déplacer, 3 professeurs indisponibles ») ; validation ★ ; exceptions créées par le 10 ; avis aux élèves et familles par le 8 (« pas de cours de 8 h à 12 h, brevet blanc »).
- Un **examen national** prime sur une réservation ordinaire (R-AG21 du 19).

### 7.4 Charge des élèves (socle 20) [R]
Les épreuves sont exposées au 20 (charge) : alerte aux professeurs qui ajoutent une évaluation ces jours-là (CC-56, CDT-254).

---

## 8. Salles et placement

### 8.1 Salles [R]
- Salles mobilisables (5) avec **capacité d'examen** (préréglée à la moitié de la capacité ordinaire [H], modifiable), équipements (prises, ordinateurs, accès PMR, rez-de-chaussée), **salles à part** (petite capacité pour mesures).
- Réservation au 19 à la planification ; conflit signalé, jamais écrasé.

### 8.2 Placement [R]
- **Proposition automatique** (déterministe, puis l'IA peut expliquer) : contraintes dures (capacité, **mesures** : salle à part, accès, ordinateur, secrétaire, heure de fin), puis souhaits (ordre alphabétique, mélange des classes, **séparation de voisins** déclarés, numéros de table) ; résultat expliqué (« Inès en salle 12 : accès PMR et tiers-temps »).
- Correction à la main, verrouillage d'un placement, recalcul du reste.
- **Contrôle** avant publication (R-EX8) : tout candidat placé une seule fois par créneau ; aucune salle au-delà de la capacité ; toute mesure servie ; heure de fin de salle ≥ heure de fin du dernier candidat (tiers-temps).
- Liste d'affichage par salle (nom, numéro de table) et **plan de salle** (grille) ; affichage public **minimisé** (nom et salle, jamais la mesure, R-EX12).

### 8.3 Anonymat [R]
Option par épreuve : **numéro d'anonymat** attribué au placement, imprimé sur l'étiquette de copie ; correspondance visible du seul responsable des examens jusqu'à la levée (§11.3).

---

## 9. Sujets confidentiels

- **Dépôt** par l'auteur (professeur ou équipe) dans l'espace de la session : fichier (7, objet immuable E5), version, **variantes** (parcours EAM, sujet adapté : agrandi, police adaptée, braille [H]) ; **co-auteurs** désignés.
- **Accès** : auteurs, responsable des examens, direction ; pas les autres professeurs de la matière par défaut ; **chaque ouverture et chaque téléchargement journalisés** (qui, quand, quelle version) ; **aucune prévisualisation par l'IA** (R-EX14).
- **Validation** par le coordonnateur de matière ou la direction ; **gel** à J-2 (paramètre) : toute modification après le gel exige une nouvelle validation et rappelle de réimprimer.
- **Impression** : le reprographe désigné reçoit un **lot d'impression** (nombre d'exemplaires par salle et par variante, calculé depuis le placement et les mesures) ; trace « imprimé le … par … » ; les exemplaires excédentaires sont comptés.
- **Ouverture** : rien n'est accessible aux élèves dans ScolIA avant l'heure de fin de l'épreuve (y compris le cahier de textes, socle 20) ; publication du sujet et du corrigé **après** l'épreuve, au choix de l'auteur.
- **Fuite présumée** : geste « sujet compromis » → alerte direction, journal des accès joint, remplacement par une variante de secours si déposée.
- **Sujets nationaux** (centre d'examen) : **hors ScolIA** (reçus sous pli) ; le 38 trace seulement la **réception**, le **stockage** (coffre) et l'**ouverture** des enveloppes (heure, témoins) (EX14).

---

## 10. Aménagements d'examen

### 10.1 Qui possède quoi [R]
- **35** : plan de l'élève (PAP, PPS, PPRE), aménagements **pédagogiques**, référent ; il **ne** porte pas la décision d'examen.
- **38** : la **demande** (suivi de la démarche de la famille), la **décision** de l'autorité (mesures accordées, session, date, autorité, recours), son **application** aux épreuves.
- **34** : la dispense médicale d'EPS (conséquence seulement) et les consignes minimales (« trousse d'urgence »).

### 10.2 Campagne de demande [R]
- Chaque année, le 38 liste les élèves **à qui proposer** une demande : élèves avec PAP ou PPS (drapeau du 35), élèves déjà aménagés l'an passé, élèves signalés par l'équipe ; **sans aucun diagnostic**.
- Information des titulaires (ou de l'élève majeur) : procédure, **date limite** (= date limite d'inscription de l'examen, R-EX10), coordonnées du médecin désigné (donnée de l'établissement) ; rappel J-15, J-3.
- **Aucune pièce médicale** n'est recueillie par ScolIA (R-EX11) : le dossier va au médecin désigné ; l'établissement peut fournir une **attestation pédagogique** (avis de l'équipe, document émis par le 7) si la procédure la prévoit [NV].
- Suivi : « demande déposée le … » (déclaré par la famille ou le référent), « décision reçue le … ».

### 10.3 Décision [R]
- Saisie par le secrétariat des examens ou le référent depuis la **notification** de l'autorité (document joint au 7, accès restreint) : session ou examen, mesures accordées (référentiel), précisions (tiers-temps **sur toutes les épreuves** ou certaines ; secrétaire **lecteur et/ou scripteur** ; ordinateur **sans correcteur** ; salle à part), date, autorité, voies de recours ; mesures refusées (pour mémoire, visibles de la direction et du référent seulement).
- **Contrôle** : majoration ≤ 1/3 (R-EX9) ; mesure inconnue du référentiel → ajout motivé.
- **Recours** : date de dépôt, issue.

### 10.4 Application [R]
- **Examens nationaux organisés localement** (oral du DNB, CCF, centre) : mesures de la décision **seules**.
- **Épreuves internes** : par défaut, les mesures **décidées pour l'examen national de l'année** s'appliquent ; si la décision n'est pas encore rendue, les mesures **demandées** s'appliquent **à titre provisoire** (marquées comme telles) ; à défaut, les **aménagements pédagogiques d'évaluation** du PAP/PPS (35) s'appliquent ; la direction peut ajouter une mesure d'établissement motivée (Q-EX3, R-EX13).
- Les mesures **pilotent** : salle (à part, PMR), durée et heure de fin de salle, **personne** (secrétaire, lecteur : AESH de l'élève si le 35 l'autorise, sinon un personnel désigné), **sujet adapté** (variante), matériel (ordinateur de l'établissement, sans réseau [H]), pauses, place (près de la porte).
- **Ce que voit chacun** (R-EX12) : surveillant de la salle → pour **ses** candidats : « tiers-temps », « secrétaire », « ordinateur », « pauses autorisées » ; jamais le motif, jamais un document ; professeurs → les mesures de leurs élèves (35) ; élève et famille → leurs mesures et leur application (convocation) ; affichage public → rien.

---

## 11. Le jour de l'épreuve

### 11.1 Convocations [R]
- **Candidat** : document personnel (7) et avis (8, application mobile) : épreuve, date, heure d'arrivée, salle, numéro de table, matériel, **mesures appliquées** (pour lui), consignes (téléphones) ; **titulaires** de l'élève mineur ; l'élève majeur décide de la copie à ses parents (4).
- **Surveillant** : ses créneaux, salles, rôle, candidats aménagés (mesure seulement), heure de fin propre ; ajout à son agenda (10, 19).
- **Secrétaire, lecteur, AESH** : son candidat, salle, durée.
- Modification après publication : nouvelle convocation « modifiée le … », ancienne retirée, avis ciblé (R-EX17).

### 11.2 Émargement et appel [R]
- Tablette ou téléphone du responsable de salle (mobile, **hors ligne** toléré) : liste de la salle, présent / absent / retard (heure), sortie temporaire (heure aller-retour), sortie définitive (heure, copie rendue) ; **abandon**.
- Les candidats sont marqués **`en_examen`** au 12 (pas d'absence de cours) ; un **absent** à l'épreuve est signalé à la vie scolaire **dans les 15 minutes** (R-EX18) : appel aux familles par le 12 et le 8.
- **Retardataire** : règle d'admission de l'établissement (préréglée : admis jusqu'à 1 h après le début sans temps supplémentaire pour l'interne [H]) ; pour le national : consignes de l'autorité [NV].

### 11.3 Copies [R]
- Ramassage par salle (comptage : copies rendues = présents) ; anonymat (§8.3) ; **répartition aux correcteurs** (équilibre du nombre, pas de correction de ses propres élèves si l'établissement le choisit, double correction possible) ; suivi « corrigée » ; **levée de l'anonymat** par le responsable après saisie de toutes les notes.
- Copies **numérisées** (facultatif, Q-EX8) : objet (7), visible du correcteur puis de l'élève après publication.

### 11.4 Notes [R]
- **Épreuve interne** : l'épreuve crée au 21 une **évaluation commune** et une **évaluation par cours** concerné (21 §4 point 4, CE-46) ; les correcteurs saisissent dans le 21 (par numéro d'anonymat si actif, le 21 reçoit la correspondance à la levée) ; statuts « absent », « dispensé », « exclu » repris du 38 ; publication au 21 (règle du 21).
- **Épreuve d'examen national organisée localement** (oral DNB, CCF) : note **au 38** (`exa_resultat_epreuve`), grille, jury, **verrou** après validation, report vers l'application nationale (bloqué D-2 : report manuel tracé, R-EX22).
- **Statistiques** : moyenne, répartition, par classe et par correcteur (harmonisation) ; **écarts** contrôle continu / épreuve (EVA-216 du 21).

### 11.5 Incidents [R]
Malaise (infirmerie : consigne minimale 34), sortie, problème matériel, sujet erroné (erratum diffusé aux salles), alerte (PPMS, socle 14 : interruption de l'épreuve, reprise, temps compensé).

### 11.6 Fraude [R]
- **Épreuve interne** : incident « fraude présumée » (faits, pièces saisies photographiées, témoins) → **direction** ; la note est **suspendue** au 21 ; la direction décide : note maintenue, épreuve annulée pour l'élève, **procédure disciplinaire** ouverte au **13** (jamais automatique, R-EX24).
- **Examen national tenu dans l'établissement** : **trame de procès-verbal** (RG-X16) remplie par le surveillant responsable, **contresignée** (signatures 7) par les autres surveillants et l'auteur (refus mentionné), transmise par le chef de centre au recteur (hors ScolIA) ; trace de la transmission.

---

## 12. Épreuves organisées par l'établissement pour un examen national

### 12.1 Oral du DNB [V-r RG-X02, R]
- Sujet ou projet choisi par l'élève (déclaré, avec date limite), éventuellement **en groupe** ; jurys (deux personnels [H, NV]) ; **planning des passages** (proposition : jurys × salles × créneaux, sans conflit d'EDT, mesures respectées) ; convocations ; grille d'évaluation (critères de l'établissement, sur le barème national [NV]) ; note sur la grille ; verrou ; **oral blanc** possible avec le même moteur.
- Élève absent : rattrapage (créneau) ou absence justifiée (12).

### 12.2 Grand oral blanc, oraux blancs [R]
Questions déposées par l'élève, professeurs de spécialité, jurys, planning, grille ; pas de règle nationale codée (RG-X13).

### 12.3 CCF (EPS, voie professionnelle) [V-r RG-X11, RG-X12, R]
- **Situations d'évaluation** par matière ou référentiel (22 pour les compétences du bac pro) : date, élèves, évaluateurs, grille ; **inaptitude** : dispense médicale (34, conséquence) → épreuve adaptée ou dispense selon les règles de l'examen [NV : saisies par l'établissement] ; **rattrapage**.
- **PV** de CCF par situation et par élève, signé (7) par les évaluateurs ; verrou ; archivage ; report vers l'application nationale (manuel tracé, bloqué D-2).
- **Chef-d'œuvre** : notes consignées durant l'élaboration (au 21 ou ici) + oral (10 ou 15 min, deux enseignants) ; calcul 50/50 **affiché** (R-EX21).
- PFMP (39) : évaluations rattachées, pas dupliquées.

### 12.4 Simulation du DNB [R]
Calcul **indicatif** des points (contrôle continu du 21 + épreuves blanches) pour l'équipe : « estimation non officielle » partout (R-EX20) ; visible des familles **seulement** si l'établissement l'active (S38-04, désactivé par défaut).

---

## 13. Examens nationaux : inscriptions, convocations, résultats

### 13.1 Préparer les inscriptions [R]
- Pour chaque candidat (élève de 3ᵉ, 1ʳᵉ pour les épreuves anticipées, terminale, voie pro) : examen, série ou voie, **spécialités** et options (lues au 5 et au 23 : spécialités choisies), langues vivantes, EPS (dispense ?), **aménagements** (décision ou demande en cours), données d'identité à vérifier (nom d'usage, prénoms, date et lieu de naissance, INE) ; **liste de contrôle** pour le secrétariat avant saisie dans Cyclades (« 4 élèves sans INE », « 2 options incohérentes avec la classe »).
- Geste « saisi dans Cyclades le … » (déclaré, horodaté) (R-EX22).

### 13.2 Confirmation d'inscription signée [V-r RG-X09, R]
- L'établissement dépose dans ScolIA (ou produit) la **confirmation** de chaque candidat (PDF reçu de Cyclades) ; signature **électronique simple** par le candidat **et** un titulaire si mineur, ou signature papier scannée ; suivi « signée / non signée », **relances**, **date limite** (paramètre) ; dépôt dans Cyclades tracé.
- Élève majeur : il signe seul. Parents séparés : **un** titulaire suffit [NV : à confirmer, Q-EX10] ; l'autre est **informé**.
- Corrections demandées par la famille (orthographe du nom, option) : demande tracée, traitée par le secrétariat avant la date de modification.

### 13.3 Aménagements (lien §10)
La liste des candidats aménagés et l'état des demandes est jointe à la préparation ; alerte « demande non déposée à J-15 de la date limite ».

### 13.4 Convocations nationales [R]
Rappel aux candidats et titulaires à la date de mise à disposition (RG-X10) ; le PDF de convocation (s'il est récupéré par l'établissement) peut être joint au coffre de l'élève (7).

### 13.5 Résultats officiels [R]
- **Rapatriés** par le secrétariat (saisie ou **tableur**, FX-09 du 9 ; import officiel si un format existe, bloqué) : examen, session, décision (`admis`, `ajourne`, `admis_apres_second_groupe`, `absent`…), mention, notes si communiquées, date.
- **Publication** dans ScolIA à l'élève et à ses titulaires **après** la publication nationale (paramètre) (R-EX25).
- **Second groupe** (bac) : liste des élèves concernés, information, accompagnement (oraux de rattrapage : hors établissement [NV]).
- **Relevés de notes** et **attestations de réussite** reçus : déposés au coffre (7).
- **Diplômes** : réception des diplômes (lot), **remise** (en main propre, à la cérémonie, par courrier recommandé, à un tiers mandaté) tracée et signée (7) ; **non retirés** listés ; conservation (§17.4).
- Résultats vers le **dossier** (2), le **37** (admission, devenir), le **47** (taux de réussite, agrégés).

---

## 14. Certifications et attestations

| Certification | Ce que fait le 38 | Liens |
|---|---|---|
| **Pix** (3ᵉ, terminale) | sessions de certification (salle informatique, créneaux, surveillants, candidats éligibles : prérequis « 5 compétences niveau 1 » **déclaré** [V-r]), convocations, émargement, résultat (niveau, score) saisi ou importé (tableur), rattrapage | 22, 23 (LSU, LSL), 37 (Parcoursup) ; plateforme Pix extérieure : aucune interface supposée [NV] |
| **ASSR 1, ASSR 2** | session, passation (salle informatique), résultat (réussite ≥ 10/20 [V-r]), **attestation** conservée (7) et **remise** tracée ; duplicata sur demande d'un ancien élève (droit au permis) | 7, 22 |
| **ASNS** (savoir nager) | validation par l'enseignant d'EPS (test), **délivrance par le chef d'établissement**, versée au livret (22) | 22, 42 (activités nautiques) |
| **PSC1** | formation (organisme agréé), participants, attestation reçue, versée | 22, 6 (formateurs) |
| **Autres** (DELF scolaire, Cambridge, Voltaire…) | certifications **libres** (référentiel de l'établissement) : sessions, inscriptions, résultats, attestations | [NV] : aucune règle codée |

---

## 15. Centre d'examen (examens nationaux reçus par l'établissement)
Lot tardif (EX14), seulement si l'établissement est centre (F38-2) : **candidats extérieurs** (liste reçue, sans compte ScolIA, données minimales, durée courte), salles et placements, **surveillants convoqués par l'autorité** (liste reçue ; personnels de l'établissement : absence de cours → 15 et 10), **réception et coffre des sujets** (heure, témoins), **ouverture des enveloppes** (procès-verbal), émargement, incidents et fraude (PV), **remise des copies** (bordereau), restitution des salles. Consignes de l'autorité chargées comme **trames** (D-023), jamais codées de mémoire (RG-X18 [NV]).

---

## 16. Intelligence artificielle (contrat IA du socle)

| Capacité | Lecture / commande | Ce que fait l'IA | Garde-fous |
|---|---|---|---|
| Proposer un placement | commande simulée | depuis le moteur déterministe ; explique les choix | contraintes dures non négociables (R-EX8) ; validation humaine |
| Proposer les surveillances | commande simulée | équité (heures cumulées, quotités), EDT, indisponibilités | jamais une contrainte d'un collègue nommée (D-033 B) ; validation |
| Planifier les oraux | commande simulée | jurys × créneaux × mesures | validation |
| Contrôler un plan | lecture | « 3 problèmes : … » | les contrôles sont **déterministes** ; l'IA reformule |
| Brouillon de convocation ou d'information | commande simulée | texte depuis la trame | relu ; envoi confirmé (CLAUDE.md) |
| Expliquer un aménagement **au surveillant** | lecture | « tiers-temps : votre salle ferme à 11 h 20 » | jamais le motif |
| **Interdits** | — | lire un **sujet** avant l'heure ; corriger, noter, proposer une note ; décider ou suggérer un aménagement ; qualifier une fraude ; lire une décision d'aménagement au-delà des mesures ; toute donnée de santé | R-EX14, R-EX27, RG-X28 |

---

## 17. Droits, minimisation, conservation

### 17.1 Catalogue d'actions (D-017) [R]
| Droit | Par défaut |
|---|---|
| `examens.session_gerer` | direction, responsable des examens (mission), secrétariat des examens |
| `examens.plan_gerer` (salles, placement, surveillances) | responsable des examens, secrétariat des examens, direction |
| `examens.publier` ★ | direction, responsable des examens |
| `examens.sujet_deposer` | professeurs (leurs épreuves), co-auteurs |
| `examens.sujet_lire` | auteurs, responsable, direction, reprographe (lot d'impression seulement) |
| `examens.emarger` | surveillants de la salle |
| `examens.incident_declarer` | surveillants, CPE, direction |
| `examens.copies_repartir` | responsable des examens |
| `examens.anonymat_lever` | responsable des examens |
| `examens.oral_noter`, `examens.ccf_noter` | jurys, évaluateurs désignés |
| `amenagements.gerer` | secrétariat des examens, référent (35), direction |
| `amenagements.mesures_lire` | professeurs des élèves (35), surveillants (leurs candidats, mesure seule) |
| `examens.inscriptions_preparer` | secrétariat, direction |
| `examens.resultats_saisir` | secrétariat, direction |
| `examens.diplomes_remettre` | secrétariat, direction |
| `certifications.gerer` | responsable (Pix : référent numérique), professeurs d'EPS (ASNS), direction |
| `examens.ceremonie_gerer` | direction, secrétariat |
| `exports.officiels` (existant) | secrétariat, direction (D-022) |

### 17.2 Ce que voit chacun [R]
| Qui | Voit |
|---|---|
| Élève | ses convocations, salle, place, mesures appliquées, résultats publiés, attestations |
| Titulaire | idem pour l'élève mineur ; élève majeur : ce qu'il partage (copie des convocations par défaut **oui**, S38-05) |
| Surveillant | ses salles, candidats (nom, numéro de table, mesure), heures de fin ; pas les notes, pas les décisions |
| Professeur | ses épreuves, ses copies, les mesures de ses élèves (35), les statistiques de ses épreuves |
| Responsable des examens | toute la session ; décisions d'aménagement (mesures et documents) |
| Direction | tout |
| Reprographe | nombres d'exemplaires et fichiers du lot d'impression, pendant la fenêtre d'impression |
| Affichage public | nom, salle, numéro de table ; rien d'autre |

### 17.3 Garde-fous [R]
- Journal des accès aux **sujets** et aux **décisions d'aménagement** (E7) ; documents de décision en **domaine restreint** (comme les pièces sensibles du 7).
- Fenêtre d'accès du reprographe limitée dans le temps.
- Aucune liste d'aménagés exportée sans motif et sans administratif (D-022).
- RLS stricte (E13).

### 17.4 Conservation (à valider avec le DPO et l'archiviste) [R]
| Donnée | Durée proposée |
|---|---|
| Sessions, plans, placements, émargements (interne) | année + 1 |
| Copies numérisées (interne) | année + 1 |
| Sujets | 5 ans (banque de sujets de l'établissement), sur décision de l'auteur |
| Décisions d'aménagement | durée de validité de la décision + 1 an ; documents de notification : idem |
| Notes d'oral DNB, PV de CCF | 2 ans après la session (recours) [H] |
| Procès-verbaux de fraude | 5 ans [H] |
| Inscriptions et confirmations signées | 2 ans après la session |
| Résultats officiels | dossier scolaire (DUA, 23 R-B14) |
| Registre de remise des diplômes, diplômes non retirés | 50 ans pour le registre [H] ; diplômes non retirés : selon les instructions de l'autorité [NV] |
| Attestations (ASSR, ASNS, PSC1) | dossier scolaire ; ASSR : tant que l'ancien élève peut en avoir besoin (permis) [H] |
| Cérémonie (réponses) | 3 mois après l'événement |

---

## 18. Notifications (socle 8)
Natures : `examens.calendrier_publie` (élèves, familles, personnels), `.cours_neutralises` (élèves, familles concernés), `.convocation` (candidat, titulaires), `.convocation_modifiee`, `.rappel_veille`, `.surveillance` (personnel), `.surveillance_modifiee`, `.absent_epreuve` (vie scolaire, titulaires), `.sujet_a_deposer` (auteur), `.sujet_gele`, `.copies_a_corriger` (correcteur), `amenagements.campagne` (titulaires, élève majeur), `.rappel_demande`, `.decision_enregistree` (titulaires, référent), `inscriptions.confirmation_a_signer`, `.rappel_signature`, `resultats.publies`, `diplomes.a_retirer`, `ceremonie.invitation`, `certifications.convocation`, `.resultat`. Parent sans exercice : `resultats.publies` seulement. « Ne pas solliciter » (D-028) respecté.

---

## 19. Cas limites (catalogue)

| # | Situation | Traitement [R] |
|---|---|---|
| CEX-01 | Élève en tiers-temps dans une salle qui ferme à l'heure normale | contrôle bloquant (R-EX8) |
| CEX-02 | Décision d'aménagement reçue la veille | mesures ajoutées ; plan recalculé pour cet élève ; convocations ciblées |
| CEX-03 | Décision qui refuse une mesure demandée | épreuves internes : la mesure provisoire tombe à la date de la décision ; information |
| CEX-04 | Recours en cours | décision initiale appliquée [H] ; issue tracée |
| CEX-05 | Majoration demandée > 1/3 | refus de saisie (R-EX9) |
| CEX-06 | Secrétaire-lecteur absent le jour J | alerte ; remplaçant désigné ; à défaut, report de l'épreuve pour l'élève (interne) |
| CEX-07 | AESH de l'élève proposé comme secrétaire | autorisé si le 35 le permet ; pour le national [NV] : décision de l'autorité |
| CEX-08 | Élève handicapé sans PAP ni PPS qui demande un aménagement | campagne ouverte à la demande ; procédure normale |
| CEX-09 | Élève qui refuse son tiers-temps le jour J | noté par le surveillant (« mesure non utilisée ») ; jamais imposé |
| CEX-10 | Deux élèves aménagés différemment dans une même salle à part | heure de fin = la plus tardive ; deux secrétaires → deux salles |
| CEX-11 | Professeur qui surveille pendant son cours d'une classe non concernée | proposition l'évite ; sinon cours au 15 (remplacement) ou neutralisé |
| CEX-12 | Professeur à temps partiel ou absent le jour de surveillance | indisponibilité lue au 6 et au 15 ; recalcul |
| CEX-13 | Surveillant malade le matin | remplacement proposé parmi les disponibles ; avis |
| CEX-14 | Salle inondée le jour J | salle de repli ; nouvelles places ; affichage modifié ; avis |
| CEX-15 | Alerte PPMS pendant l'épreuve | interruption, heure notée, temps compensé, reprise (14) |
| CEX-16 | Erratum sur un sujet | diffusion à toutes les salles (mobile), heure tracée |
| CEX-17 | Sujet modifié après le gel | nouvelle validation, rappel de réimpression, ancien lot invalidé |
| CEX-18 | Fuite présumée d'un sujet | geste « compromis », journal des accès, variante de secours |
| CEX-19 | Élève absent à l'épreuve | `absent` au 38, alerte vie scolaire en 15 min, statut « absent » au 21 (règle du 21) |
| CEX-20 | Élève malade pendant l'épreuve | sortie à l'infirmerie, copie rendue, statut « abandon » ou reprise |
| CEX-21 | Retardataire de 1 h 30 | règle d'admission (S38-02) ; refusé : absent justifiable |
| CEX-22 | Téléphone qui sonne | incident ; fraude présumée selon la règle de l'établissement (interne) ou PV (national) |
| CEX-23 | Fraude présumée à l'interne | note suspendue, direction décide, procédure au 13 éventuelle (R-EX24) |
| CEX-24 | Élève exclu temporairement (13) un jour de brevet blanc | l'exclusion prime [H] ; épreuve de rattrapage ou absence ; décision de la direction |
| CEX-25 | Élève en sortie scolaire (42) un jour d'épreuve | conflit signalé à la planification, jamais résolu seul |
| CEX-26 | Élève en stage (39) ou PFMP | conflit signalé ; dispense de l'épreuve blanche ou rattrapage |
| CEX-27 | Élève interne (41) | convocation aussi au responsable d'internat si épreuve tôt [H] |
| CEX-28 | Élève qui change de classe la veille | placement recalculé ; candidats calculés à la date de l'épreuve (R-EX4) |
| CEX-29 | Élève arrivé en cours d'année | ajouté au public ; aménagement absent : campagne individuelle |
| CEX-30 | Élève parti | retiré, convocation annulée |
| CEX-31 | Groupe de LV2 : sujet allemand et espagnol | variantes par groupe (D-022) ; correcteurs par groupe |
| CEX-32 | EAM : trois parcours dans une même classe | variantes de sujet par parcours lu au 5 ; comptage des exemplaires |
| CEX-33 | Correcteur qui corrige ses propres élèves | interdit si l'établissement le choisit (S38-03) |
| CEX-34 | Copie perdue | statut « copie non retrouvée » ; décision direction (note neutre, rattrapage) ; jamais un 0 automatique |
| CEX-35 | Note saisie sur un numéro d'anonymat erroné | correction avant levée ; après levée, correction au 21 tracée |
| CEX-36 | Famille qui conteste une note d'examen blanc | règle du 21 (réclamation) |
| CEX-37 | Confirmation d'inscription non signée à J-2 | relance ciblée, appel, rendez-vous de signature |
| CEX-38 | Parents séparés : l'un signe, l'autre conteste l'option | Q-EX10 ; information des deux ; arbitrage par la direction selon le 4 |
| CEX-39 | Élève majeur qui ne veut pas que ses parents voient ses résultats | publication à lui seul ; titulaires non informés sauf partage |
| CEX-40 | Erreur d'état civil sur la confirmation | demande de correction tracée ; pièce du dossier (4) |
| CEX-41 | Résultat officiel importé avec un INE inconnu | ligne rejetée (9), jamais un élève créé |
| CEX-42 | Résultat publié en ScolIA avant la publication nationale | impossible : date paramétrée (R-EX25) |
| CEX-43 | Famille qui s'oppose à figurer dans la liste des lauréats (cérémonie) | exclue de la liste, aucune invitation publique (R-EX29) |
| CEX-44 | Diplôme remis à un tiers | mandat signé (7) ; remise tracée |
| CEX-45 | Diplôme non retiré depuis 10 ans | liste ; conservation selon l'autorité [NV] |
| CEX-46 | Ancien élève qui demande un duplicata d'ASSR 2 | attestation reproduite depuis l'archive (7) ; identité vérifiée |
| CEX-47 | Pix : élève qui n'a pas le prérequis | non inscrit ; liste au référent ; rattrapage |
| CEX-48 | Pix : session interrompue (panne réseau) | incident ; nouvelle session ; candidats reconvoqués |
| CEX-49 | Inaptitude d'EPS totale à l'année (34) | dispense de CCF selon les règles de l'examen [NV] saisies par l'établissement ; aucun motif médical visible |
| CEX-50 | Inaptitude partielle le jour du CCF | épreuve adaptée ou rattrapage ; certificat **au 34**, conséquence seule au 38 |
| CEX-51 | Oral du DNB en groupe de trois | planning commun, notes individuelles |
| CEX-52 | Élève qui change de sujet d'oral après la date limite | accord de la direction tracé |
| CEX-53 | Centre d'examen : candidat extérieur aménagé | décision reçue de l'autorité, mesures appliquées ; données supprimées après la session |
| CEX-54 | Deux établissements du même organisme passent un brevet blanc commun | aucune donnée partagée (D-013) ; sujets partagés par export manuel |

---

## 20. Actions métier (EXA-100 et suivantes)

Légende : DIR, ADJ (adjoint), REX (responsable des examens, mission), SEX (secrétariat des examens), PRF, PEPS (professeur d'EPS), PLP (professeur de lycée pro), SUR (surveillant), JUR (jury, évaluateur), REP (reprographe), CPE, INF, AESH, REF35 (référent handicap), ELV, ELM, PAR, SYS. ★ confirmation renforcée.

### 20.1 Paramètres et gabarits
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| EXA-100 | Désigner le responsable des examens (mission, socle 6) | DIR | — | — |
| EXA-101 | Paramétrer les dates nationales de l'année (inscriptions, LSL, convocations, résultats, épreuves) | SEX | R-EX2 | `examens.calendrier_maj` |
| EXA-102 | Charger le référentiel des mesures d'aménagement | SEX | RG-X15 | — |
| EXA-103 | Paramétrer la capacité d'examen des salles | REX | 5 | — |
| EXA-104 | Paramétrer les règles de l'établissement (retardataires, correction de ses élèves, anonymat par défaut) | DIR | S38-02, 03 | — |
| EXA-105 | Gérer les trames (convocations, PV, attestations, information) — 3 ou 4 par type | DIR | D-023 | — |
| EXA-106 | Gérer les certifications libres (référentiel) | DIR | RG-X23 | — |
| EXA-110 | Choisir un gabarit de session | REX | — | — |
| EXA-111 | Gabarit « brevet blanc DNB 2027 » | SYS | RG-X02 | — |
| EXA-112 | Gabarit « bac blanc » (philosophie, spécialités, français) | SYS | — | — |
| EXA-113 | Gabarit « EAM blanche » (3 parcours, QCM, sans calculatrice) | SYS | RG-X08 | — |
| EXA-114 | Gabarit « oral du DNB », « grand oral blanc », « oral de langues » | SYS | — | — |
| EXA-115 | Gabarit « CCF EPS », « CCF voie pro », « chef-d'œuvre » | SYS | RG-X11 | — |
| EXA-116 | Gabarit « épreuves communes » | SYS | 21 CE-46 | — |
| EXA-117 | Gabarits Pix, ASSR, ASNS, PSC1 | SYS | §14 | — |

### 20.2 Sessions et épreuves
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| EXA-120 | Créer une session (nature, période, publics, règles) | REX, DIR | §7.1 | `examens.session_creee` |
| EXA-121 | Dupliquer la session de l'an passé | REX | dates à valider | `examens.session_creee` |
| EXA-122 | Ajouter, modifier une épreuve (matière, date, durée, matériel, variantes) | REX | — | `examens.epreuve_maj` |
| EXA-123 | Définir le public (niveaux, classes, groupes, élèves) | REX | R-EX4 | — |
| EXA-124 | Exempter un candidat (motif) | DIR ★ | RG-X03 | `examens.exemption` |
| EXA-125 | Choisir la règle de notes (évaluation commune au 21, compte ou non) | DIR, REX | Q-EX6 | — |
| EXA-126 | Calculer les séances touchées (10) | SYS | §7.3 | — |
| EXA-127 | Valider la neutralisation et les déplacements | DIR ★ | — | `edt.exception` (10) |
| EXA-128 | Annoncer les cours neutralisés aux élèves et familles | SYS | 8 | `examens.cours_neutralises` |
| EXA-129 | Publier le calendrier des épreuves (élèves, familles, personnels) | REX | — | `examens.calendrier_publie` |
| EXA-130 | Annuler, reporter une épreuve (avis ciblés, EDT rétabli) | DIR ★ | — | `examens.epreuve_annulee` |
| EXA-131 | Exposer les épreuves à la charge des élèves (20) | SYS | CC-56 | — |
| EXA-132 | Voir les conflits (sorties, stages, exclusions, conseils) | REX | CEX-24 à 26 | — |
| EXA-133 | Contrôler le plan de la session (« tout est prêt » ou liste) | SYS, REX | R-EX8 | — |

### 20.3 Salles, placement, surveillances
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| EXA-140 | Réserver les salles (19) | REX | R-AG21 | — |
| EXA-141 | Ajouter une salle à part (mesures) | REX | — | — |
| EXA-142 | Proposer le placement | SYS | §8.2 | — |
| EXA-143 | Ajuster, verrouiller une place | REX | — | — |
| EXA-144 | Déclarer des voisins à séparer | REX, CPE | — | — |
| EXA-145 | Attribuer les numéros d'anonymat | SYS | §8.3 | — |
| EXA-146 | Produire listes d'affichage et plans de salle | REX | R-EX12 | — |
| EXA-147 | Changer un candidat de salle le jour J | REX | — | `examens.placement_maj` |
| EXA-150 | Recenser les disponibilités des surveillants (EDT, quotité, absences) | SYS | 6, 10, 15 | — |
| EXA-151 | Proposer les surveillances (équité, EDT) | SYS | R-EX15 | — |
| EXA-152 | Ajuster, échanger des surveillances (demande d'échange entre collègues) | REX, PRF | — | — |
| EXA-153 | Désigner le responsable de chaque salle | REX | — | — |
| EXA-154 | Désigner secrétaires, lecteurs, AESH par candidat | REX, REF35 | R-EX13 | — |
| EXA-155 | Voir ses surveillances (agenda, mobile) | PRF, SUR | — | — |
| EXA-156 | Déclarer une indisponibilité tardive | PRF | — | — |
| EXA-157 | Remplacer un surveillant le jour J | REX | CEX-13 | `examens.surveillance_maj` |
| EXA-158 | Bilan des heures de surveillance par personnel (équité, éventuelle rémunération) | DIR | F38-4 | — |

### 20.4 Sujets
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| EXA-170 | Demander le sujet d'une épreuve (échéance) | REX | — | `examens.sujet_a_deposer` |
| EXA-171 | Déposer un sujet, une variante, un corrigé | PRF | §9 | `examens.sujet_depose` |
| EXA-172 | Désigner des co-auteurs | PRF | — | — |
| EXA-173 | Valider un sujet | coordonnateur, DIR | — | `examens.sujet_valide` |
| EXA-174 | Geler les sujets (J-2) | SYS | R-EX14 | `examens.sujet_gele` |
| EXA-175 | Calculer le lot d'impression (par salle, variante, mesure) | SYS | — | — |
| EXA-176 | Imprimer le lot (fenêtre limitée) et le déclarer imprimé | REP | journal | `examens.sujet_imprime` |
| EXA-177 | Déclarer un sujet compromis | DIR ★ | CEX-18 | `examens.sujet_compromis` |
| EXA-178 | Diffuser un erratum aux salles | REX | CEX-16 | — |
| EXA-179 | Publier sujet et corrigé après l'épreuve | auteur | R-EX14 | — |
| EXA-180 | Banque de sujets de l'établissement (consulter les anciens) | PRF | droits | — |
| EXA-181 | Journal des accès à un sujet | DIR, REX | E7 | — |

### 20.5 Aménagements
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| EXA-200 | Ouvrir la campagne de demandes de l'année | SEX, REF35 | §10.2 | `amenagements.campagne` |
| EXA-201 | Lister les élèves à qui proposer une demande (drapeaux PAP/PPS, l'an passé, signalés) | SYS | sans diagnostic | — |
| EXA-202 | Informer les titulaires ou l'élève majeur (procédure, date limite, médecin désigné) | SEX | R-EX10 | — |
| EXA-203 | Déclarer « demande déposée le … » | PAR, REF35 | — | `amenagements.demande_declaree` |
| EXA-204 | Produire l'attestation pédagogique de l'équipe (si prévue) | REF35, PP | 7 ; [NV] | — |
| EXA-205 | Relancer avant la date limite | SYS | J-15, J-3 | — |
| EXA-206 | Saisir la décision de l'autorité (mesures, épreuves, précisions, recours) | SEX | R-EX9 | `amenagements.decision_enregistree` |
| EXA-207 | Joindre la notification (domaine restreint) | SEX | 7 | — |
| EXA-208 | Enregistrer un recours et son issue | SEX | CEX-04 | — |
| EXA-209 | Ajouter une mesure d'établissement pour les épreuves internes (motif) | DIR ★ | R-EX13 | — |
| EXA-210 | Voir les mesures appliquées à une épreuve et ce qui les sert | REX | R-EX8 | — |
| EXA-211 | Voir les mesures de ses élèves | PRF | 35 | — |
| EXA-212 | Voir ses mesures et leur application | ELV, PAR | — | — |
| EXA-213 | Noter « mesure non utilisée » le jour J | SUR | CEX-09 | — |
| EXA-214 | Bilan des mesures servies (direction, référent) | DIR | — | — |
| EXA-215 | Reconduire les décisions pluriannuelles [NV] à la session suivante | SEX | contrôle | — |

### 20.6 Convocations, jour J, incidents
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| EXA-220 | Publier les convocations (candidats, titulaires, surveillants, secrétaires) | REX ★ | R-EX8, R-EX17 | `examens.convocation` |
| EXA-221 | Modifier une convocation (avis ciblé) | REX | R-EX17 | `examens.convocation_modifiee` |
| EXA-222 | Rappel la veille | SYS | — | — |
| EXA-223 | Voir sa convocation (web, mobile) | ELV, PAR, PRF | — | — |
| EXA-224 | Émarger (présent, absent, retard, sortie, abandon) | SUR | §11.2 | `examens.emargement` |
| EXA-225 | Signaler les absents à la vie scolaire (15 min) | SYS | R-EX18 | `examens.absent_epreuve` |
| EXA-226 | Marquer les candidats `en_examen` au 12 | SYS | — | — |
| EXA-227 | Admettre un retardataire (règle) | SUR, REX | S38-02 | — |
| EXA-228 | Déclarer un incident (malaise, matériel, sortie) | SUR, CPE | — | `examens.incident` |
| EXA-229 | Interrompre et reprendre l'épreuve (alerte PPMS), temps compensé | REX ★ | CEX-15 | — |
| EXA-230 | Déclarer une fraude présumée (épreuve interne) | SUR | R-EX24 | `examens.fraude_presumee` |
| EXA-231 | Décider des suites d'une fraude interne (maintien, annulation, procédure 13) | DIR ★ | R-EX24 | `examens.fraude_suites` |
| EXA-232 | Rédiger le PV de fraude (examen national) depuis la trame, contresignatures | SUR, REX | RG-X16 | `examens.pv_fraude` |
| EXA-233 | Tracer la transmission du PV au recteur | DIR | — | — |
| EXA-234 | Compter les copies par salle (rendues = présents) | SUR | — | — |
| EXA-235 | Clore la salle (heure, signature du responsable) | SUR | — | — |

### 20.7 Copies, correction, notes
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| EXA-250 | Créer l'évaluation commune et les évaluations par cours au 21 | SYS | 21 §4 | `evaluations.*` (21) |
| EXA-251 | Répartir les copies aux correcteurs | REX | S38-03 | `examens.copies_reparties` |
| EXA-252 | Double correction, correction croisée | REX | — | — |
| EXA-253 | Déclarer « corrigée », signaler un problème de copie | PRF | — | — |
| EXA-254 | Saisir par numéro d'anonymat (au 21) | PRF | — | — |
| EXA-255 | Lever l'anonymat | REX ★ | toutes notes saisies | `examens.anonymat_leve` |
| EXA-256 | Numériser les copies (facultatif) | REX | Q-EX8 | — |
| EXA-257 | Déclarer une copie perdue | REX | CEX-34 | — |
| EXA-258 | Statistiques de l'épreuve (classes, correcteurs) | PRF, DIR | — | — |
| EXA-259 | Harmoniser (réunion, ajustement décidé au 21) | DIR | 21 | — |
| EXA-260 | Écarts contrôle continu / épreuve | DIR | EVA-216 | — |
| EXA-261 | Simulation du DNB (indicative) | DIR, PP | R-EX20 | — |
| EXA-262 | Activer la simulation pour les familles | DIR | S38-04 | — |
| EXA-263 | Clore la session (bilan, archivage) | REX | — | `examens.session_close` |

### 20.8 Oraux et CCF
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| EXA-280 | Ouvrir le dépôt des sujets d'oral du DNB (date limite) | REX | §12.1 | — |
| EXA-281 | Déclarer son sujet ou projet, son groupe | ELV | — | — |
| EXA-282 | Valider les sujets, constituer les groupes | PP, REX | — | — |
| EXA-283 | Constituer les jurys | REX | — | — |
| EXA-284 | Proposer le planning des passages | SYS | — | — |
| EXA-285 | Convoquer candidats et jurys | REX | — | — |
| EXA-286 | Noter sur la grille, valider | JUR | R-EX21 | `examens.oral_note` |
| EXA-287 | Verrouiller les notes d'oral | REX ★ | — | — |
| EXA-288 | Organiser un rattrapage | REX | — | — |
| EXA-289 | Déposer ses questions de grand oral (blanc) | ELV | §12.2 | — |
| EXA-300 | Créer les situations de CCF (matière, date, élèves, évaluateurs, grille) | PEPS, PLP | §12.3 | `examens.ccf_situation` |
| EXA-301 | Prendre en compte une inaptitude (conséquence du 34) | PEPS | R-EX12 | — |
| EXA-302 | Évaluer une situation (grille), valider | PEPS, PLP | — | `examens.ccf_note` |
| EXA-303 | Signer le PV de CCF | évaluateurs | 7 | `examens.ccf_pv` |
| EXA-304 | Rattrapage de CCF | PEPS, PLP | — | — |
| EXA-305 | Chef-d'œuvre : notes d'élaboration, oral, calcul 50/50 affiché | PLP | RG-X11 | — |
| EXA-306 | Relier une évaluation de PFMP (39) | PLP | — | — |
| EXA-307 | Tableau des CCF (manquants, PV non signés) | DIR, REX | — | — |
| EXA-308 | Report des notes vers l'application nationale (manuel tracé) | SEX | R-EX22 | — |

### 20.9 Inscriptions nationales et résultats
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| EXA-320 | Préparer la liste des candidats par examen (série, spécialités, options, LV, EPS, aménagements) | SEX | §13.1 | — |
| EXA-321 | Liste de contrôle avant saisie (INE, état civil, options incohérentes) | SYS | — | — |
| EXA-322 | Déclarer « saisi dans Cyclades le … » | SEX | R-EX22 | `inscriptions.saisie_declaree` |
| EXA-323 | Déposer les confirmations d'inscription (PDF) | SEX | RG-X09 | — |
| EXA-324 | Signer sa confirmation (candidat, titulaire) | ELV, ELM, PAR | 7 | `inscriptions.confirmation_signee` |
| EXA-325 | Relancer les non signées, rendez-vous de signature | SEX | CEX-37 | — |
| EXA-326 | Déclarer « confirmations déposées dans Cyclades » | SEX | — | — |
| EXA-327 | Demander une correction (état civil, option) | PAR, ELM | CEX-40 | — |
| EXA-328 | Rappeler la mise à disposition des convocations nationales | SYS | RG-X10 | — |
| EXA-329 | Joindre la convocation nationale au coffre | SEX | 7 | — |
| EXA-340 | Saisir ou importer les résultats officiels (tableur) | SEX | FX-09, CEX-41 | `examens.resultat_officiel` |
| EXA-341 | Publier les résultats aux élèves et familles à la date nationale | SYS | R-EX25 | `resultats.publies` |
| EXA-342 | Lister les élèves au second groupe | SEX | — | — |
| EXA-343 | Déposer relevés et attestations de réussite au coffre | SEX | 7 | — |
| EXA-344 | Réceptionner les diplômes (lot) | SEX | — | — |
| EXA-345 | Remettre un diplôme (en main propre, cérémonie, courrier, mandataire), signature | SEX | CEX-44 | `diplomes.remis` |
| EXA-346 | Lister les diplômes non retirés, relancer | SEX | — | `diplomes.a_retirer` |
| EXA-347 | Taux de réussite (agrégés) | DIR | R-EX28 | — |
| EXA-348 | Export officiel (si format) — bloqué D-2 | SEX | lot EX17 | `export.realise` |

### 20.10 Certifications
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| EXA-360 | Créer une session de certification (Pix, ASSR, PSC1…) | responsable | §14 | `certifications.session` |
| EXA-361 | Lister les éligibles (prérequis déclaré) | SYS | RG-X19 | — |
| EXA-362 | Convoquer, émarger (moteur commun) | responsable, SUR | — | — |
| EXA-363 | Saisir ou importer les résultats | responsable | — | `certifications.resultat` |
| EXA-364 | Produire et remettre l'attestation (ASSR, ASNS) | DIR | RG-X20, X21 | `certifications.attestation` |
| EXA-365 | Valider l'ASNS (test d'EPS) | PEPS | RG-X21 | — |
| EXA-366 | Délivrer un duplicata à un ancien élève | SEX | CEX-46 | — |
| EXA-367 | Verser au livret (22, 23) | SYS | — | — |
| EXA-368 | Planifier le rattrapage Pix | responsable | — | — |

### 20.11 Cérémonie (reprise de M17)
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| EXA-440 | Créer la cérémonie (lieu, date, diplômes concernés : DNB, bac, les deux) | DIR, SEX | NXI-01 | `ceremonie.creee` |
| EXA-441 | Capacité totale, personnes par élève, date limite | SEX | NXI-02, 06 | — |
| EXA-442 | Constituer la liste des invités depuis les lauréats (résultats officiels), sans les opposants | SYS | R-EX29 | — |
| EXA-443 | Ajouter un invité hors liste (motif) | DIR | — | — |
| EXA-444 | Répondre (présence, nombre) dans le compte ou par lien nominatif | ancien élève, PAR | R-EX30 | — |
| EXA-445 | Envoyer l'invitation et les rappels (8) | SEX | — | `ceremonie.invitation` |
| EXA-446 | Tableau des réponses, liste d'accueil (14) | SEX | — | — |
| EXA-447 | Lier la remise des diplômes à la cérémonie (EXA-345) | SEX | — | — |
| EXA-448 | Proposer le questionnaire de devenir (37) aux consentants | SEX | 37 | — |
| EXA-449 | Clore la cérémonie, purger les réponses à 3 mois | SYS | §17.4 | — |
| EXA-450 | S'opposer à figurer parmi les invités publics | PAR, ELM | R-EX29 | — |
| EXA-451 | Annoncer la cérémonie (17) | DIR | — | — |
| EXA-452 | Reprise des réponses de la cérémonie en cours (Next.js), si la bascule tombe avant | SEX | Q-EX13 | — |

### 20.12 Centre d'examen, IA, mobile, pilotage
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| EXA-400 | Déclarer l'établissement centre pour une session nationale | DIR | F38-2 | — |
| EXA-401 | Recevoir la liste des candidats extérieurs (données minimales) | SEX | CEX-53 | — |
| EXA-402 | Recevoir la liste des surveillants convoqués | SEX | — | — |
| EXA-403 | Tracer la réception et le coffre des sujets nationaux (heure, témoins) | DIR | RG-X18 | — |
| EXA-404 | Tracer l'ouverture des enveloppes (PV) | DIR | — | — |
| EXA-405 | Bordereau de remise des copies | REX | — | — |
| EXA-406 | Purger les données des candidats extérieurs après la session | SYS | §17.4 | — |
| EXA-410 | IA : proposer placement, surveillances, planning d'oraux | REX | §16 | — |
| EXA-411 | IA : contrôle du plan reformulé | REX | — | — |
| EXA-412 | IA : brouillon d'information aux familles | REX | confirmation | — |
| EXA-413 | IA : expliquer une mesure au surveillant | SUR | sans motif | — |
| EXA-420 | Mobile : convocation, rappel, salle (élève, famille) | ELV, PAR | 11 | — |
| EXA-421 | Mobile : émargement hors ligne, incidents | SUR | 11 | — |
| EXA-422 | Mobile : erratum reçu en salle | SUR | — | — |
| EXA-430 | Tableau de bord des examens de l'année (sessions, prêts, problèmes) | DIR | — | — |
| EXA-431 | Indicateurs (réussite, mesures servies, incidents) au 47 | DIR | R-EX28 | — |

**Total : 180 actions**, plus 36 de la passe personas (§28.2) = **216**.

---

## 21. Règles transverses (R-EX1 et suivantes) [R]

| # | Règle |
|---|---|
| R-EX1 | Le 38 ne calcule ni moyenne ni note du 21 ; il organise et transmet les statuts. |
| R-EX2 | Toute date nationale est un **paramètre** daté et sourcé. |
| R-EX3 | Un examen national prime sur une réservation ordinaire, sans l'écraser (R-AG21). |
| R-EX4 | Candidats d'une épreuve calculés **à la date de l'épreuve**. |
| R-EX5 | Chaque écriture porte auteur, date, version ; aucun écrasement silencieux (E6, E7). |
| R-EX6 | Les séances touchées sont proposées puis **validées** ; jamais neutralisées en silence. |
| R-EX7 | Un plan publié modifié produit des avis **ciblés** aux seules personnes touchées. |
| R-EX8 | **Contrôle bloquant de publication** : candidat placé une fois par créneau ; capacité tenue ; **toute mesure servie** (salle, durée, personne, sujet, matériel) ; heure de fin de salle ≥ fin du dernier candidat ; chaque salle a un responsable. |
| R-EX9 | Majoration de temps ≤ 1/3 par épreuve. |
| R-EX10 | La date limite de demande d'aménagement = date limite d'inscription de l'examen (paramètre), rappelée. |
| R-EX11 | Aucun document médical n'est déposé ni conservé dans le 38. |
| R-EX12 | Les mesures se montrent **sans motif** et **à qui en a besoin** (surveillant : ses candidats ; affichage : rien). |
| R-EX13 | Aux épreuves internes : mesures décidées > mesures demandées (provisoires, marquées) > aménagements d'évaluation du PAP/PPS > mesure d'établissement motivée. |
| R-EX14 | Sujet : accès réservé, chaque ouverture journalisée, gel à J-2, rien de visible des élèves ni de l'IA avant la fin de l'épreuve. |
| R-EX15 | Surveillances : équité mesurée (heures cumulées rapportées à la quotité), EDT respecté, jamais une contrainte personnelle d'un collègue exposée. |
| R-EX16 | Un professeur ne surveille pas une épreuve pendant un de ses cours maintenus. |
| R-EX17 | Une convocation modifiée remplace la précédente, marquée « modifiée le … ». |
| R-EX18 | Un absent à une épreuve est signalé à la vie scolaire dans les 15 minutes. |
| R-EX19 | Le comptage des copies (rendues = présents) est fait par salle avant clôture. |
| R-EX20 | Toute simulation du DNB est marquée « estimation non officielle » ; désactivée pour les familles par défaut. |
| R-EX21 | Une note d'oral ou de CCF validée est **verrouillée** ; correction motivée, versionnée. |
| R-EX22 | Tant qu'aucune interface officielle n'existe, ScolIA **trace le report**, ne prétend jamais avoir transmis. |
| R-EX23 | Anonymat : correspondance visible du seul responsable jusqu'à la levée. |
| R-EX24 | Aucune sanction ni annulation automatique sur une fraude ; décision de la direction ; procédure au 13. |
| R-EX25 | Un résultat national n'est publié dans ScolIA qu'à la date nationale. |
| R-EX26 | Une copie perdue ne vaut jamais 0 automatiquement. |
| R-EX27 | L'IA ne corrige, ne note, ne décide aucun aménagement ni aucune suite de fraude. |
| R-EX28 | Indicateurs agrégés ; petits effectifs (< 5) masqués. |
| R-EX29 | Les lauréats sont informés de l'usage de leur résultat (cérémonie, liste interne) et peuvent s'y opposer ; ScolIA ne publie aucun résultat à l'extérieur. |
| R-EX30 | Aucune page publique ni lien ne renvoie une donnée personnelle sans authentification (ND-01). |

---

## 22. Événements (outbox, dans la transaction du fait)
**Émis** : `examens.session_creee`, `.epreuve_maj`, `.epreuve_annulee`, `.exemption`, `.calendrier_publie`, `.calendrier_maj`, `.cours_neutralises`, `.placement_maj`, `.surveillance_maj`, `.sujet_a_deposer`, `.sujet_depose`, `.sujet_valide`, `.sujet_gele`, `.sujet_imprime`, `.sujet_compromis`, `.convocation`, `.convocation_modifiee`, `.emargement`, `.absent_epreuve`, `.incident`, `.fraude_presumee`, `.fraude_suites`, `.pv_fraude`, `.copies_reparties`, `.anonymat_leve`, `.oral_note`, `.ccf_situation`, `.ccf_note`, `.ccf_pv`, `.resultat_officiel`, `.session_close` ; `amenagements.campagne`, `.demande_declaree`, `.decision_enregistree` ; `inscriptions.saisie_declaree`, `.confirmation_signee` ; `resultats.publies` ; `diplomes.remis`, `.a_retirer` ; `certifications.session`, `.resultat`, `.attestation` ; `ceremonie.creee`, `.invitation`.
**Consommés** : `edt.seance_*`, `edt.version_publiee` (10) → recalcul ; `referentiel.cours_membre_*`, `fermeture_*` (5) → candidats, salles ; `absence_personnel.*` (15) → surveillances ; `sante.dispense_*` (34) → CCF, EPS ; `besoins.plan_*` (35) → campagne d'aménagements ; `sortie.*` (42), `stage.periode_*` (39), `discipline.exclusion_*` (13) → conflits ; `eleve.majeur`, `.sorti`, `.decede`, `liens.*` (2, 4) ; `evaluations.notes_saisies` (21) → levée d'anonymat possible.

---

## 23. Écrans clés (Leptos)
1. **Tableau des examens de l'année** (tuiles Bento : sessions, état « prêt / problèmes », prochaines échéances).
2. **Session** (épreuves, publics, séances touchées, contrôle du plan).
3. **Placement** (salles × candidats, plan de salle glisser-déposer, mesures en pastille).
4. **Surveillances** (grille personnels × créneaux, équité, échanges).
5. **Sujets** (dépôt, variantes, validation, gel, lot d'impression, journal).
6. **Aménagements** (campagne, décisions, application par épreuve, bilan).
7. **Salle le jour J** (tablette : émargement, incidents, erratum, clôture).
8. **Copies et correction** (répartition, avancement, anonymat).
9. **Oraux et CCF** (planning, grilles, PV).
10. **Inscriptions nationales et résultats** (contrôles, signatures, résultats, diplômes).
11. **Certifications**, **cérémonie**.
12. **Élève et famille** : « mes épreuves » (convocations, salle, mesures), résultats, attestations.

---

## 24. Modèle de données cible (brique `examens`, sans JSON, E1–E14)

Mêmes règles qu'au 37 (UUID applicatif, `etablissement_id NOT NULL`, FK composites, `cree_le`/`modifie_le`, RLS stricte, registre, pas de JSON, BYTEA ni tableau, fichiers en `ent_objet`).

```
-- Référentiels
exa_mesure_ref                id, etablissement_id NULL (NULL = préréglage global, E12 globale_reference), code, libelle,
                              famille ('temps','salle','personne','materiel','sujet','autre'), parametre_unite ('minutes','fraction','aucun'),
                              ordre, archive_le NULL
exa_calendrier_date           id, etablissement_id, annee_id, nature ('inscription_limite','confirmation_limite','amenagement_limite',
                              'convocations_dispo','resultats_dnb','resultats_bac','lsl_limite','eam','autre'), le DATE, heure TIME NULL,
                              libelle VARCHAR(200), source VARCHAR(300)
exa_regle_etablissement       id, etablissement_id, retard_admission_minutes SMALLINT DEFAULT 60, correction_propres_eleves BOOLEAN DEFAULT true,
                              anonymat_defaut BOOLEAN DEFAULT false, gel_sujet_jours SMALLINT DEFAULT 2, simulation_dnb_familles BOOLEAN DEFAULT false
exa_certification_ref         id, etablissement_id NULL, code, libelle, nature ('pix','assr1','assr2','asns','psc1','libre'), archive_le NULL

-- Sessions et épreuves
exa_session                   id, etablissement_id, annee_id, nature ('interne','nationale_locale','centre_national','certification'),
                              gabarit_code VARCHAR(40), titre VARCHAR(200), du DATE, au DATE, responsable_personne_id,
                              regle_notes ('evaluation_commune_comptee','evaluation_commune_non_comptee','hors_carnet'),
                              certification_id NULL, etat ('brouillon','planifiee','organisee','publiee','en_cours','terminee','corrigee',
                              'close','annulee'), version
exa_epreuve                   id, etablissement_id, session_id, matiere_id NULL, libelle VARCHAR(200), nature ('ecrit','oral','pratique','qcm',
                              'numerique'), debut TIMESTAMPTZ, duree_minutes SMALLINT, materiel VARCHAR(600), anonymat BOOLEAN,
                              evaluation_commune_id NULL (21), etat ('prevue','tenue','annulee','reportee'), version
exa_epreuve_public            id, etablissement_id, epreuve_id, niveau_id NULL, classe_id NULL, groupe_id NULL, eleve_id NULL   -- CHECK un seul
exa_epreuve_variante          id, etablissement_id, epreuve_id, code, libelle, critere ('parcours','groupe','adaptation'), cle_critere VARCHAR(80) NULL
exa_seance_touchee            id, etablissement_id, session_id, seance_id (10), nature ('neutralisee','deplacee','non_assuree'),
                              valide_par NULL, valide_le NULL
exa_candidat                  id, etablissement_id, epreuve_id, eleve_id NULL, candidat_externe_id NULL, variante_id NULL,
                              statut ('inscrit','exempte','present','absent','retard','abandon','exclu','dispense'), motif_exemption VARCHAR(400) NULL,
                              anonymat_numero VARCHAR(20) NULL, version    -- CHECK (eleve_id IS NOT NULL) <> (candidat_externe_id IS NOT NULL)
exa_candidat_externe          id, etablissement_id, session_id, nom, prenoms, date_naissance DATE, numero_candidat VARCHAR(40), supprime_le NULL

-- Salles, places, surveillances
exa_salle                     id, etablissement_id, epreuve_id, salle_id (5), capacite_utile SMALLINT, a_part BOOLEAN, fin_prevue TIMESTAMPTZ,
                              responsable_personne_id NULL, reservation_id NULL (19), etat ('prevue','ouverte','close'), close_le NULL
exa_place                     id, etablissement_id, candidat_id, salle_epreuve_id, numero_table SMALLINT NULL, verrouillee BOOLEAN,
                              fin_candidat TIMESTAMPTZ    UNIQUE (etablissement_id, candidat_id)
exa_voisin_separe             id, etablissement_id, session_id, eleve_a_id, eleve_b_id
exa_surveillance              id, etablissement_id, salle_epreuve_id, personne_id, role ('responsable_salle','surveillant','secretaire',
                              'lecteur','appariteur','jury'), debut TIMESTAMPTZ, fin TIMESTAMPTZ, candidat_id NULL (secrétaire, lecteur), version
exa_surveillance_echange      id, etablissement_id, surveillance_id, demandeur_id, repreneur_id, etat ('demande','accepte','refuse','valide'), le

-- Sujets
exa_sujet                     id, etablissement_id, epreuve_id, variante_id NULL, nature ('sujet','corrige','secours'), objet_id (ent_objet),
                              version SMALLINT, depose_par, depose_le, valide_par NULL, valide_le NULL, gele_le NULL, compromis_le NULL,
                              publie_eleves_le NULL
exa_sujet_auteur              id, etablissement_id, epreuve_id, personne_id
exa_sujet_acces               id, etablissement_id, sujet_id, personne_id, nature ('ouverture','telechargement','impression'), le   -- ajout seul
exa_impression_lot            id, etablissement_id, epreuve_id, reprographe_personne_id, fenetre_du TIMESTAMPTZ, fenetre_au TIMESTAMPTZ,
                              imprime_le NULL
exa_impression_ligne          id, etablissement_id, lot_id, salle_epreuve_id, sujet_id, exemplaires SMALLINT

-- Aménagements
exa_amenagement_demande       id, etablissement_id, eleve_id, annee_id, examen_code VARCHAR(40), propose_le, informe_le NULL,
                              deposee_le NULL, declaree_par NULL, etat ('a_proposer','proposee','deposee','decidee','sans_suite')
exa_amenagement_decision      id, etablissement_id, eleve_id, demande_id NULL, examen_code VARCHAR(40), session_libelle VARCHAR(120),
                              autorite VARCHAR(200), decide_le DATE, valable_jusqu_au DATE NULL, document_id NULL (7, domaine restreint),
                              recours_depose_le NULL, recours_issue ('maintenue','modifiee','annulee') NULL, saisie_par, version
exa_amenagement_mesure        id, etablissement_id, decision_id NULL, demande_id NULL, eleve_id, mesure_ref_id, valeur NUMERIC NULL,
                              precision VARCHAR(300) NULL, portee ('toutes_epreuves','epreuves_listees'), statut ('accordee','refusee',
                              'demandee_provisoire','pedagogique','etablissement'), motif_etablissement VARCHAR(400) NULL
                              -- CHECK (mesure de temps) valeur <= 0.3334 si unité fraction                     (R-EX9)
exa_amenagement_mesure_epreuve id, etablissement_id, mesure_id, matiere_id
exa_mesure_application        id, etablissement_id, candidat_id, mesure_id, servie_par ('salle','duree','personne','sujet','materiel'),
                              non_utilisee_le NULL

-- Jour J
exa_emargement                id, etablissement_id, candidat_id, nature ('entree','sortie_temporaire','retour','sortie_definitive','abandon'),
                              le, par, source ('web','mobile_hors_ligne')   -- ajout seul
exa_incident                  id, etablissement_id, salle_epreuve_id NULL, candidat_id NULL, nature ('retard','malaise','materiel','sortie',
                              'fraude_presumee','alerte','erratum','autre'), le, par, faits VARCHAR(2000), suites ('aucune','note_maintenue',
                              'epreuve_annulee','procedure_discipline','pv_transmis') NULL, decide_par NULL, decide_le NULL,
                              procedure_id NULL (13), version
exa_incident_piece            id, etablissement_id, incident_id, objet_id
exa_pv                        id, etablissement_id, incident_id NULL, nature ('fraude','ouverture_sujets','remise_copies','ccf'),
                              document_id (7), transmis_le NULL, transmis_a VARCHAR(200) NULL
exa_pv_signature              id, etablissement_id, pv_id, personne_id NULL, qualite ('surveillant','auteur','chef_centre','evaluateur'),
                              signe_le NULL, refus BOOLEAN
exa_interruption              id, etablissement_id, epreuve_id, du TIMESTAMPTZ, au TIMESTAMPTZ NULL, motif VARCHAR(300), temps_compense_minutes SMALLINT

-- Copies
exa_copie                     id, etablissement_id, candidat_id, salle_epreuve_id, rendue_le NULL, correcteur_personne_id NULL,
                              second_correcteur_personne_id NULL, corrigee_le NULL, perdue_le NULL, objet_id NULL (numérisation)
exa_anonymat_levee            id, etablissement_id, epreuve_id, leve_par, leve_le

-- Oraux, CCF, résultats d'épreuves nationales organisées localement
exa_oral_sujet                id, etablissement_id, candidat_id, titre VARCHAR(300), groupe_code VARCHAR(20) NULL, depose_le, valide_par NULL
exa_jury                      id, etablissement_id, epreuve_id, libelle VARCHAR(80)
exa_jury_membre               id, etablissement_id, jury_id, personne_id
exa_passage                   id, etablissement_id, candidat_id, jury_id, debut TIMESTAMPTZ, salle_id NULL
exa_grille                    id, etablissement_id, code, libelle, bareme NUMERIC, archive_le NULL
exa_grille_critere            id, etablissement_id, grille_id, libelle VARCHAR(300), points NUMERIC, ordre
exa_ccf_situation             id, etablissement_id, session_id, matiere_id, libelle VARCHAR(200), le DATE, grille_id NULL
exa_resultat_epreuve          id, etablissement_id, candidat_id, grille_id NULL, note NUMERIC NULL, statut ('note','absent','dispense','inapte'),
                              valide_par NULL, valide_le NULL, verrouille BOOLEAN, reporte_le NULL, version
exa_resultat_critere          id, etablissement_id, resultat_id, critere_id, points NUMERIC
exa_resultat_historique       id, etablissement_id, resultat_id, version, note NUMERIC NULL, motif VARCHAR(400), par, le   -- ajout seul

-- National : inscriptions, résultats, diplômes
exa_inscription_nationale     id, etablissement_id, eleve_id, annee_id, examen_code VARCHAR(40), serie_voie VARCHAR(80) NULL,
                              saisie_cyclades_le NULL, saisie_par NULL, confirmation_objet_id NULL, confirmation_deposee_le NULL, version
exa_inscription_choix         id, etablissement_id, inscription_id, nature ('specialite','option','langue','eps','autre'), matiere_id NULL,
                              libelle VARCHAR(200)
exa_confirmation_signature    id, etablissement_id, inscription_id, signataire_personne_id, qualite ('candidat','titulaire'), signe_le, mode ('electronique','papier')
exa_correction_demande        id, etablissement_id, inscription_id, demande_par, objet VARCHAR(600), etat ('ouverte','traitee','refusee'), traite_par NULL
exa_resultat_officiel         id, etablissement_id, eleve_id, examen_code VARCHAR(40), session_libelle VARCHAR(60), decision ('admis','ajourne',
                              'second_groupe','admis_second_groupe','absent','autre'), mention VARCHAR(60) NULL, source ('saisie','fichier'),
                              saisi_par, saisi_le, publie_le NULL, version
exa_resultat_officiel_note    id, etablissement_id, resultat_id, libelle VARCHAR(120), note NUMERIC NULL
exa_diplome                   id, etablissement_id, eleve_id, resultat_id NULL, examen_code VARCHAR(40), recu_le DATE NULL,
                              remis_le NULL, mode_remise ('main_propre','ceremonie','courrier','mandataire') NULL, remis_a VARCHAR(200) NULL,
                              signature_document_id NULL (7)

-- Certifications
exa_certification_passage     id, etablissement_id, session_id NULL, eleve_id, certification_id, prerequis_declare BOOLEAN NULL,
                              resultat ('obtenue','non_obtenue','absent','en_attente'), niveau VARCHAR(40) NULL, score NUMERIC NULL,
                              le DATE, attestation_document_id NULL, version

-- Cérémonie
exa_ceremonie                 id, etablissement_id, annee_id, titre VARCHAR(200), lieu VARCHAR(300), debut TIMESTAMPTZ, fin TIMESTAMPTZ NULL,
                              diplomes ('dnb','bac','les_deux','autre'), capacite_totale SMALLINT, max_par_eleve SMALLINT, reponse_limite TIMESTAMPTZ,
                              etat ('brouillon','ouverte','close')
exa_ceremonie_invite          id, etablissement_id, ceremonie_id, eleve_id, origine ('laureat','ajout_direction'), motif VARCHAR(300) NULL
exa_ceremonie_reponse         id, etablissement_id, invite_id, reponse ('present','absent'), personnes SMALLINT, repondu_par_personne_id NULL,
                              lien_id NULL (7), le
exa_opposition_publication    id, etablissement_id, eleve_id, nature ('ceremonie','liste_interne'), par, le, retiree_le NULL
```
**Exceptions D-036** : aucune. **Tables globales** (E12, `globale_reference`) : lignes à `etablissement_id NULL` de `exa_mesure_ref` et `exa_certification_ref` (préréglages, sans donnée d'établissement, E4). **Files d'effets de bord** : aucune nouvelle (avis et convocations par le 8 et le 7, déjà E9). **Volatiles** : aucune. **Fichiers** : sujets, copies, PV, confirmations, notifications, attestations = `ent_objet` (E5), domaine restreint pour les décisions d'aménagement. **Registre** : export `double` pour `exa_amenagement_*` et `exa_sujet*`.

---

## 25. Lots de code (équipe Sonnet) et migrations

### 25.1 Lots
| Lot | Contenu | Dépend de | Migr. | Taille | Critère de fin |
|---|---|---|---|---|---|
| **EX0** | XD1 à XD4 : export « Notanet » renommé ou retiré (avec X0 du 9), notes publiées seulement, journal | — | — | S | aucun libellé officiel ; test « note non publiée exclue » |
| **EX1** | référentiels (mesures, certifications), calendrier daté, règles d'établissement, trames | 7 | 2760 | S | préréglages chargés |
| **EX2** | sessions, gabarits, épreuves, publics, variantes, candidats, séances touchées (10), conflits | EX1 ; 5, 10, 19 | 2761 | L | brevet blanc créé depuis le gabarit ; neutralisation validée |
| **EX3** | salles, placement (moteur déterministe), anonymat, affichages, contrôle R-EX8 | EX2 | 2762 | L | contrôle bloquant ; test tiers-temps |
| **EX4** | surveillances (disponibilités, proposition équitable, échanges, remplacement) | EX3 ; 6, 15 | 2763 | M | R-EX15, R-EX16 |
| **EX5** | aménagements (campagne, décisions, mesures, application, bilan) | EX3 ; 35 (drapeau), 7 | 2764 | L | R-EX8, R-EX9, R-EX11, R-EX12, R-EX13 |
| **EX6** | convocations, rappels, jour J (émargement mobile hors ligne, absents, incidents, interruptions) | EX4, EX5 ; 8, 11, 12, 14 | 2765 | L | absent signalé en 15 min ; avis ciblés |
| **EX7** | sujets (dépôt, accès journalisé, gel, lot d'impression, compromis, publication) | EX2 ; 7 | 2766 | M | R-EX14 ; journal |
| **EX8** | copies, répartition, anonymat, évaluation commune au 21, statistiques, simulation DNB | EX6 ; **21** (évaluation commune) | 2767 | M | aucune note hors 21 ; levée d'anonymat |
| **EX9** | fraude (interne et PV national), signatures | EX6 ; 13, 7 | 2768 | S | aucune suite automatique |
| **EX10** | oraux (DNB, grand oral blanc) : sujets, jurys, planning, grilles, verrou | EX4 | 2769 | M | R-EX21 |
| **EX11** | CCF (EPS, voie pro, chef-d'œuvre), PV, inaptitudes (34) | EX10 ; 22, 34, 39 | 2770 | M | PV signés ; motif médical invisible |
| **EX12** | inscriptions nationales (préparation, contrôles, confirmations signées, corrections) | EX1 ; 5, 23, 7 | 2771 | M | liste des non signées ; signature candidat + titulaire |
| **EX13** | résultats officiels (tableur), publication datée, diplômes et remise | EX12 ; 9 (X2) | 2772 | M | R-EX25 |
| **EX14** | certifications (Pix, ASSR, ASNS, PSC1, libres) | EX6 ; 22, 23 | 2773 | M | attestation remise et versée |
| **EX15** | cérémonie (reprise M17), oppositions | EX13 ; 8, 19, 37 | 2774 | S | R-EX29, R-EX30 |
| **EX16** | centre d'examen national ; IA ; mobile ; tableau de bord et indicateurs (47) | EX6–EX15 ; IA_COEUR | 2775 | M | jeu d'évaluation IA ; purge des externes |
| **EX17** | formats officiels (si une interface existe) | **D-2** ; 9 X7 | — | ? | **bloqué** |
| **EX18** | RLS stricte, FK composites, isolation, registre | tous | 2778 | S | matrice verte |

**2776–2777** attribuées par la passe personas (§28.6) ; **2778** RLS stricte (EX18) ; **2779** retraits (réservé, sur décision, E8). Aucune réserve libre : un besoin nouveau demandera une plage au pilote.

### 25.2 Ordre et calendrier (D-034)
- **V1 (janvier–février 2027)** : EX0, EX1, EX2, EX3, EX5 (sans campagne), EX6, EX8 — un **brevet blanc** et un **bac blanc** complets, aménagements appliqués.
- **Épaississement (février–août 2027)** : EX4, EX7, EX9 à EX15 ; EX16 selon F38-2 ; usage en **doublon** en 2027-2028 (rien de national ne dépend de ScolIA).

---

## 26. Questions, décisions appliquées par défaut (D-031) et points pour Florian

### 26.1 Tranchées selon la recommandation (*appliqué par défaut, D-031*)
| # | Question | Décision appliquée |
|---|---|---|
| Q-EX1 | Qui possède la décision d'aménagement d'examen : 35 ou 38 ? | **38** (elle pilote l'organisation) ; 35 garde le plan et les aménagements pédagogiques |
| Q-EX2 | ScolIA recueille-t-il les pièces médicales de la demande ? | **Non** (R-EX11) |
| Q-EX3 | Épreuves internes : quelles mesures ? | décidées > demandées provisoires > PAP/PPS > mesure d'établissement motivée (R-EX13) |
| Q-EX4 | Sujet « coffre » : chiffrement à ouverture différée ? | **Non** : accès réservé, journal, gel, fenêtre d'impression (plus simple et suffisant [R]) |
| Q-EX5 | Placement : par défaut alphabétique ou mélangé ? | **mélange des classes**, ordre alphabétique dans la salle ; voisins séparés |
| Q-EX6 | Une épreuve interne compte-t-elle dans la moyenne ? | **Choix par session** ; préréglage « évaluation commune comptée » (usage le plus courant [H]) |
| Q-EX7 | Simulation du DNB visible des familles ? | **Non par défaut** (S38-04) |
| Q-EX8 | Numériser les copies ? | **Facultatif**, par épreuve |
| Q-EX9 | Publier les lauréats (presse, site) ? | **Jamais** par ScolIA ; liste interne et cérémonie avec opposition (R-EX29) |
| Q-EX10 | Confirmation d'inscription d'un mineur : un ou deux titulaires ? | **Un titulaire** suffit, l'autre est **informé** ; à confirmer sur le texte [NV] |
| Q-EX11 | AESH de l'élève comme secrétaire aux épreuves internes ? | **Oui** si le 35 l'autorise ; national : selon la décision de l'autorité |
| Q-EX12 | Les mesures d'examen s'appliquent-elles aux évaluations ordinaires (21) ? | **Non** par le 38 ; le 35 gère les aménagements pédagogiques des évaluations ordinaires |
| Q-EX13 | Reprendre les réponses d'une cérémonie Next.js en cours ? | **Non** : finir dans Next.js, sauf bascule pendant l'inscription (EXA-452) |
| Q-EX14 | Banque de sujets de l'établissement | **Oui**, 5 ans, sur décision de l'auteur |

### 26.2 Ce qui engage Florian (non appliqué)
| # | Point | Pourquoi |
|---|---|---|
| **F38-1** | = F37-1 : **ND-01** (page publique des invitations, Next.js en production) | production, intouchable pour le pilote |
| F38-2 | L'établissement est-il **centre d'examen** (DNB, bac, EAM, ASSR) ? quelles consignes reçoit-il ? | conditionne le lot EX16 |
| F38-3 | Démarche éditeur : interroger la DNE sur **Cyclades** et **Notanet** (existe-t-il un échange éditeur ?) | D-029, D-2 |
| F38-4 | Rémunération des **surveillances et corrections d'examens blancs** dans le privé (HSE, Next.js M08) | coût, paie (49) |
| F38-5 | DPO : durées (§17.4), décisions d'aménagement, copies numérisées, cérémonie | registre |
| F38-6 | Pratique actuelle (Charlemagne, tableurs) pour brevet blanc, tiers-temps, CCF | calibrage V1 |
| F38-7 | Juriste : usage des résultats pour la cérémonie, opposition, publication | consentement |

### 26.3 Réglages signalés (D-023)
| # | Réglage | Préréglage |
|---|---|---|
| S38-01 | Capacité d'examen des salles | moitié de la capacité |
| S38-02 | Admission des retardataires (interne) | jusqu'à 60 min, sans temps supplémentaire |
| S38-03 | Correction de ses propres élèves | autorisée |
| S38-04 | Simulation DNB pour les familles | désactivée |
| S38-05 | Copie des convocations aux parents d'un élève majeur | oui (l'élève peut retirer) |
| S38-06 | Gel des sujets | J-2 |
| S38-07 | Rappels de signature | J-7, J-2 |

---

## 27. Faits, hypothèses, contradictions

**Faits [F]** : `notanet_export.rs`, `service.rs:15157-15209`, `routes.rs:595`, `tests_d022.rs:368-383` relus le 2026-11-11 ; Next.js M17 relu (lecture seule). Rien compilé ni exécuté.

**Hypothèses [H]** : pratiques de terrain (§1, §3), capacité d'examen, règle des retardataires, durées de conservation des PV et notes d'oral.

**Contradictions**
- **C1** `SOCLES.md` §35 place « aménagements pédagogiques **et d'examen** » au 35 ; ici la **décision d'examen** est au 38 (Q-EX1). À reporter au 35 lors de sa conception (le 35 n'est pas encore conçu).
- **C2** `09_INTEROP.md` FX-37 nomme « DNB / Notanet » ; les sources officielles trouvées le 2026-11-11 parlent du **LSU → Cyclades** sans « Notanet » (RG-X05) : le nom « Notanet » est peut-être obsolète ; à vérifier (F38-3).
- **C3** Épreuve anticipée de mathématiques : une page ministérielle dit « depuis la session 2026 », le calendrier dit passation en juin 2026 **pour** le bac 2027 : retenu le calendrier ; paramètre.
- **C4** `SOCLE2_DOMAINES.md` EXA-02 dit « aménagements issus de D18 » (= 35) : précisé (décision au 38, plan au 35).
- **C5** RG-X24 repose sur un avis CNIL de 2012 et un arrêté de 2013, peut-être remplacés : à reconfirmer (F38-7).

---

## 28. [ajout personas] Ce que change la passe personas (`PERSONAS_38.md`, 2026-11-11)

### 28.1 Bloquants tranchés (*appliqué par défaut, D-031*)
- **B38-1 Le surveillant remplaçant de dernière minute** reçoit-il les mesures des candidats ? → **oui**, au moment de sa désignation et seulement pour la salle, avec journal ; il les perd à la clôture de la salle (R-EX31).
- **B38-2 Une famille découvre la mesure d'un autre élève** par l'affichage ou le plan de salle (salle à part nommée « salle tiers-temps ») → les **noms de salles** affichés sont neutres (« salle 12 »), jamais « tiers-temps », « ULIS », « handicap » (R-EX32).
- **B38-3 Sujet visible par le cahier de textes ou la banque de sujets** avant l'épreuve (attaquant, professeur imprudent) → le dépôt d'un sujet d'une épreuve **non tenue** est **refusé** partout ailleurs que dans le 38 ; la banque ne montre que les sujets d'épreuves **tenues** (R-EX33).

**Précisions** : le contrôle de publication (R-EX8, EXA-133) porte sur **toutes les sessions** d'un même créneau (salles, surveillants, candidats), pas sur une seule (M38-01) ; les **co-auteurs** d'un sujet (EXA-172) sont des **personnels** seulement (M38-30).

### 28.2 Actions ajoutées
| Id | Action | Acteurs | Règles |
|---|---|---|---|
| EXA-460 | Transmettre les mesures au surveillant remplaçant (salle seulement, journal) | SYS | R-EX31 |
| EXA-461 | Retirer l'accès aux mesures à la clôture de la salle | SYS | R-EX31 |
| EXA-462 | Nommer les salles affichées de façon neutre | SYS | R-EX32 |
| EXA-463 | Refuser le dépôt d'un sujet d'épreuve non tenue hors du 38 | SYS | R-EX33 |
| EXA-464 | Prévoir un temps de pause pour un élève avec PAI (consigne minimale du 34) | REX | 34 |
| EXA-465 | Joindre la consigne minimale d'urgence (« trousse ») au responsable de salle | SYS | 34 |
| EXA-466 | Prévoir un candidat en fauteuil (salle accessible, table adaptée) | REX | R-EX8 |
| EXA-467 | Convoquer l'AESH de l'élève et prévenir son employeur des horaires | REX | 35, 6 |
| EXA-468 | Mode « jeûne » ou fête religieuse déclarée : signaler l'épreuve un jour férié de l'élève (information, pas d'exemption automatique) | REX | — |
| EXA-469 | Épreuve un samedi : avis aux familles séparées (chaque titulaire) | SYS | 4 |
| EXA-470 | Remplacer à chaud une salle indisponible, réaffecter places et surveillants | REX | CEX-14 |
| EXA-471 | Afficher au candidat « ce que je dois apporter » (matériel, convocation) | SYS | — |
| EXA-472 | Traduire la convocation (langue de la famille) | SYS | 8 |
| EXA-473 | Imprimer les convocations pour les familles sans compte | SEX | 7 |
| EXA-474 | Voir l'historique des modifications du plan (qui a déplacé qui) | DIR | E7 |
| EXA-475 | Échanger une surveillance via le mobile avec validation | PRF | — |
| EXA-476 | Signaler un conflit de surveillance avec une obligation personnelle (sans motif public) | PRF | R-EX15 |
| EXA-477 | Liste des candidats « à risque d'absence » (déterministe : absents aux deux dernières épreuves) pour la vie scolaire | CPE | sans score |
| EXA-478 | Réouvrir une salle close (erreur), motif | REX ★ | — |
| EXA-479 | Déclarer « copie non rendue » (le candidat est parti avec) | SUR | CEX-34 |
| EXA-480 | Déclarer une contestation de note d'oral (recours) | PAR, ELM | R-EX21 |
| EXA-481 | Tracer l'issue d'une contestation | DIR | — |
| EXA-482 | Exporter pour l'auditeur le registre des traitements du socle | DPO | 27 |
| EXA-483 | Droit d'accès : données d'examen d'une personne | SEX | 27 |
| EXA-484 | Rapport de purge (copies, candidats externes, cérémonie) | SYS | §17.4 |
| EXA-485 | Journal de lecture des décisions d'aménagement | DIR, DPO | E7 |
| EXA-486 | Alerte d'accès anormal à un sujet (nombre d'ouvertures, horaire inhabituel) | SYS | R-EX14 |
| EXA-487 | Limiter et signaler les tentatives sur les liens de réponse à la cérémonie | SYS | R-EX30 |
| EXA-488 | Mode « doublon » : sessions tenues sans avis aux familles sans accord de la direction | DIR | D-034 |
| EXA-489 | Reprendre les mesures d'aménagement de l'ancien établissement (document fourni par la famille) | SEX | R-EX11 |
| EXA-490 | Marquer un candidat « allophone » pour l'usage d'un dictionnaire bilingue si la règle de l'épreuve l'autorise | REX | [NV] national |
| EXA-491 | Préparer les étiquettes de copies (numéro d'anonymat, épreuve) | REX | — |
| EXA-492 | Bordereau de ramassage par salle signé | SUR | R-EX19 |
| EXA-493 | Annuler une convocation d'un élève parti | SYS | CEX-30 |
| EXA-494 | Voir les épreuves de la semaine sur la page « Aujourd'hui » (29) | tous | 29 |
| EXA-495 | Prévenir le restaurant scolaire et l'internat d'un horaire décalé (40, 41) | SYS | — |

### 28.3 Règles ajoutées
| # | Règle |
|---|---|
| R-EX31 | L'accès d'un surveillant aux mesures est **borné** à sa salle et à la durée de sa mission ; journalisé. |
| R-EX32 | Aucun libellé affiché (salle, liste, plan) ne révèle une mesure ou un dispositif. |
| R-EX33 | Un sujet d'épreuve non tenue n'existe nulle part ailleurs que dans le 38. |
| R-EX34 | En mode doublon, aucune convocation ni avis aux familles sans décision de la direction. |

### 28.4 Cas limites ajoutés
| # | Situation | Traitement |
|---|---|---|
| CEX-55 | Surveillant remplaçant à 7 h 45 | mesures transmises (R-EX31) |
| CEX-56 | Élève avec PAI allergie et collation pendant l'épreuve | consigne minimale (34), pause, information du responsable de salle |
| CEX-57 | Élève qui jeûne pendant une épreuve longue | information, pas d'exemption ; infirmerie prévenue si la famille le demande |
| CEX-58 | Professeur qui dépose le sujet du bac blanc dans le cahier de textes | refus (R-EX33) |

### 28.5 Questions ajoutées (*appliqué par défaut, D-031*)
- Q-EX15 : un surveillant peut-il exporter la liste de sa salle ? → **non** ; impression de la feuille d'émargement seulement.
- Q-EX16 : l'élève voit-il le nom de ses correcteurs ? → **non** avant la levée d'anonymat ; après, selon le 21.

### 28.6 Compléments de modèle (réserve 2776–2777)
- **2776** : `exa_acces_mesure (id, etablissement_id, surveillance_id, ouvert_le, ferme_le)` (R-EX31) ; `exa_sujet_alerte (id, etablissement_id, sujet_id, nature, le)` (EXA-486) ; `exa_ceremonie_tentative (id, etablissement_id, lien_id, le, issue)` (volatile).
- **2777** : `exa_session_mode (id, etablissement_id, session_id, mode ('reel','doublon'), decide_par, decide_le)` (R-EX34) ; `exa_contestation (id, etablissement_id, resultat_id, depose_par, le, issue NULL, decide_par NULL)` (EXA-480, 481).

### 28.7 Lots modifiés
EX3 reçoit EXA-462, 466, 470 ; EX5 reçoit EXA-460, 461, 464, 465, 467, 489 ; EX6 reçoit EXA-469, 471–473, 477–479, 492–495 ; EX7 reçoit EXA-463, 486 ; EX10 reçoit EXA-480, 481 ; EX16 reçoit EXA-482–485, 487 ; mode doublon (EXA-488) en **EX1**.
