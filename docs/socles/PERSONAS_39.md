# Passe « personas » sur le socle 39 — Stages, PFMP et entreprises

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** (à relire par Florian) ; les recommandations s'appliquent par défaut (D-031), sauf ce qui engage l'établissement à l'extérieur (§8.2). Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production. Le code Next.js a été **lu seulement** (rien exécuté, rien construit, aucun `.env` lu).
**Branche :** `ent-socle-39-conception`, partie de `origin/ent-mc0hoe`.
**Éprouve :** `docs/socles/39_STAGES.md` (§0 à §31), avec `CLAUDE.md`, `docs/DECISIONS.md` D-017, D-031, **D-034**, **D-035**, **D-036**, `docs/PRA_SAUVEGARDES.md` §18.4 ; forme reprise de `PERSONAS_42.md` et `PERSONAS_41.md`.
**Ajouts :** faits **directement** dans `39_STAGES.md`, au **§32** (actions STG-320 à STG-359, règles R-SG27 à R-SG34, cas limites CSG-58 à CSG-75, réglages S-3915 à S-3918, questions Q-SG34 à Q-SG40, lots révisés). Rien n'est supprimé.
**Numérotation :** manques **M-3901 à M-3972** (plage propre au socle 39). Gravité : **B** bloquant, **H** haute, **M** moyenne, **F** faible.

---

## 0. À lire en premier : quatre bloquants et leur recommandation (appliquée par défaut, D-031)

La conception tient sur le fond : la convention signée commande tout, les signataires sont calculés, le tuteur agit par lien, l'allocation se calcule au jour. Vécue par dix-huit personas, elle révèle quatre bloquants.

| # | Point | Pourquoi c'est bloquant | Recommandation (appliquée par défaut) | Lots |
|---|---|---|---|---|
| **A** | **Les PFMP de septembre se préparent en juin, sur l'année suivante.** Les terminales partent souvent dès la deuxième ou troisième semaine de septembre (P01, P02). Les conventions doivent être signées **avant l'été** ; or l'année N+1, ses classes et ses scolarisations n'existent pas encore dans le 5 et le 2 en juin. La conception crée les stages depuis une période « dans l'année du 5 » (STG-120). | Sans correctif, le DDFPT prépare 140 conventions de rentrée hors ScolIA, ou les fait toutes signer pendant la semaine de la rentrée : c'est exactement la surcharge que le socle doit supprimer, et le risque n°1 de départs sans convention. | **Périodes et stages sur l'année suivante dès juin**, rattachés à l'**inscription prévue** de l'élève (socle 3, `inscription_annuelle_prevue`) et à la formation visée, pas à la classe ; reclassés seuls à la rentrée ; un élève qui ne se réinscrit pas voit ses stages annulés avec avis aux signataires (STG-320 à STG-324, R-SG27). | ST1 |
| **B** | **Un signalement de violence contre le tuteur laisse au tuteur son lien et ses avis.** P15 (Sarah) signale un harcèlement sexuel. La conception renvoie au 36, mais le tuteur garde son lien actif, reçoit la relance « tout va bien ? » à J+2, voit l'absence de Sarah le lendemain, et peut la contacter par le fil de la convention. | L'auteur présumé est informé, en temps réel, de l'absence de la victime ; il peut faire pression. C'est un risque direct pour une mineure. | **Mise à l'abri immédiate** : au signalement protégé, **tous** les liens de l'organisme pour ce stage sont suspendus, les relances et avis vers l'organisme sont coupés, le fil est gelé ; l'organisme est informé **seulement** par un humain, avec un message neutre préparé (« le stage est suspendu ; l'établissement vous contactera ») ; l'absence de l'élève n'est plus visible du tuteur (R-SG28, STG-330 à STG-333). | ST3 |
| **C** | **Les grands organismes imposent leur propre convention ou leur outil de signature.** Une enseigne nationale (P11), un hôpital ou une collectivité signent avec leur propre plateforme, ou exigent leur convention-cadre. La conception ne connaît que la convention ScolIA et le papier. | Sans circuit prévu, le secrétariat imprime, scanne et ressaisit ; la règle « pas de convention signée » devient invérifiable, ou contournée « parce que l'entreprise l'a signée chez elle ». | **Convention de l'organisme acceptée** comme document P, à condition qu'elle porte **les mentions exigées** (liste vérifiée par le secrétariat, case par case, depuis la trame du dispositif) et **toutes** les signatures exigées ; ScolIA produit alors l'**annexe pédagogique et les horaires** contrôlés à joindre ; **convention-cadre** d'un organisme pour l'année, avec annexes individuelles par élève (STG-334 à STG-338, R-SG29). | ST1 |
| **D** | **L'attestation non rendue fait perdre à l'élève son allocation et peut-être son examen.** Le tuteur de P10 ne renvoie pas l'attestation ; l'état liquidatif doit partir **dans les deux mois** et être conforme à l'attestation (§5.5). La conception relance seulement le tuteur. | L'élève, souvent d'une famille modeste, perd 100 à 300 € pour une faute de l'organisme ; le décompte des semaines de l'examen reste incertain. | **Attestation en présence** : le référent, lors de la visite de bilan, fait signer le tuteur **sur son téléphone** (niveau G « constaté par un agent », 7 §10.2) ; relances J+3, J+10, J+30 ; **échéance ApLyPro** suivie avec alerte à J+45 ; liste « attestations manquantes » du DDFPT ; jamais d'attestation fabriquée par l'établissement (STG-339 à STG-342, R-SG30). | ST5, ST6 |

**Défauts Next.js ajoutés** (signalés à Florian, Next.js intouchable) : aucun nouveau défaut critique au-delà de DN-01 à DN-20 ; la passe confirme que **DN-01 à DN-04** rendent possibles aujourd'hui, en production, les scénarios des attaquants P17a et P17b (§4) : **F-SG1 reste la première action** de ce socle.

---

## 1. Méthode
Celle de `PERSONAS_42.md` §1 : dix-huit personas réalistes (privé sous contrat — l'établissement de Florian, lycée général, technologique et professionnel avec collège — et public : collège REP, lycée professionnel public), dont un **attaquant** (quatre visages) et un **auditeur RGPD** ; pour chacune : son année, ses irritants, ses peurs, ce qui la ravirait ; puis les **manques** (M-39xx), contradictions et cas limites, versés au §32 du 39. Vérifications ponctuelles dans le code Next.js (lecture seule) : réglages de périodes par année (`stage-periods-config.ts`), purge annuelle (`stage-auto-purge.ts`), page publique de signature (`public/sign/route.ts`).

## 2. Les personas en un tableau

| # | Persona | Contexte | Ce qu'elle éprouve |
|---|---|---|---|
| P01 | M. Garnier, DDFPT | lycée professionnel privé, 6 sections, 420 élèves, 3 périodes par niveau | le plan annuel, septembre, les semaines dues |
| P02 | Mme Lefèvre, cheffe d'établissement | même lycée, signe tout | parapheur, responsabilité, accident |
| P03 | M. Okafor, enseignant référent | électrotechnique, 14 élèves en PFMP à 40 km à la ronde | visites, tournées, mobile, absences |
| P04 | Mme Simon, professeure principale de 3e | collège public REP | élèves sans stage, familles éloignées de l'écrit |
| P05 | Mme Vidal, secrétariat des stages, et Mme Benali, CPE | lycée de Florian | volume, papier, appel, élève introuvable |
| P06 | Kévin, 15 ans, seconde bac pro électricité | travaux réglementés, habilitation électrique | aptitude, dérogation, âge |
| P07 | Inès, 17 ans puis 18 pendant sa PFMP | bac pro commercialisation, restauration le soir | âge qui change, nuit, majorité |
| P08 | Famille Morel | parents séparés, père visé par une interdiction de contact | qui signe, qui voit l'adresse |
| P09 | Yanis, 3e ULIS, et Mme Diallo, AESH | PPS, entreprise à préparer | consignes, consentement, visite |
| P10 | M. Bertin, tuteur | garage, 6 salariés, téléphone seulement | une page, un code, peu de temps |
| P11 | Mme Roux, RH d'une enseigne nationale | convention-cadre, outil de signature maison | circuit de l'organisme |
| P12 | Mme Chaber, gestionnaire (OGEC) | allocation PFMP, ApLyPro | jours, plafonds, bénéficiaire, échéances |
| P13 | Mme Ndiaye, infirmière | avis d'aptitude des mineurs | délai, données de santé |
| P14 | Lucas, seconde GT | 1élève1stage, deux lieux | import, deux conventions |
| P15 | Sarah, 16 ans, bac pro commerce | harcèlement par le tuteur | mise à l'abri |
| P16 | M. Blanc, délégué à la protection des données | auditeur RGPD | bases légales, durées, tiers |
| P17 | Les attaquants | élève, ex-conjoint, faux tuteur, personnel qui part | liens, adresses, carnet |
| P18 | Florian, administrateur | bascule de l'été 2027 | propriété des périodes, reprise |

---

## 3. Audit de parité avec Next.js (fait de code)

1. **Réglages par année scolaire** [F] : périodes, contraintes, référents et observateurs sont stockés par année dans des « jetons » de configuration (`stage-db.ts:33-45`, `stage-periods-config.ts`) ; la conception les reprend en tables (§25) mais **ne dit pas comment l'année N+1 se prépare avant son ouverture** : bloquant A.
2. **Purge annuelle** [F] : `stage-auto-purge.ts` archive à la première lecture de septembre ; une convention de rentrée préparée en juin pour l'année suivante serait **archivée** si elle était datée de l'année N : confirme A.
3. **Discussion par lien** [F] : `STAGE_DISCUSSION_LINK_ROLES` inclut le tuteur ; rien ne gèle le fil : confirme B.
4. **Dépôt papier réservé au tuteur et au RH** [F] (`stage-types.ts:527`) : il n'existe pas de convention de l'organisme comme base : confirme C.
5. **Aucune attestation** [F] : rien dans Next.js ; D est nouveau.

---

## 4. Parcours et manques, persona par persona

### P01 — M. Garnier, DDFPT, en juin et en septembre
**Son année.** En juin, il arrête les périodes de l'année suivante (terminale : 2e quinzaine de septembre ; première : novembre et mars ; seconde : janvier et mai), relance les organismes habituels, prépare les conventions de rentrée des terminales. En septembre, 140 conventions doivent être signées en dix jours. En mai, il vérifie les semaines de chaque terminale avant l'examen.
**Irritants.** Les classes de l'année suivante n'existent pas ; les élèves redoublants ou réorientés brouillent les listes ; les organismes répondent tard ; les tableurs de semaines sont faux.
**Ce qui le ravirait.** Préparer en juin sur l'année N+1 ; un plan de semaines par élève et par cycle qui se met à jour seul ; une relance des organismes de l'an dernier en un clic.
**Manques.**
- **M-3901 (B)** — préparer périodes et conventions sur l'année N+1 avant son ouverture (bloquant A).
- M-3902 (H) — plan **pluriannuel** des semaines par élève (cycle de trois ans, redoublement, réorientation de CAP en bac pro avec reprise des semaines) ; STG-125 ne voit qu'une année.
- M-3903 (H) — **positionnement réglementaire** (réduction de durée de PFMP pour un élève venant d'une autre formation) : décision saisie avec sa pièce, semaines dues recalculées.
- M-3904 (M) — relance des organismes « avez-vous une place pour un élève de première en mars ? » **par formation et par période**, avec réponse en un clic par lien (STG-144 trop vague).
- M-3905 (M) — duplication d'une période d'un niveau à l'autre avec décalage.
- M-3906 (F) — vue « carte des organismes » par formation (sans géocodage : liste par commune).

### P02 — Mme Lefèvre, cheffe d'établissement
**Son année.** Elle signe, elle répond des élèves, elle reçoit les appels quand un accident arrive.
**Irritants.** Le parapheur de septembre ; des conventions arrivées incomplètes ; ne pas savoir si l'entreprise a une dérogation pour les machines.
**Peurs.** Un mineur blessé sur une machine sans dérogation ; un élève parti sans convention.
**Manques.**
- M-3907 (H) — **délégation de signature temporaire** (adjoint pendant ses congés de fin août) visible dans le parapheur, avec date de fin ; le 6 la porte, le 39 doit la proposer au bon moment.
- M-3908 (H) — dans le parapheur, **un indicateur de risque** par convention (mineur + travaux réglementés + organisme nouveau + pas de visite prévue) pour regarder d'abord celles-là.
- M-3909 (M) — **attestation « zéro départ sans convention »** de la période, datée et signée, à produire en cas de contrôle ou d'accident.
- M-3910 (M) — en cas d'accident, une **fiche de crise** : élève, organisme, tuteur, référent, titulaires joignables, assurances, état de la déclaration.

### P03 — M. Okafor, enseignant référent
**Son année.** Quatorze élèves dans des PME électriques, à 40 km à la ronde ; deux visites chacun ; des tuteurs injoignables le matin.
**Irritants.** Organiser la tournée ; saisir le compte rendu le soir ; apprendre l'absence d'un élève trois jours après.
**Ce qui le ravirait.** Une tournée proposée sur ses heures libérées ; le compte rendu en trois minutes sur son téléphone dans la voiture, sans réseau ; l'absence du jour en notification.
**Manques.**
- M-3911 (H) — **notification immédiate** de l'absence signalée par le tuteur ou constatée (pas seulement au tableau de bord).
- M-3912 (H) — visites **hors ligne** dès ST1 pour le minimum (adresses, téléphones, horaires du jour) : sans réseau, il n'a même pas l'adresse (ST4 seulement dans la conception).
- M-3913 (M) — **frais kilométriques** : kilomètres déclarés par visite, cumul, sans géocodage.
- M-3914 (M) — visite **commune** à deux élèves du même organisme.
- M-3915 (M) — remplacement par un collègue d'une visite précise (pas de toute la mission).
- M-3916 (F) — rappel la veille de la visite au tuteur (lien, rendez-vous).

### P04 — Mme Simon, professeure principale de 3e en collège REP
**Son année.** 26 élèves, 6 qui n'ont rien trouvé en décembre ; des familles qui ne lisent pas l'écrit ou n'ont pas d'e-mail ; un seul ordinateur à la maison pour trois enfants.
**Irritants.** Les conventions papier perdues ; les élèves qui « ont trouvé chez un oncle » sans adresse ; les relances orales.
**Peurs.** L'élève qui reste à la maison la semaine du stage.
**Manques.**
- M-3917 (H) — **réseau d'offres partagé** entre établissements d'un même réseau ou d'un même bassin : les offres d'un collège favorisé ne profitent pas au collège REP voisin. Hors cloisonnement (E4) aujourd'hui : **proposé** comme publication volontaire d'une offre à d'autres établissements ScolIA, sans aucune donnée d'élève (Q-SG34).
- M-3918 (H) — **semaine de stage dans l'établissement** pour l'élève sans lieu (§5.4) : un programme type (découverte des métiers de l'établissement, CDI, ateliers), pas une simple permanence.
- M-3919 (M) — **convention dans la langue de la famille** et lecture à voix haute sur le téléphone (accessibilité) ; signature au guichet le soir de la réunion.
- M-3920 (M) — liste des démarches de l'élève (STG-157) **utile au PP** : « a contacté 4 entreprises, 0 réponse » plutôt que « sans lieu ».
- M-3921 (F) — relance des familles **par SMS** (OVH) quand l'e-mail ne sert pas.

### P05 — Mme Vidal (secrétariat) et Mme Benali (CPE)
**Leur année.** Mme Vidal reçoit les papiers, crée les organismes, relance ; Mme Benali gère l'appel et l'élève qui « devait être en stage ».
**Irritants.** Organismes en double ; conventions signées sans les horaires ; élève vu en ville un jour de stage.
**Manques.**
- M-3922 (H) — **contrôle des mentions** d'un papier ou d'une convention d'organisme case par case (bloquant C).
- M-3923 (H) — l'élève **ni en entreprise ni au lycée** : la conception lève l'alerte (STG-268) ; il manque **qui appelle qui** (procédure de la vie scolaire du 12 : famille, puis tuteur, puis référent, avec heure).
- M-3924 (M) — **numérotation** des conventions par série STG visible et recherchable par l'organisme (« votre convention STG-2027-0142 »).
- M-3925 (M) — fusion d'organismes **sans perte** des contacts et de l'historique (STG-113 à préciser).
- M-3926 (F) — modèle de courrier papier pour les organismes sans e-mail.

### P06 — Kévin, 15 ans, seconde bac pro électricité
**Son année.** Deux PFMP chez un artisan électricien ; il touchera des armoires sous tension en présence du tuteur.
**Points éprouvés.** Travaux réglementés dès 15 ans avec dérogation de l'organisme et de l'établissement, **aptitude** médicale annuelle, **habilitation électrique** et carnet de formation (art. 14 de la convention-type).
**Manques.**
- M-3927 (H) — l'**habilitation électrique** (titre, niveau, date) et le **carnet de formation** sont des exigences du stage, avec la copie de l'attestation de formation **conservée 50 ans** (§5.8) : non modélisés.
- M-3928 (H) — l'aptitude est **annuelle** : une aptitude de juin ne vaut plus en septembre de l'année suivante ; échéance à suivre par élève.
- M-3929 (M) — la **déclaration de dérogation de l'organisme** a une date et une validité de trois ans : alerte si elle expire avant la fin du stage.
- M-3930 (F) — liste « avis d'aptitude à rendre avant le … » pour l'infirmière, triée par date de départ (P13).

### P07 — Inès, 17 ans puis 18 pendant sa PFMP
**Son année.** Restaurant, service du soir. Elle a 18 ans le jeudi de la deuxième semaine.
**Points éprouvés.** Les règles changent le jeudi : nuit interdite avant, autorisée après **seulement si** elle est nommément désignée (art. 9) ; le circuit de signature d'un **avenant** après le jeudi la fait signer seule ; ses parents ne voient plus rien si elle le décide (2).
**Manques.**
- M-3931 (H) — l'**autorisation de nuit nominative** d'un élève qui devient majeur pendant le stage doit pouvoir être donnée **à l'avance**, effective à la date anniversaire.
- M-3932 (M) — l'allocation de l'élève qui devient majeur : bénéficiaire changé à la date ? La FAQ parle de l'élève majeur [V] ; recommandation : bénéficiaire apprécié **à la date de l'état liquidatif**, autorisation des titulaires devenue sans objet.
- M-3933 (M) — **transport le soir** : la convention doit dire comment elle rentre (frais, L124-13) ; champ « trajet de retour après 22 h » et alerte.

### P08 — Famille Morel, parents séparés, interdiction de contact du père
**Leur situation.** La mère a la résidence ; le père est visé par une interdiction d'entrer en contact avec l'enfant (décision connue du socle 4).
**Points éprouvés.** R-SG23 cache l'adresse de l'organisme au père ; la règle `un` fait signer la mère.
**Manques.**
- M-3934 (H) — le père garde l'**autorité parentale** (une interdiction de contact n'est pas un retrait) : le 7 l'inviterait à signer en second (« un suffit, l'autre peut signer »), et le document lui montrerait **l'adresse de l'organisme**. Il faut un **document expurgé** pour un titulaire visé par une interdiction (adresse et nom du tuteur masqués) ou ne pas l'inviter, selon la mesure (Q-SG35, juriste).
- M-3935 (M) — l'organisme doit savoir **qui ne doit pas venir chercher** ou contacter l'élève (consigne de sécurité, sans motif), comme le 14 le fait à l'accueil.
- M-3936 (F) — information de l'autre titulaire « pour information » (art. 3) : respecter les droits du lien.

### P09 — Yanis, 3e ULIS, et Mme Diallo, AESH
**Son année.** Une semaine d'observation dans une jardinerie ; Mme Diallo l'accompagne deux demi-journées.
**Manques.**
- M-3937 (H) — **présence de l'AESH** pendant le stage : elle quitte l'établissement ; son temps est planifié par le 35 et le 6 ; la convention doit la mentionner (encadrement) ; le 39 doit le proposer.
- M-3938 (M) — **préparation de l'organisme** : rendez-vous préalable (19) proposé automatiquement quand un PPS existe, avec consentement de la famille.
- M-3939 (M) — **horaires aménagés** (demi-journées) : la règle « 5 jours » de la 3e doit admettre des journées partielles sans être signalée comme anomalie.

### P10 — M. Bertin, tuteur dans un garage
**Son année.** Deux stagiaires par an ; pas d'ordinateur au garage ; il répond le soir.
**Points éprouvés.** La page unique, le code par SMS, « tout va bien », l'attestation.
**Manques.**
- M-3940 (H) — un **seul lien** pour tous ses stagiaires de l'année (deux élèves) plutôt qu'un par élève, avec portée par élève (R-SG9 dit « un par stage »).
- M-3941 (H) — **attestation sur place** lors de la visite (bloquant D).
- M-3942 (M) — grille d'évaluation **courte** (10 critères au plus, réponses en un geste) ; la grille complète reste au référent.
- M-3943 (M) — **pas de code à chaque ouverture** : code demandé pour signer et pour l'évaluation ; consultation sur appareil déjà vérifié pendant 30 jours.
- M-3944 (F) — fermeture du garage une journée : « jour non travaillé » déclarable par lui.

### P11 — Mme Roux, RH d'une enseigne nationale
**Sa situation.** Convention-cadre annuelle avec le lycée, outil de signature électronique interne, refus des liens externes par la politique de sécurité de son groupe.
**Manques.**
- M-3945 (B) — **convention de l'organisme** et **convention-cadre** (bloquant C).
- M-3946 (H) — signature par **plusieurs magasins** de l'enseigne (sites) avec un seul signataire RH.
- M-3947 (M) — **désignation des tuteurs** par magasin après la signature de la convention-cadre (annexe individuelle).

### P12 — Mme Chaber, gestionnaire (OGEC)
**Son année.** Décision d'attribution annuelle en octobre, états liquidatifs après chaque PFMP, pièces à conserver dix ans, contrôles de l'ASP.
**Manques.**
- M-3948 (H) — **décision d'attribution annuelle** (liste des élèves éligibles, bénéficiaire, pièces complètes ou non) : la conception ne prévoit que l'état par période.
- M-3949 (H) — **pièces de l'allocation** (identité, filiation, RIB, autorisation) : liste par élève « complet / manque X », relance des familles, sans exposer les pièces au référent.
- M-3950 (M) — **plafond annuel** atteint en cours d'année (rattrapage) : alerte.
- M-3951 (M) — **contrôle de l'ASP** : dossier de preuve d'un élève exportable (convention, avenants, attestation, état liquidatif) en un fichier.
- M-3952 (F) — échéance des deux mois **par période** dans le tableau de bord.

### P13 — Mme Ndiaye, infirmière
**Points éprouvés.** Liste des avis à rendre ; réponse oui / non / à revoir ; rien de médical dans le 39.
**Manques.**
- M-3953 (M) — **connaître le poste** pour rendre l'avis : la liste des travaux déclarés et des machines (convention) doit lui être lisible ; c'est une donnée du 39 transmise au 34, pas l'inverse.
- M-3954 (F) — délai de réponse visible et relance automatique avant la date de départ.

### P14 — Lucas, seconde GT, deux lieux
**Son année.** Une semaine dans une agence d'architecture, une semaine dans une mairie ; offres trouvées sur 1élève1stage, conventions signées sur la plateforme.
**Manques.**
- M-3955 (H) — **deux conventions**, deux stages, un seul « stage de seconde » pour le suivi : regroupement par **séquence** (deux lieux, une obligation).
- M-3956 (M) — import de la plateforme : la convention peut arriver **sans** signature d'un des titulaires (témoignage, §5.3) : R-SG18 couvre, mais il faut la **relance** du titulaire manquant par ScolIA.
- M-3957 (F) — dispense (service national, mobilité) enregistrée comme **sans objet**, pas « sans lieu ».

### P15 — Sarah, 16 ans, bac pro commerce : harcèlement par le tuteur
**Son parcours.** Mardi soir, elle appuie sur « j'ai un problème », choisit « violences sexistes ou sexuelles ». Le référent est prévenu, la direction aussi.
**Manques.**
- **M-3958 (B)** — mise à l'abri immédiate, liens et avis de l'organisme coupés, fil gelé (bloquant B).
- M-3959 (H) — **personne-ressource** hors heures (soir, week-end) : le référent ne lit pas ses messages à 21 h ; le signalement doit aussi atteindre la **personne d'astreinte** de l'établissement (6, comme le 41), et proposer le **numéro national** adapté.
- M-3960 (H) — **rattrapage** de la PFMP chez un autre organisme sans perte de semaines ni d'allocation pour l'élève (les jours non effectués ne sont pas de sa faute) ; jamais de mention du motif sur l'attestation ou le bulletin.
- M-3961 (M) — l'organisme **interdit** pour les autres élèves, sans motif visible hors de la direction.

### P16 — M. Blanc, délégué à la protection des données (auditeur RGPD)
**Ses questions.** Base légale ; information des tiers ; durées ; minimisation ; transferts ; registre ; analyse d'impact ; droits des personnes.
**Constats.**
- M-3962 (H) — **base légale** à écrire par traitement : mission d'intérêt public (art. 6-1-e) pour les stages, obligation légale (6-1-c) pour l'allocation et les accidents ; **mention d'information des contacts** (texte prêt, version, date d'envoi) ; la conception dit « informés au premier envoi » sans texte.
- M-3963 (H) — **ApLyPro et ASP** : la transmission est faite par l'établissement ; ScolIA n'en est que l'outil de préparation ; le registre doit le dire.
- M-3964 (M) — **analyse d'impact** : pas obligatoire a priori [H], mais les signalements de violences et les accidents sont des données sensibles : AIPD recommandée pour ST3 (Q-SG36, DPO).
- M-3965 (M) — **droits des contacts** (accès, rectification, opposition) : un formulaire par lien (le contact a un lien) plutôt qu'un e-mail au secrétariat.
- M-3966 (M) — **journal des lectures** des données sensibles (incidents, allocation) consultable par le DPO (STG-318 à préciser : quelles données).
- M-3967 (F) — la **liste « déconseillé »** est une donnée sur une personne morale mais peut viser indirectement une personne (petit artisan seul) : motif codé, jamais transmis, durée limitée (3 ans puis revue).

### P17 — Les attaquants
- **P17a — L'élève qui veut signer à la place de ses parents.** Next.js le permet aujourd'hui (DN-01, DN-07). Dans ScolIA : aucun jeton dans une projection, signataires calculés, code au canal vérifié du titulaire. **Reste** : l'élève qui connaît le mot de passe de la messagerie de sa mère reçoit le code. M-3968 (M) — **S2 par passkey** pour les titulaires qui en ont une, et avis « une signature a été faite en votre nom » sur un **second canal** (SMS si l'e-mail a servi).
- **P17b — L'ex-conjoint qui cherche l'adresse de l'enfant.** Bloqué par R-SG23 ; reste M-3934 (le document de la convention).
- **P17c — Le faux tuteur** qui écrit au lycée « je remplace M. Bertin, envoyez-moi le lien ». Bloqué : le changement de contact est validé par l'établissement, l'ancien lien révoqué. M-3969 (M) — **vérification par rappel** de l'organisme (numéro du carnet, pas celui du message) inscrite dans la procédure de STG-116.
- **P17d — Le personnel qui part chez un concurrent avec le carnet d'organismes.** M-3970 (M) — **export des contacts** limité aux rôles habilités, journalisé, plafonné, alerte au-delà de 200 lignes ; pas d'export du carnet par un référent.

### P18 — Florian, l'été 2027
**Son problème.** Next.js gère encore les PFMP de 2026-2027 ; ScolIA doit prendre celles de 2027-2028, préparées en juin 2027 (bloquant A), pendant l'année de doublon (D-034) où l'allocation reste dans ApLyPro.
**Manques.**
- M-3971 (H) — **date de bascule par période et par dispositif** : les PFMP de septembre 2027 naissent dans ScolIA dès juin 2027 ; les stages de seconde de juin 2027 finissent dans Next.js (R-SG26 complété).
- M-3972 (M) — **organismes et contacts repris en premier** (avant les conventions) pour que la préparation de juin parte d'un carnet complet.

### Manques transverses : l'IA, le mobile, les tests
L'IA ne doit pas voir le motif d'une suspension (B) ; l'application mobile de l'élève doit fonctionner hors ligne pour les horaires et contacts (déjà STG-258) ; un test « signalement protégé → aucun avis vers l'organisme » est ajouté au §21.4.

---

## 5. Vérification réglementaire (2026-11-11)
Faite par la conception (§5 du 39) sur sources officielles ; la passe n'a pas ajouté de recherche. Points à confirmer par un juriste, nés de la passe : document expurgé pour un titulaire visé par une interdiction (M-3934) ; validité d'une convention de l'organisme qui ne reprend pas le modèle national (M-3945) ; bénéficiaire de l'allocation quand l'élève devient majeur (M-3932) ; partage d'offres entre établissements (M-3917, RGPD : aucune donnée d'élève).

## 6. Manques qui relèvent d'autres socles
| Socle | Manque |
|---|---|
| 3 | inscription prévue de l'année N+1 consultable en juin par le 39 (bloquant A) |
| 4 | interdiction de contact distincte du retrait de l'autorité parentale, lisible par le 7 pour expurger (M-3934) |
| 6 | délégation de signature temporaire (M-3907) ; astreinte (M-3959) ; renommage de la mission (Q-SG26) |
| 7 | niveau G « constaté par un agent » sur mobile (M-3941) ; document expurgé par destinataire (M-3934) ; convention-cadre et annexes individuelles (M-3945) |
| 12 | procédure « élève introuvable » pour les stages (M-3923) |
| 34 | aptitude annuelle avec échéance (M-3928) ; lecture des travaux déclarés (M-3953) |
| 35 | présence de l'AESH hors de l'établissement (M-3937) |
| 36 | réception d'un signalement venant du 39 (bloquant B) |

## 7. Synthèse

### 7.1 Manques par gravité
| Gravité | Nombre |
|---|---|
| B | 3 (M-3901, M-3945, M-3958) ; le bloquant D porte sur M-3941, classé H par manque mais bloquant par son effet sur l'allocation et l'examen |
| H | 26 |
| M | 32 |
| F | 11 |
| **Total** | **72** (M-3901 à M-3972) |

### 7.2 Ajouts dans la conception (`39_STAGES.md` §32)
Actions STG-320 à STG-359 (40) ; règles R-SG27 à R-SG34 (8) ; cas limites CSG-58 à CSG-75 (18) ; réglages S-3915 à S-3918 (4) ; questions Q-SG34 à Q-SG40 ; lots révisés (ST1 élargi, ST3 avancé).

### 7.3 Les dix ajouts les plus importants
1. Préparation en juin sur l'année N+1 (A).
2. Mise à l'abri sur signalement de violences (B).
3. Convention de l'organisme et convention-cadre (C).
4. Attestation en présence et suivi des échéances de l'allocation (D).
5. Plan pluriannuel des semaines et positionnement réglementaire.
6. Habilitation électrique, aptitude annuelle, validité des dérogations.
7. Document expurgé pour un titulaire visé par une interdiction.
8. Décision d'attribution annuelle et pièces de l'allocation.
9. Un lien par tuteur et par année, code seulement pour signer.
10. Signature faite en votre nom : avis sur un second canal.

### 7.4 Ce que la passe a montré sur la conception
La conception est solide sur la sécurité des liens et la règle « pas de convention, pas de départ » ; elle était **trop centrée sur une année scolaire** (A), **trop confiante envers l'organisme** une fois le stage commencé (B) et **trop attachée à son propre modèle de convention** (C). Elle sous-estimait l'administratif de l'allocation (P12).

## 8. Décisions : appliquées par défaut (D-031) et soumises à Florian

### 8.1 Appliquées par défaut (D-031)
1. A, B, C, D comme au §0.
2. Lien unique par tuteur et par année scolaire, portée par élève (M-3940).
3. Code demandé pour signer et évaluer, pas pour consulter sur un appareil vérifié (M-3943).
4. Bénéficiaire de l'allocation apprécié à la date de l'état liquidatif (M-3932).
5. Visites minimales hors ligne dès ST1 (M-3912).
6. Habilitation électrique et aptitude annuelle comme exigences (M-3927, M-3928).
7. Export des contacts restreint et plafonné (M-3970).
8. AIPD recommandée pour ST3 (M-3964), à confirmer par le DPO.

### 8.2 Soumis à Florian
| N° | Sujet | Pourquoi |
|---|---|---|
| F-SG1 | Failles DN-01 à DN-04 en production | Next.js intouchable pour le pilote |
| Q-SG34 | Partage d'offres entre établissements ScolIA | change le cloisonnement (E4), engage les établissements |
| Q-SG35 | Titulaire visé par une interdiction : document expurgé ou pas d'invitation | juriste |
| Q-SG36 | AIPD pour les incidents et signalements | DPO |
| Q-SG37 | Convention-cadre d'une enseigne : modèle accepté | juriste, établissement |
| Q-SG38 | Date de bascule par période (été 2027) | décision de Florian |
| Q-SG39 | Numéro national à afficher sur le signalement de violences | choix de l'établissement |
| Q-SG40 | Kilomètres des visites : qui rembourse, sur quel barème | organisme de gestion |

## 8 bis. Réglages signalés (D-023)
S-3915 durée de validité d'un appareil vérifié du tuteur (30 jours) ; S-3916 relances d'attestation (J+3, J+10, J+30) ; S-3917 alerte d'échéance ApLyPro (J+45) ; S-3918 semaine d'accueil interne type pour les élèves sans lieu.

## 9. Faits, hypothèses, contradictions
- **Faits** : Next.js stocke ses réglages de stages par année et purge à la rentrée [F] ; la discussion par lien inclut le tuteur [F] ; aucune attestation n'existe [F].
- **Hypothèses** : les terminales partent dès septembre dans l'établissement de Florian [H] ; les grandes enseignes imposent leur outil de signature [H, pratique courante].
- **Contradictions** : R-SG9 (« un lien par stage ») contre M-3940 (un lien par tuteur et par année) : **M-3940 l'emporte**, la portée reste par élève ; STG-120 (« dans l'année du 5 ») contre le bloquant A : **A l'emporte** (R-SG27).
