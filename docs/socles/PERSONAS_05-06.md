# Passe « personas » sur les socles 5 et 6

**Date :** 2026-11-07 — **Statut :** proposition, **non décidée** (à relire par Florian). Aucun code écrit ni modifié ; aucun accès à la base ni aux secrets de production ; l'application Next.js n'a pas été lue pour ce document.
**Branche :** `ent-personas-socles-5-6`.
**Éprouve :** `docs/socles/05_STRUCTURE.md` et `06_PERSONNELS.md` (lus en entier), avec `VISION.md`, `DECISIONS.md` (D-017, D-019 à D-025), `SOCLES.md`, `MATRICE_CHARLEMAGNE.md`, `PERSONAS_02-04.md` (méthode et format repris), `MIGRATIONS.md`.
**Ajouts :** faits **directement** dans les deux conceptions, marqués **[personas]** : actions `STR-93` à `STR-114` et `PRS-116` à `PRS-147`, règles `R-S1` à `R-S24` et `R-P1` à `R-P26`, cas limites (tableau §4.5 du socle 5, cas `P47` à `P60` du socle 6), compléments de modèle (§9.10 et §8.14), lots, questions `Q-S20` à `Q-S26` et `Q-P28` à `Q-P35`. Rien n'a été supprimé : quand un ajout contredit le texte d'origine, une note **[personas]** est posée à côté.
**Numérotation des manques :** elle continue celle de `PERSONAS_02-04.md` (qui s'arrête à M-138) : **M-139 à M-236**. Les réglages signalés continuent aussi : **S-24 à S-43**.

---

## 0. À lire en premier : ce qu'il faut trancher avant le premier lot de code

Aucun lot des socles 5 et 6 n'est commencé (conceptions seulement, `PROJECT_STATUS.md`). Les deux conceptions ont été écrites **en parallèle**, le socle 6 sans avoir pu lire le socle 5 (`06_PERSONNELS.md` §5.4, §20). En les faisant vivre ensemble par les mêmes personnes, six points apparaissent qui changent les **premiers** lots. Ils sont à trancher avant S0/S1 (socle 5) et P0/P1 (socle 6).

| # | Point | Pourquoi | Ce que le lot doit faire | Gravité | Lots touchés |
|---|---|---|---|---|---|
| A | **Un seul objet pour « ce qui est enseigné à qui ».** Le socle 5 l'appelle **cours** (`ent_cours`, il le possède, le « service » est la présence d'une personne) ; le socle 6 l'appelle **service d'enseignement** (`ent_service_enseignement`, il le possède, la présence est une « intervention ») | Deux tables, deux propriétaires, deux commandes de création (STR-56/57 et PRS-40), deux reprises de `ent_service_enseignement` (05 §17 M7, 06 §17 n° 354) pour la même chose. L'adjoint qui prépare la rentrée (P02) ne saurait pas où créer « Espagnol 4AB » | **Q-S20 / Q-P28** : une seule table. Recommandation : l'existant `ent_service_enseignement`, enrichi, **devient le cours** ; il est **créé et structuré par le socle 5** (public, volume, barrettes, génération depuis la maquette) ; le socle 6 possède les **interventions** (titulaire, co-enseignant, suppléant) et les heures dues. PRS-40 appelle STR-57 | **bloquant** | S4, P3 |
| B | **Une seule fonction « qui voit quels élèves ».** Le socle 5 annonce `structure::perimetre` comme « seule implémentation » (05 §6.3, §8.3) ; le socle 6 annonce `services::eleves_vus` qui « remplace `eleves_suivis_par_prof` » (06 §5.3, §9.1) | Deux calculs de D-022 finiront par diverger : un professeur verra un élève sur un écran et pas sur un autre, ou pire, l'inverse sur une route d'export | **Q-S21** : une seule fonction (au socle 5, qui lit les interventions du socle 6), un seul jeu de tests D-022 | **bloquant** | S1, P4 |
| C | **Engagements simultanés.** Le socle 6 interdit deux engagements qui se chevauchent pour un même agent (contrainte d'exclusion, 06 §8.2) | Dans le privé, un maître payé par l'État fait aussi des surveillances payées par l'OGEC ; un AED est aussi vacataire d'étude ; un agent de la collectivité fait des heures pour l'OGEC [H, fréquent]. Le comptable (P09) ne peut pas l'enregistrer | **R-P1** : l'exclusion porte sur (agent, employeur, nature), pas sur l'agent seul. À changer **dans la migration 352** | **bloquant** | P1 |
| D | **Fuseau horaire dans le socle 6.** Le socle 6 écrit « dates métier en Europe/Paris » partout (§7, §8, lot P0 « dates en Europe/Paris dans `perimetre.rs` ») ; le socle 5 met le fuseau au profil d'établissement (STR-03) ; la passe 2–4 a déjà corrigé ce point pour les socles 2 à 4 (A0-a) | À La Réunion (P17), un accès qui se ferme « le soir de la date de fin » se fermerait à 2 h du matin heure locale ; une suppléance commencerait à 2 h la veille | **R-P2** : lot P0 → « dates dans le fuseau de l'établissement (profil du socle 5), Europe/Paris par défaut » | important | P0 |
| E | **Élève inclus dans un cours d'une autre classe.** Le public d'un cours est une classe **ou** un groupe, rien d'autre (05 §9.7) | Un élève d'ULIS qui suit les mathématiques en 5B n'est sur **aucune** liste d'appel de ce cours : il peut être absent sans que personne ne le signale (P20). Même chose pour le décloisonnement à l'école, l'élève en avance, le parcours aménagé | **R-S5** : membres individuels datés d'un cours (inclusion, exclusion), lus par `public_du_cours` et par la règle de visibilité. À prévoir **dans le modèle du lot S4** | **bloquant** | S4, S6 |
| F | **Noms d'événements divergents.** Le socle 6 consomme `annee.ouverte`, `classe.archivee` ; le socle 5 émet `referentiel.annee_ouverte`, `referentiel.classe_archivee` | Un consommateur abonné au mauvais nom ne reçoit jamais rien, sans erreur | **R-S23 / R-P26** : les noms du socle 5 font foi ; corriger les tableaux du socle 6 au lot P1 | confort | P1 |

---

## 1. Méthode

**Démarche.** Celle de `PERSONAS_02-04.md` §1 et §6 : 24 personas réalistes et variées, choisies dans les tableaux « acteurs » des deux socles et complétées pour couvrir public, privé sous contrat, hors contrat ; école, collège, lycée général, lycée professionnel ; métropole et outre-mer ; direction, bureau, terrain ; nouveaux venus et personnels à deux casquettes ; un élève qui change d'option ; un contrôleur (inspection). Pour chacune : son année **mois par mois** (préparation de rentrée, rentrée, changements en cours d'année, conseils, examens, bascule), puis ses **incidents** (erreur, contestation, panne, situation de crise). Chaque fois qu'elle voulait faire quelque chose que les conceptions ne prévoient pas, ou que c'était trop long, ambigu, risqué ou frustrant, le manque est noté.

**Grille transversale** (posée à chaque persona, `PERSONAS_02-04.md` §6 point 4) : décès ; danger et protection ; panne et hors ligne ; fuseau ; langue et accessibilité ; conflit d'intérêts ; fermeture (vacances, neige, grève) ; erreur et annulation ; contestation ; fraude ; plusieurs établissements (D-013) ; reprise en pleine saison ; « qui le voit ? ».

**Couverture Charlemagne** (D-023, D-024) : chaque ligne de `MATRICE_CHARLEMAGNE.md` portée par les socles 5 et 6 (STR-01 à STR-14, PER-01 à PER-20, EDT-15, EDT-17, SAN-17) a été relue ; celles qui restaient « TROU » ou « carte » sont traitées au §4.

**Gravité** (identique à la passe 2–4).
- **bloquant** : sans lui, l'établissement contourne de façon dangereuse (sécurité d'un enfant, fuite d'une donnée, pratique probablement illégale) ou ne peut pas saisir un cas réel ; ou deux conceptions se contredisent au point d'empêcher de coder.
- **important** : cas fréquent sans solution propre, perte de temps forte ou risque d'erreur ; ou fonction de Charlemagne absente.
- **confort** : améliore nettement le quotidien ; un contournement existe.
- **différenciant** : peu de logiciels du marché le font [H, comparaison de mémoire] ; argument de vente.

**Conventions.** [H] hypothèse ; **[NV, à vérifier]** règle réglementaire citée de mémoire, à vérifier sur un texte officiel avant de coder ; [R] recommandation. Numéros de socles : `SOCLES.md`.

**Limites.** Les personas sont construites, pas interviewées. Les règles statutaires (dotation, services des enseignants, IMP, Pacte, PV d'installation, contrôle des établissements hors contrat, honorabilité, registre du personnel…) sont citées **de mémoire** : toutes marquées [NV]. Aucune fonction de Charlemagne n'a pu être vérifiée au détail (même limite que la matrice, §0).

---

## 2. Les personas en un tableau

| # | Persona | Établissement | Ce qu'elle éprouve surtout |
|---|---|---|---|
| P01 | **Anne**, cheffe d'établissement coordinatrice | Ensemble scolaire privé sous contrat : école (200) et collège (650) | Structure prévisionnelle, constitution des 6ᵉ, rentrée, constat d'effectifs |
| P02 | **Laurent**, proviseur adjoint | Lycée public général et technologique, 1 400 élèves | Dotation, spécialités et barrettes, services, export vers l'emploi du temps |
| P03 | **Martine**, secrétaire de scolarité (utilisatrice de Charlemagne depuis 15 ans) | Lycée privé sous contrat, 900 élèves, passage à ScolIA en janvier | Reprise en cours d'année, listes, bascule, rentrée |
| P04 | **Thomas**, professeur principal de 2ᵈᵉ | Lycée public | Mouvements de sa classe, changements d'option, équipe |
| P05 | **Julie**, contractuelle de SVT arrivée le 13 novembre sur un poste vacant | Collège public | Premier jour, droits, renouvellements de contrat |
| P06 | **Karim**, professeur de musique contractuel sur deux collèges ScolIA | Deux collèges publics | Deux emplois du temps, perte de passkey le jour de la rentrée |
| P07 | **Mathis**, AED étudiant ; **Léna**, AED et étudiante en BTS du même lycée | Lycée avec internat et BTS | Planning de service, conflit d'intérêts |
| P08 | **Fatou**, agent d'accueil (employée de la région) | Lycée public | « Qui est de service ? », visiteurs, salles |
| P09 | **Bernard**, comptable de l'OGEC | Ensemble scolaire privé (celui de P01) | Personnels de droit privé, doubles employeurs, service fait |
| P10 | **Mme Roussel**, inspectrice pédagogique ; **M. Garnier**, inspecteur chargé du contrôle d'un hors contrat | Lycée public ; établissement hors contrat | Accès temporaire, dossier de contrôle |
| P11 | **Chloé**, qui quitte le latin à la Toussaint puis change de spécialité en 1ʳᵉ ; **Hugo**, réorienté en voie professionnelle en novembre | Lycée polyvalent public | Changements d'enseignements, réorientation, échange international |
| P12 | **Nathalie**, CPE | Lycée avec internat, 180 internes | Neige, internat, accueil sans cours |
| P13 | **Sophie**, professeure d'EPS, coordinatrice de l'association sportive | Collège public | Gymnase municipal, piscine, capacités |
| P14 | **M. Benali**, directeur délégué aux formations | Lycée professionnel | Périodes en entreprise, apprentis, ateliers, habilitations |
| P15 | **Agnès**, directrice de l'école de l'ensemble | École privée sous contrat (celle de P01) | Classes à plusieurs niveaux, décloisonnement, décharge |
| P16 | **Olivier**, directeur | Établissement hors contrat international, 300 élèves | Calendrier propre, vacataires à l'heure, contrôle |
| P17 | **M. Payet**, proviseur | Lycée public à La Réunion | Calendrier austral, alerte cyclonique, fuseau |
| P18 | **Corinne**, secrétaire de direction | Collège public | Prises de fonctions, attestations, départs, organisation |
| P19 | **Élodie**, titulaire sur zone de remplacement | Trois établissements ScolIA dans l'année | Suppléances successives et prolongées |
| P20 | **Samia**, AESH | Trois élèves dans deux établissements | Élève d'ULIS inclus, changements de groupes |
| P21 | **Mme Perrin**, adjointe gestionnaire | Lycée public | Agents, vacances, examens, salles |
| P22 | **Famille Lefèvre** | Deux enfants dans une cité scolaire (collège et lycée) | Calendriers, rentrée, équipe |
| P23 | **Yann**, référent numérique et administrateur | Collège privé | Revue des accès, accès temporaires, postes partagés |
| P24 | **Les représentants du personnel de l'OGEC** et **un professeur** qui exerce son droit d'accès | Ensemble privé (celui de P01) | Les personnels comme personnes protégées |

---

## 3. Parcours et manques, persona par persona

Chaque tableau donne : le manque, pourquoi il compte, sa gravité, le socle concerné, et l'ajout fait (identifiant dans la conception) ou le socle ultérieur où il est renvoyé.

### P01 — Anne, cheffe d'établissement coordinatrice (ensemble privé sous contrat)

**Année.** *Janvier* : la dotation des maîtres arrive de l'académie [NV : circuit exact dans le privé sous contrat, via la direction diocésaine] ; elle bâtit trois structures possibles pour la rentrée (5 ou 6 classes de 6ᵉ, avec ou sans dédoublement des sciences). *Février* : présentation au conseil d'établissement et à l'OGEC (budget). *Mars–avril* : réinscriptions ; elle demande aux professeurs leurs **vœux de service** (niveaux, options, heures supplémentaires) sur une feuille papier. *Juin* : commission de liaison école–collège avec les professeurs de CM2 de sa propre école **et de douze écoles extérieures** ; constitution des 6ᵉ ; validation le 28 juin. *Juillet* : neuf désistements, six arrivées. *Fin août* : rééquilibrage, ouverture de l'année, pré-rentrée. *Septembre* : constat d'effectifs, dont dépend le forfait versé par la commune et le département [NV] ; vingt familles demandent un changement de classe. *Novembre* : un premier conseil pédagogique, réservé aux personnels, doit apparaître dans le calendrier des professeurs mais pas des familles.
**Incidents.** Elle ouvre l'année N+1 le 25 août au lieu du 29 : les familles reçoivent la classe trop tôt, et elle voudrait **revenir en arrière**. En juillet, elle apprend qu'un élève fait l'objet d'une **interdiction de contact** avec un autre élève du niveau : la constitution les avait mis ensemble. Une mère de jumeaux appelle : « surtout séparez-les ».

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-139 | Enregistrer plusieurs **scénarios de structure prévisionnelle** (nombre de classes, dédoublements, options), les comparer en heures, en retenir un, le présenter | La simulation « sans écriture » (STR-45) ne garde rien : chaque présentation au conseil se refait dans un tableur | important | 5 | STR-98 |
| M-140 | Recueillir les **vœux de service** des enseignants pour N+1 | Étape réelle d'avril–mai, faite sur papier ; sans elle, la répartition de juin part de zéro | important | 6 | PRS-127 |
| M-141 | **Avis des enseignants des écoles d'origine** pour la constitution des 6ᵉ (liaison école–collège) | Ce sont eux qui connaissent les élèves ; ils ne sont pas personnels du collège. Sans accès invité ni import, l'avis arrive par papier et n'entre pas dans le calcul | important | 5 | STR-107, R-S17 |
| M-142 | **Jumeaux et fratries du même niveau** : choix de la famille (ensemble, séparés, indifférent) recueilli même quand les souhaits des familles sont désactivés | Demande quasi systématique des familles [H] ; sans elle, la direction note au crayon | confort | 5 | R-S8 |
| M-143 | Une **mesure d'éloignement** (interdiction de contact, mesure éducative, protection) devient une contrainte « séparer » dure, sans motif visible | Oublier la mesure met une victime et son agresseur dans la même classe ; la contrainte manuelle dépend de la mémoire de quelqu'un (renvoi M-035 de la passe 2–4) | important | 5 (+13, 36) | R-S7 |
| M-144 | **Rouvrir une campagne validée** pour intégrer les arrivées et départs d'été, sans recalculer les élèves déjà placés | Neuf désistements et six arrivées déséquilibrent les classes ; refaire la constitution casserait des répartitions déjà annoncées | important | 5 | STR-106, R-S18 |
| M-145 | Publier aux familles, **avant l'ouverture**, les informations de rentrée de N+1 : date et heure de rentrée du niveau, classe si publiée, professeur principal si nommé, calendrier | Constat G13 de la matrice : jusqu'à la bascule, les familles ne voient que l'année finie, sans explication | important | 5 (+29) | STR-96 |
| M-146 | **Annuler une ouverture d'année** faite par erreur ou trop tôt (retour arrière complet) | Trou T-31 de la matrice ; la réouverture d'une année close ne couvre pas ce cas | important | 5 | STR-95, R-S3 |
| M-147 | **Arrêter un constat d'effectifs** à une date : chiffres figés, signés, comparables, exportables | Les financements (forfait communal et départemental dans le privé [NV]), les enquêtes et la DDEC demandent des chiffres « à la date de », qui ne doivent plus bouger ensuite | important | 5 (+47, 9) | STR-97, R-S4 |
| M-148 | Instruire les **demandes de changement de classe** des familles après la publication | Vingt demandes en septembre [H], traitées par courriels sans trace ni réponse homogène | confort | 5 | STR-108 |
| M-149 | Événements du calendrier **réservés aux personnels** (conseil pédagogique, journée de formation, réunion de pré-rentrée) | Le calendrier ne connaît que « publié aux familles ou non » : les professeurs n'ont pas de calendrier interne | confort | 5 | R-S19 |

### P02 — Laurent, proviseur adjoint (lycée public général et technologique)

**Année.** *Décembre–janvier* : dotation horaire globale [NV] ; tableau de répartition des moyens par discipline présenté au conseil d'administration [NV]. *Février–mars* : les élèves de 2ᵈᵉ choisissent leurs spécialités ; il compte les **combinaisons** (doublettes en terminale, triplettes en 1ʳᵉ) pour construire les **barrettes** : si « SES + HLP + NSI » concerne 14 élèves et que HLP et NSI sont sur la même barrette, ces élèves sont bloqués. *Avril–mai* : les coordinateurs de discipline proposent la répartition de leur discipline ; vœux d'emploi du temps des professeurs. *Juin* : export vers le logiciel d'emploi du temps. *Juillet* : il veut que chaque professeur voie ses classes de l'an prochain pour préparer l'été. *Septembre* : chaque enseignant signe son **état de ventilation de service** [NV] ; heures supplémentaires années ; missions particulières (référent numérique, coordination) et missions du Pacte [NV]. *Octobre* : le groupe d'espagnol de 1ʳᵉ compte 36 élèves, il faut le **dédoubler**.
**Incidents.** Un élève est resté **trois semaines dans le mauvais groupe** de spécialité : les appels ont été faits, deux notes saisies. En construisant les cours de N+1, il ne sait pas s'il doit passer par l'écran « structure » ou l'écran « services ».

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-150 | **Un seul objet** pour « ce qui est enseigné à qui » (cours du socle 5 = service d'enseignement du socle 6) | §0 point A : deux tables, deux propriétaires, deux commandes pour la même chose | **bloquant** | 5, 6 | Q-S20, Q-P28 |
| M-151 | **Une seule fonction** de périmètre D-022 | §0 point B | **bloquant** | 5, 6 (+1) | Q-S21 |
| M-152 | Tableau des **combinaisons de choix** (doublettes, triplettes, LV × options) et détection des **incompatibilités avec les barrettes**, avec simulation | C'est le travail le plus délicat de mars en lycée [H] ; aujourd'hui dans un tableur, avec des erreurs découvertes à la rentrée | différenciant | 5 | STR-99 |
| M-153 | **Dédoubler** (ou fusionner) un groupe, avec cours dupliqué et service à pourvoir | Cas le plus fréquent de septembre ; STR-49 ne traite que les classes | important | 5 | STR-103 |
| M-154 | **Corriger rétroactivement** une affectation erronée, avec aperçu des appels et notes concernés | Ni un changement (qui préviendrait les familles), ni une suppression (interdite) ; les absences saisies dans le mauvais groupe doivent être revues | important | 5 (+12, 21) | STR-104, R-S6 |
| M-155 | **Composantes du service dû** : obligation de service, décharges, pondérations, heures supplémentaires année = attribuées − dues [NV] | Q-P22 prévoit des « heures dues » sans les composer ; l'écart affiché serait faux pour la moitié des professeurs de lycée [H] | important | 6 | PRS-130 |
| M-156 | Éditer et faire signer l'**état de service** de chaque enseignant (trame), avec réserves possibles [NV] | Document annuel signé dans le public (et pour les maîtres du privé [H]) ; Charlemagne et les logiciels d'EDT le produisent [H] | important | 6 (+7) | PRS-131 |
| M-157 | Attacher à une mission une **rémunération indicative** (indemnité pour mission particulière, part modulable du PP, Pacte, heures supplémentaires effectives, rémunération OGEC) et un volume, **sans montant** [NV] | Sans cela, l'état des missions rémunérées se tient à part, et la paie (49) n'a pas sa source | important | 6 (+49) | PRS-132, R-P11 |
| M-158 | Le **coordinateur de discipline** propose la répartition de sa discipline ; la direction arbitre | Pratique courante [H] ; aujourd'hui des courriels et un tableur par discipline | confort | 6 | PRS-128 |
| M-159 | **Publier la répartition prévisionnelle** aux enseignants (leurs cours de N+1, sans élèves) à une date choisie | Les professeurs préparent l'été sur leurs futures classes ; PRS-52 n'ouvre qu'à la rentrée | important | 6 | PRS-129, R-P17 |
| M-160 | Contrôle de rentrée : **effectif d'un cours supérieur à la capacité** de la salle prévue | Découvert le jour de la rentrée, quand 34 élèves arrivent dans un laboratoire de 24 places | confort | 5 (+10) | R-S20 |

### P03 — Martine, secrétaire de scolarité (lycée privé sous contrat, sous Charlemagne)

**Année.** *Décembre* : le lycée signe avec ScolIA ; bascule prévue au retour des vacances d'hiver. *Janvier* : reprise ; elle exige que « dans quelle classe et quel groupe était chaque élève depuis septembre » soit juste, sinon les professeurs perdront la trace des élèves partis en octobre. *Mars* : export des structures vers la base nationale ; les codes de division de la base nationale ne sont pas les libellés qu'elle affiche (« 2NDE3 » contre « 2ᵈᵉ Arc-en-ciel ») [NV : longueur et format]. *Juin* : décisions de passage. *Juillet* : la commission d'appel fait passer un élève en 1ʳᵉ alors qu'il était prévu en redoublement : il est déjà dans une classe de 2ᵈᵉ de N+1. *Fin juillet* : fermeture du secrétariat trois semaines. *Rentrée* : listes à afficher sous le préau, listes d'appel papier « au cas où », étiquettes pour les casiers, trombinoscopes pour les professeurs. Toute l'année : des listes « à la demande » (les demi-pensionnaires latinistes de 2ᵈᵉ, les élèves de 1ʳᵉ sans LV2), qu'elle obtient aujourd'hui avec les « éditions » de Charlemagne [H : nom et contenu exacts à vérifier].
**Incidents.** Le matin de la rentrée, le Wi-Fi tombe : il faut les listes papier dans dix minutes. La liste affichée l'an dernier mentionnait « ULIS » à côté de deux noms.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-161 | **Reprise en cours d'année** de la structure et des **affectations datées depuis la rentrée** (pas seulement l'état du jour), options comprises | Sans les dates, la règle « le professeur voit l'élève pour les dates où il l'avait » (Q-S6) ne fonctionne pas pour le premier semestre ; les élèves partis disparaissent des bulletins | important | 5 (+28) | STR-113, R-S13 |
| M-162 | Imprimer ou exporter les **listes** par classe, groupe, option, langue, régime, selon **quatre trames** : liste simple, émargement, trombinoscope, étiquettes ; plus la liste d'affichage de rentrée | Fonction de base de Charlemagne [H] ; indispensable le jour d'une panne | important | 5 | STR-111 |
| M-163 | **Liste à la demande** : filtres de structure, colonnes choisies dans un catalogue court, favoris ; export réservé aux administratifs | Les secrétariats vivent de ces listes ; sans elles, export complet puis tableur (constat G10 de la matrice) | important | 5 (+2) | STR-112 (réglage signalé S-43) |
| M-164 | **Liste d'affichage de rentrée** sans donnée sensible : prénom et initiale du nom, aucune mention de dispositif (ULIS, UPE2A), aucune date de naissance | Une liste sur un mur est lue par tout le monde ; une mention « ULIS » révèle un handicap | important | 5 | R-S9 |
| M-165 | **Code national de la division** distinct du libellé affiché, contrôlé avant l'export | Ressaisie et rejets à chaque export (constat G6) | important | 5 (+9) | R-S10 |
| M-166 | **Réviser la montée** après une décision d'appel ou une commission de juillet | L'élève est déjà placé dans une classe de N+1 qui ne lui correspond plus ; rien ne le signale | important | 5 (+23) | STR-105 |
| M-167 | **Jours d'ouverture des services** (secrétariat, intendance, restauration, internat, accueil), distincts des jours de cours, lus par les délais des autres socles | Renvoi M-005 de la passe 2–4 : un délai qui expire le 10 août quand le secrétariat est fermé | important | 5 (+3, 24) | STR-93, R-S2 |
| M-168 | **Dates des examens nationaux** (écrits du brevet, du baccalauréat) adoptées dans le calendrier, comme les vacances | Chaque année ressaisies ; les fins de cours avancées en dépendent | confort | 5 (+38) | STR-114 |

### P04 — Thomas, professeur principal de 2ᵈᵉ (lycée public)

**Année.** *Septembre* : trois élèves demandent à changer d'option (latin, section européenne) ; il veut donner son avis et savoir quand c'est décidé. *Novembre* : l'infirmière intervient deux heures dans sa classe (éducation à la sexualité [NV]) ; elle doit avoir la liste du jour, et il veut la voir dans « l'équipe de la classe » ce jour-là. *Décembre* : conseil ; deux élèves ont changé de classe, dont un pour des raisons de protection ; la famille d'accueil ne doit pas le savoir. *Mars* : il part trois semaines en congé de paternité ; sa collègue co-PP reprend.
**Incidents.** Les familles lui demandent dès le 2 septembre « qui est le PP ? », alors que la nomination n'est publiée nulle part.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-169 | **Demande de changement d'enseignement** (option, spécialité, langue) déposée par la famille, l'élève majeur ou le PP, avec avis, décision, effet daté, réponse | STR-62 est une saisie du secrétariat : la demande, l'avis du PP et la réponse à la famille n'ont pas de place | important | 5 (+18) | STR-100 |
| M-170 | **Changement de classe pour protection** : motif codé confidentiel, notifications sans motif, aucune information de la classe d'origine sur la destination | Une notification « Léo rejoint la 2ᵈᵉ 4 » envoyée à toute l'équipe d'origine peut suffire à retrouver un élève protégé | important | 5 | R-S12 |
| M-171 | **Intervention ponctuelle** d'un agent ou d'un partenaire sur des séances, sans service annuel, avec accès limité à ces séances | Infirmière, psychologue, partenaires de prévention : fréquent [H] ; aujourd'hui ils n'existent nulle part, ou reçoivent trop de droits | important | 6 (+10) | PRS-124 |
| M-172 | Le **professeur principal visible des familles** dès la publication des classes de N+1 | Sinon les familles écrivent au secrétariat pour le demander | confort | 6 | R-P15 |

### P05 — Julie, contractuelle de SVT arrivée le 13 novembre sur un poste vacant (collège public)

**Parcours.** *Lundi 10* : le rectorat la nomme [NV]. *Mercredi 12* : la secrétaire l'invite. *Jeudi 13* : elle arrive à 8 h, sans adresse académique (créée deux semaines plus tard [H]), sans clé de sécurité, avec un téléphone ancien. Premier cours à 10 h. Le titulaire du poste n'existe pas (poste vacant depuis la rentrée) : personne ne lui dit où en étaient les classes. Le procès-verbal d'installation doit être signé le jour même [NV]. *31 décembre* : fin de son premier contrat ; le renouvellement est saisi le 2 janvier.
**Incidents.** Le 1ᵉʳ janvier à minuit, son accès se ferme ; le 2 au matin elle ne peut plus faire l'appel. Elle cherche quels élèves ont un aménagement (socle 35).

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-173 | Enregistrer la **prise de fonctions effective** (date réelle, distincte de la date prévue ; procès-verbal d'installation [NV]) | La date réelle déclenche la paie dans le public [NV] et fait foi en cas de litige ; la date prévue n'est pas la date vécue | important | 6 (+7) | PRS-116 |
| M-174 | **Droits ouverts dès la prise de poste**, pas au premier cours | Renvoi M-043 de la passe 2–4 : sinon, appel sur papier les premiers jours | important | 6 | R-P3 |
| M-175 | **Dossier d'arrivée** rempli par l'agent (coordonnées, contact d'urgence, photo, pièces demandées selon la nature), validé par le secrétariat | Trou T-07 de la matrice (accueil d'un nouvel agent) ; le Next.js le fait, la conception 06 non | important | 6 (+7) | PRS-117, PRS-118 |
| M-176 | **Dossier de l'arrivant** sur une page : emploi du temps, classes avec trombinoscope, PP et CPE de ses classes, élèves à aménagement, plan des salles, contacts utiles, règlement | Tout existe, éparpillé dans six écrans ; la première heure décide de la suite | différenciant | 6 (+5, 35) | PRS-123 |
| M-177 | **Note de passation** attachée au cours : progression, évaluations prévues, points d'attention ; écrite par le titulaire ou, à défaut, le coordinateur | Quand le titulaire est malade ou que le poste est vacant, personne ne transmet rien | différenciant | 6 | PRS-122, R-P7 |
| M-178 | **Invitation sur l'adresse personnelle** quand l'adresse professionnelle n'existe pas encore, puis bascule sans nouveau compte | Cas ordinaire des contractuels [H] | confort | 6 | R-P18 |
| M-179 | **Renouveler un engagement en un geste**, sans rupture d'accès ; alertes avant la fin | Les contrats courts se renouvellent souvent au dernier moment [H] ; une nuit sans accès = un appel sur papier, un cahier de textes perdu | important | 6 | PRS-121, R-P9 |
| M-180 | **Clé de sécurité de prêt** (parc de l'établissement), enrôlée au guichet, rendue au départ | D-009 sans téléphone compatible ; Q-P18 prévoit des clés « fournies » sans prêt ni retour | important | 6 (+44) | PRS-119 |

### P06 — Karim, professeur de musique contractuel sur deux collèges ScolIA

**Année.** Douze heures au collège A, six au collège B. *Septembre* : deux pré-rentrées le même jour ; il choisit A. *Toute l'année* : il ressaisit dans B les créneaux pris dans A pour ne pas être convoqué ; sa chorale (élèves de la 6ᵉ à la 3ᵉ, mercredi midi) compte dans son service [NV]. *Décembre* : deux conseils de classe au même moment.
**Incidents.** Le jour de la rentrée, il perd son téléphone. La récupération par e-mail prévoit un **délai de sécurité de 24 à 48 h** (D-018), et la direction ne peut pas le dépanner parce qu'il est rattaché à deux établissements. Il fait l'appel sur papier deux jours.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-181 | **Accès de secours local** pour un agent de plusieurs établissements qui a perdu sa passkey : clé de prêt valable dans **cet** établissement seulement, au plus 48 h, pendant la récupération D-018 | D-018 protège bien l'identité, mais bloque le travail de terrain les jours critiques | important | 6 (+1) | PRS-120, R-P16, Q-P31 |
| M-182 | **Partager automatiquement ses créneaux occupés** d'un établissement ScolIA vers l'autre, sur décision de l'agent, sans lieu ni nature | PRS-55 demande une ressaisie ; l'agent seul peut relier ses deux établissements (D-013) | différenciant | 6 (+10) | PRS-141 |
| M-183 | Un **groupe d'activité** (chorale, atelier) peut porter un cours compté dans le service | Sinon la chorale n'existe que comme activité (socle 43), hors du service et des heures dues | confort | 5, 6 | R-S21 |
| M-184 | Deux conseils ou réunions au même moment dans deux établissements | Convocations à arbitrer ; l'information « indisponible » suffit | confort | 19, 23 | renvoi |

### P07 — Mathis, AED étudiant ; Léna, AED et étudiante en BTS du même lycée

**Année.** Quinze AED : grille du matin, cantine (deux services), étude, permanence, nuits d'internat. Le **planning** est un tableur de la CPE, sur deux semaines tournantes, avec des exceptions chaque semaine (examens universitaires de Mathis en janvier, absences). Les heures sont annualisées [NV]. Les AED échangent leurs créneaux entre eux par messagerie personnelle.
**Incidents.** Léna est en 2ᵉ année de BTS dans le lycée **et** AED : par la portée « vie scolaire, établissement », elle voit les absences, les retards et les punitions de ses camarades de promotion.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-185 | **Planning de service** des personnels non enseignants : postes (grille, cantine, étude, nuit), cycles A/B, exceptions, compteur d'heures annualisées | Trou T-09 de la matrice et PER-10 (« plannings » cités pour Charlemagne [C5]) ; sans lui, tableur, et la vie scolaire ne sait pas qui est où | important | 6 (+15) | PRS-133, R-P10, Q-P30 |
| M-186 | **Échange de service** entre agents, validé par le responsable | Échanges réels, aujourd'hui hors de tout contrôle | confort | 6 | PRS-134 |
| M-187 | **Casquette élève et casquette personnel** pour la même personne : la portée personnel exclut ses propres classes et groupes ; alerte à la direction | Fuite certaine de données entre camarades, sans contournement possible avec une portée « établissement » | **bloquant** | 6 (+1, 2) | R-P5 |

### P08 — Fatou, agent d'accueil (employée de la région)

**Journée.** 7 h 30 : un intervenant de prévention se présente, personne ne l'a annoncé. 9 h : un parent cherche la classe de son fils : la salle a changé, le cours a lieu au gymnase municipal. 11 h : un malaise dans la cour, il faut joindre « l'AED de service » et un secouriste. 14 h : un livreur demande l'agent de maintenance.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-188 | **Qui est de service maintenant** (poste, agent, moyen de le joindre) : accueil, vie scolaire, direction ; base de la mise en sûreté | En cas d'urgence, chercher dans un tableur coûte des minutes | important | 6 (+14) | PRS-135 |
| M-189 | **Intervenants et visiteurs attendus aujourd'hui** | Contrôle d'accès réel [H] ; l'accueil est le premier filtre | confort | 6 (+14) | PRS-136 |
| M-190 | « **Où est cette classe maintenant ?** » qui tient compte des changements de salle et des lieux extérieurs | Port de lecture de l'emploi du temps ; le socle 5 fournit les lieux (STR-109) | confort | 10 (+5) | renvoi |

### P09 — Bernard, comptable de l'OGEC (ensemble privé de P01)

**Année.** Quarante salariés de l'OGEC (vie scolaire, administration, cuisine, entretien) et soixante maîtres payés par l'État. Huit maîtres font aussi des **surveillances et des études payées par l'OGEC**. Trois vacataires d'étude sont payés à l'heure **selon le service fait**. Il tient le registre des entrées et sorties du personnel [NV : registre unique du personnel] et a besoin, chaque mois, des heures réellement assurées. Les fins de contrat de droit privé ont leurs motifs (fin de période d'essai, rupture conventionnelle, licenciement).
**Incidents.** Il veut créer l'engagement « salarié OGEC » d'un maître : refusé, « un engagement existe déjà sur cette période ».

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-191 | **Engagements simultanés** auprès de deux employeurs ou à deux titres | §0 point C ; cas réel impossible à saisir | **bloquant** | 6 (+49) | R-P1, Q-P29 |
| M-192 | **Motifs de fin** de droit privé et de la fonction publique (fin de période d'essai, rupture conventionnelle, licenciement, abandon de poste, détachement, radiation) [NV] | La liste de 06 §8.2 ne couvre pas le privé ; « autre » partout rend tout comptage impossible | confort | 6 | R-P13 |
| M-193 | **Relevé du service fait** par intervenant (séances assurées d'après l'emploi du temps et l'appel) | Vacataires, hors contrat, heures OGEC : la paie (49) et l'état mensuel en dépendent ; aujourd'hui recompté à la main | important | 6 (+10, 12, 15, 49) | PRS-145 |
| M-194 | **Export du registre des entrées et sorties** des salariés de l'organisme (dates, nature, fonction), tracé | Obligation de l'employeur [NV] ; les dates viennent du socle 6, le registre complet relève du socle 49 | confort | 6 (+49) | R-P22 |

### P10 — Mme Roussel, inspectrice pédagogique ; M. Garnier, inspecteur chargé du contrôle d'un hors contrat

**Mme Roussel** vient inspecter une professeure de physique (rendez-vous de carrière [NV]) : elle veut voir le cahier de textes et la progression de l'année, l'emploi du temps, et la liste de la classe visitée, **ce jour-là seulement**. Aujourd'hui on lui imprime des pages, ou on lui prête un compte.
**M. Garnier** contrôle un établissement hors contrat [NV : contrôle de l'État, pièces exigibles] : liste des personnels avec fonctions et titres déclarés, structure, effectifs, emplois du temps.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-195 | **Accès de consultation temporaire** pour un inspecteur, un formateur ou un tuteur extérieur : lecture du cahier, de la progression, de l'emploi du temps d'un enseignant et de la liste de la classe visitée, sur une journée ; enseignant informé ; journal | Sans lui : compte prêté (interdit, 06 §13.2) ou impressions | important | 6 (+1) | PRS-125, R-P8, Q-P34 |
| M-196 | **Dossier de contrôle ou d'inspection** : liste des personnels (fonctions, titres déclarés), structure, effectifs, emplois du temps, en un export tracé [NV] | Le hors contrat le prépare dans l'urgence ; tout est dans l'ENT | important | 6 (+5) | PRS-126 |
| M-197 | Convocation au rendez-vous de carrière, visites d'un formateur | Réunions et rendez-vous | confort | 19 | renvoi |

### P11 — Chloé (changements d'enseignements) et Hugo (réorientation)

**Chloé, 2ᵈᵉ puis 1ʳᵉ.** *Toussaint de 2ᵈᵉ* : elle abandonne le latin. *Décembre de 1ʳᵉ* : elle change une spécialité (le lycée l'accepte jusqu'aux vacances de Noël [NV : règles de changement de spécialité]). Ses notes du premier trimestre dans l'ancienne spécialité doivent rester sur son bulletin et dans les exports officiels ; en fin de 1ʳᵉ, elle **abandonne** une spécialité pour la terminale [NV : la spécialité abandonnée est évaluée en fin de 1ʳᵉ]. *Février de 1ʳᵉ* : elle part trois mois en échange au Canada ; une correspondante canadienne est accueillie dans sa classe au printemps.
**Hugo, 2ᵈᵉ générale.** En novembre, une passerelle l'oriente vers une 2ᵈᵉ professionnelle du même lycée polyvalent : nouvelle formation, nouvelle classe, nouveaux enseignements.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-198 | **Enseignement quitté ou abandonné** conservé avec sa nature (changement en cours d'année, abandon de fin de 1ʳᵉ) et ses dates | Les exports d'examen et de candidature dans l'enseignement supérieur en dépendent [NV] ; un simple « abandonné » ne dit pas lequel des deux | important | 5 (+38, 9) | R-S16 |
| M-199 | **Réorientation en cours d'année** (changement de formation) en une commande : inscription annuelle (socle 2), classe, enseignements suivis, groupes | Sinon quatre gestes dans deux socles, et l'élève reste dans des groupes de la voie générale | important | 5 (+2) | STR-101 |
| M-200 | **Absence de longue durée hors de l'établissement** (échange international, scolarité partagée) et **accueil temporaire** d'un élève non inscrit (correspondant, mini-stage) dans des cours datés | Un correspondant présent en classe doit être sur la liste d'appel (sécurité) ; un élève parti trois mois ne doit pas être compté absent à chaque heure | important | 5 (+2, 12) | STR-102 |
| M-201 | Le bulletin et les moyennes de l'enseignement quitté | Calculés sur les périodes suivies | confort | 21, 23 | renvoi |

### P12 — Nathalie, CPE (lycée avec internat, 180 internes)

**Incident de janvier.** Neige, le préfet interdit les transports scolaires le jeudi [NV] : la direction déclare une fermeture. La conception produit un événement unique : séances annulées, **repas annulés**, familles prévenues. Or les 180 internes sont dans l'établissement : il faut les nourrir, les surveiller, et accueillir les externes déjà arrivés à pied. L'internat ne ferme pas.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-202 | **Fermeture à effets différenciés** : cours, accueil des élèves présents, restauration, internat, transports ; l'aperçu liste les internes présents | Annuler la restauration de 180 mineurs présents par effet automatique est dangereux | **bloquant** | 5 (+40, 41) | R-S1 |
| M-203 | **« Établissement ouvert, cours non assurés »** (accueil des élèves présents) | Situation réelle des jours de grève des transports ou d'intempéries [H] ; l'appel doit continuer pour les présents | important | 5 (+12) | R-S1 |
| M-204 | Mesure d'éloignement prise en compte dans l'attribution des chambres d'internat | Même logique que M-143 | important | 41 | renvoi |

### P13 — Sophie, professeure d'EPS, coordinatrice de l'association sportive

**Année.** Les cours d'EPS ont lieu au gymnase municipal (dix minutes à pied) et à la piscine intercommunale (car) : la mairie attribue des **créneaux** au collège par convention ; la piscine ferme deux semaines en mars pour vidange. En natation, l'encadrement impose un nombre maximal d'élèves par groupe [NV]. Elle est secouriste du travail et titulaire d'une qualification de surveillance de baignade.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-205 | **Lieux extérieurs** (gymnase, piscine, stade) avec adresse, temps de trajet et **fenêtres de disponibilité** conventionnées | Une salle « extérieure » sans disponibilité ni trajet fausse l'emploi du temps et les sorties | important | 5 (+10, 42) | STR-109, R-S14 |
| M-206 | **Capacité bloquante** d'un groupe quand elle relève de la sécurité (bassin, atelier, laboratoire) | Un avertissement ne suffit pas quand la limite est une règle de sécurité | confort | 5 | R-S15 |
| M-207 | Qualifications usuelles manquantes (sauveteur secouriste du travail, sécurité incendie, conduite d'engins, hygiène alimentaire, animation) [NV] | Le socle 14 et les sorties en ont besoin ; « autre » rend la liste inexploitable | confort | 6 | R-P14 |

### P14 — M. Benali, directeur délégué aux formations (lycée professionnel)

**Année.** Chaque classe a ses **périodes de formation en milieu professionnel**, à des dates différentes ; pendant ces périodes, les professeurs suivent les élèves en entreprise. Deux classes mêlent élèves sous statut scolaire et apprentis, dont le calendrier d'alternance diffère. Les ateliers ont des capacités de sécurité ; les professeurs d'atelier ont des **habilitations** (électrique, conduite d'engins) à renouveler.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-208 | **Mixité des publics** : calendrier d'alternance des apprentis porté par un groupe de la classe | Faisable avec les cibles de groupe, mais le cas doit être prévu et testé | confort | 5 | cas limite §4.5 |
| M-209 | **Alerte d'expiration** des habilitations des enseignants d'atelier avant leurs séances | Un professeur non habilité devant une machine engage la responsabilité de l'établissement [NV] | important | 6 (+10) | R-P14 |
| M-210 | Heures libérées pendant les périodes en entreprise et visites de suivi | Emploi du temps et stages | confort | 10, 39 | renvoi |

### P15 — Agnès, directrice de l'école de l'ensemble (premier degré)

**Année.** Elle enseigne à 75 % et a une **décharge de direction** [NV]. Classes à deux niveaux ; le jeudi, **décloisonnement** : les CE2 vont en CM1 pour les mathématiques. Deux agents spécialisés des écoles maternelles (employés par la commune ou l'OGEC selon les écoles [NV]) suivent un planning. En juin, elle participe à la liaison avec le collège de l'ensemble.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-211 | **Décloisonnement** : élèves d'une classe inclus dans le cours d'une autre | Même mécanisme que l'inclusion d'ULIS (M-224) ; sans lui le premier degré est impossible (05 §5.4) | important | 5 (+50) | STR-102 |
| M-212 | **Décharge de direction** comme composante du service dû | Sans elle, l'écart d'heures de la directrice est faux | confort | 6 (+50) | PRS-130 |
| M-213 | **Planning des agents de maternelle** | Même besoin que les AED | confort | 6 (+50) | PRS-133 |

### P16 — Olivier, directeur d'un établissement hors contrat international

**Année.** Rentrée le 25 août, trois « terms », classes nommées « Year 7 » à « Year 13 », double programme. La moitié des enseignants sont **vacataires payés à l'heure** (voir M-193). Un contrôle de l'État a lieu en mars (voir M-196). Les intervenants (musique, théâtre, sports) et les bénévoles sont nombreux ; il veut savoir pour chacun si la **vérification d'honorabilité** a été faite et quand la renouveler [NV : obligations applicables aux établissements hors contrat et aux bénévoles].

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-214 | **Vérification d'honorabilité** des personnels, intervenants et bénévoles : date, auteur, résultat binaire, échéance ; jamais le contenu [NV] | Protection des mineurs ; suivi aujourd'hui dans un classeur | important | 6 | PRS-143 |

### P17 — M. Payet, proviseur à La Réunion

**Année.** Rentrée mi-août, année « à cheval » sur deux années civiles, fuseau à +3 h l'hiver et +2 h l'été par rapport à Paris. En février, **alerte cyclonique** : la préfecture décide de la fermeture selon le niveau d'alerte, puis de sa levée, parfois dans la nuit [NV].

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-215 | **Fuseau de l'établissement** dans le socle 6 (fin d'accès, suppléances, délégations, délai de grâce) | §0 point D | important | 6 | R-P2 |
| M-216 | **Fermeture conditionnelle et levée programmée** (alerte météo, décision préfectorale) | La fermeture se déclare au jour le jour ; sa levée doit prévenir tout le monde sans nouvelle saisie | important | 5 (+8) | STR-94 |

### P18 — Corinne, secrétaire de direction (collège public)

**Année.** *Septembre* : douze prises de fonctions, chacune avec son procès-verbal [NV] ; organigramme pour l'accueil et le site. *Toute l'année* : « pouvez-vous me faire une **attestation d'emploi** ? » (contractuels, AED, AESH : banque, logement, inscription à un concours) ; dépôt des arrêtés et états de service dans le dossier de chacun. *Janvier* : une professeure part en **disponibilité** pour un an : ni départ, ni absence. *Mars* : un AED devient contractuel enseignant (nouvelle fonction, nouveaux droits). Les demandes de congé des agents doivent aller à « leur responsable » : qui est-ce ?

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-217 | **Attestation d'emploi** demandée par l'agent, délivrée selon une trame, signée par un habilité | Demande la plus fréquente au secrétariat de direction [H] | important | 6 (+7) | PRS-142 |
| M-218 | **« Mes documents »** : pièces de l'agent (états de service, procès-verbal, arrêtés, attestations), dépôt en lot avec rattachement | PER-08 de la matrice ; aujourd'hui des courriels un par un | confort | 6 (+7, 49) | PRS-140 |
| M-219 | **Organigramme fonctionnel** calculé, version réduite pour les familles | PER-03 de la matrice ; refait à la main chaque rentrée | confort | 6 | PRS-138 |
| M-220 | **Responsable fonctionnel** d'un agent (CPE pour les AED, gestionnaire pour les agents), valideur des demandes | Les socles 15 (congés, absences) et N1 (demandes) devraient sinon coder les circuits en dur | important | 6 (+15, N1) | PRS-139 |
| M-221 | **Suspension d'un engagement** (disponibilité, détachement, congé parental) sans le clore | 06 §4.2 P10 parle d'« engagement suspendu », mais le modèle (§8.2) n'a aucun état de suspension : contradiction interne | important | 6 | PRS-147, R-P4 |
| M-222 | **Changement de fonction en cours d'année** (AED devenu enseignant), avec rôles proposés de nouveau | Sans geste prévu : fin, recréation, perte d'historique | confort | 6 | PRS-146 |

### P19 — Élodie, titulaire sur zone de remplacement

**Année.** Trois suppléances : six semaines au collège A, quatre mois au lycée B (prolongées **cinq fois** par le rectorat, d'une à trois semaines à chaque fois [H]), puis deux mois au collège C. Même identité, même passkey (D-013) ; chaque établissement ne connaît que sa période.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-223 | **Prolonger une suppléance** en un geste, sans nouvelle annonce aux familles ni nouvelle note de passation | Cinq prolongations = cinq messages « changement de professeur » qui n'en sont pas | confort | 6 (+15) | R-P19 |

### P20 — Samia, AESH (trois élèves, deux établissements)

**Année.** Elle accompagne Yanis (ULIS, collège A), qui suit les mathématiques et l'EPS en 5B et le reste dans sa classe de référence ; et deux élèves du collège B. Elle doit connaître leur emploi du temps exact et être prévenue quand un groupe change. Les professeurs de la 5B ne voient pas Yanis dans leur liste d'appel.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-224 | **Membres individuels d'un cours** (inclusion, exclusion), datés | §0 point E : un élève d'ULIS absent d'une liste d'appel peut disparaître sans alerte | **bloquant** | 5 | STR-102, R-S5, Q-S23 |
| M-225 | Les **titulaires d'une mission sur des élèves** (AESH, tuteur) reçoivent les changements de classe, de groupe et d'inclusion de ces élèves | Sinon elle attend l'élève dans la mauvaise salle | important | 6 (+5, 35) | R-P20 |
| M-226 | L'**AESH dans l'équipe de la classe** quand un élève qu'elle accompagne en fait partie | 06 §8.8 ne la compte que dans « l'équipe de l'élève » ; elle manque aux convocations de conseil | confort | 6 (+23) | R-P21 |

### P21 — Mme Perrin, adjointe gestionnaire (lycée public)

**Année.** Douze agents de la région : planning, permanences des vacances, grands ménages d'été, astreintes des personnels logés [NV]. En juin, quatorze salles sont **réquisitionnées** pour les examens pendant dix jours.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-227 | **Indisponibilité de plusieurs salles** pour un événement du calendrier, en un geste | Quatorze saisies une par une, et autant d'oublis | confort | 5 | STR-110 |
| M-228 | **Postes d'astreinte et permanences** dans le planning de service [NV] | Mêmes besoins que les AED, sur les vacances | confort | 6 | PRS-133 |

### P22 — Famille Lefèvre (deux enfants dans une cité scolaire)

Emma est en 4ᵉ, Jules en 1ʳᵉ. Les journées banalisées ne sont pas les mêmes ; la rentrée non plus (4ᵉ le mardi, 1ʳᵉ le mercredi). Les parents veulent **un seul calendrier**, dans leur téléphone, et savoir dès juillet la date de rentrée de chacun (M-145).

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-229 | **Calendrier familial réuni** (tous les enfants, un abonnement), chaque événement étiqueté par enfant | Deux abonnements, deux applications de calendrier, des oublis | confort | 5 | R-S22 |

### P23 — Yann, référent numérique et administrateur (collège privé)

Il conduit la revue des accès chaque trimestre ; il veut y voir aussi les accès temporaires (inspection, intervenants) et les clés de sécurité prêtées. En salle informatique, les professeurs se connectent sur un poste partagé.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-230 | **Accès temporaires et clés prêtées** dans la revue des accès | Ce sont les accès les plus facilement oubliés | confort | 6 | R-P23 |
| M-231 | Fermeture de session automatique sur un poste de salle en fin de séance | Sécurité des postes partagés | confort | 1, 12 | renvoi |

### P24 — Les représentants du personnel de l'OGEC, et un professeur qui exerce son droit d'accès

Les représentants apprennent que l'ENT pourrait proposer un **pouls d'humeur** anonyme et un planning de service : ils demandent à être consultés avant toute mise en œuvre d'un outil qui peut servir à contrôler l'activité [NV : consultation du comité social et économique]. Un professeur demande ce que l'établissement sait de lui et qui l'a consulté (PRS-106 répond).

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-232 | **Pouls d'humeur anonyme** (trou T-08), désactivé par défaut, agrégé à partir d'un seuil, jamais nominatif | Fonction du Next.js ; risque de perception de surveillance : à soumettre à Florian | confort | 6 (+47) | PRS-144, R-P12, Q-P32 |
| M-233 | **Information et consultation des représentants** avant d'activer planning, indicateurs d'activité ou pouls [NV] | Un outil déployé sans consultation dans le privé expose l'OGEC [NV] | important | 6 | R-P24 |

### Manques transverses (plusieurs personas)

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-234 | **Noms d'événements alignés** entre les socles 5 et 6 | §0 point F | confort | 5, 6 | R-S23, R-P26 |
| M-235 | **Contrat IA** des actions ajoutées (D-025) : lectures et commandes déclarées, niveau de confirmation, aucune donnée scellée ou sensible lue par l'IA | Le contrat IA est un critère de fin de lot ; les ajouts doivent le porter dès leur conception | important | 5, 6 | R-S24, R-P25 |
| M-236 | **Lien personnel avec un élève** hors lien légal (neveu, enfant d'amis, voisin) : déclaration par l'agent, déport automatique (conseil, constitution, discipline), accès tracé à part | Renvois M-100 et M-101 de la passe 2–4 : la casquette parent ne couvre que les liens légaux ; Thomas (P04) a dans sa classe le fils de son voisin, Anne (P01) sa nièce en 6ᵉ | important | 6 (+5, 13, 23) | PRS-137, R-P6 |

---

## 4. Couverture Charlemagne des socles 5 et 6 (D-023, D-024)

Relecture des lignes de `MATRICE_CHARLEMAGNE.md` portées par les socles 5 et 6, après cette passe. « Conçu » ne veut pas dire fait.

| Ligne de la matrice | Avant | Après la passe | Ajout |
|---|---|---|---|
| STR-01 à STR-08, STR-10 à STR-14 | codé ou conçu | conçu, complété (lieux extérieurs, fermetures différenciées, services administratifs, examens nationaux, constat) | STR-93, 94, 97, 109, 110, 114 |
| STR-09 Annulation d'une bascule (T-31) | partiel | **conçu** | STR-95, R-S3 |
| Listes, étiquettes, trombinoscopes, « éditions » [H] (pas de ligne dans la matrice) | absent | **conçu** | STR-111, STR-112 |
| Enquêtes et constats d'effectifs (T-25, part du socle 5) | carte | conçu côté structure ; transmission au socle 9 | STR-97 |
| PER-01 Fiche personnel | partiel | conçu, complété (prise de fonctions, suspension, changement de fonction, motifs) | PRS-116, 146, 147, R-P13 |
| PER-02 Arrivées et départs | conçu | conçu, complété (renouvellement sans rupture, dossier de l'arrivant) | PRS-121, 123 |
| PER-03 Annuaire, organigramme | conçu (sans organigramme) | **conçu** | PRS-138 |
| PER-04 Services, PP, équipes, référents | conçu | conçu, complété (vœux, proposition du coordinateur, publication, état de service, composantes du service dû) | PRS-127 à 131 |
| PER-05 Casquettes | conçu | conçu, complété (élève + personnel) | R-P5 |
| PER-06 Habilitations | conçu | conçu, complété (codes, alertes avant séance, honorabilité) | R-P14, PRS-143 |
| PER-07 Accueil d'un nouvel agent (T-07) | **TROU** | **conçu** | PRS-117, 118, 123 |
| PER-08 Dépôt des documents RH | carte | conçu (partie socle 6 ; moteur au socle 7) | PRS-140 |
| PER-10 Planning de service (T-09) | **TROU** | **conçu** (question Q-P30 sur sa place) | PRS-133 à 135 |
| PER-14 Pouls d'humeur (T-08) | **TROU** | conçu, **désactivé**, à décider | PRS-144, Q-P32 |
| PER-15 Heures supplémentaires effectives (T-02) | TROU | reste au socle 15 / N1 ; le socle 6 porte la nature de rémunération des missions | PRS-132 |
| PER-16, PER-17 Paie (T-04) | carte / TROU | reste au socle 49 ; le socle 6 fournit engagements simultanés et service fait | R-P1, PRS-145 |
| PER-09, PER-11, PER-12, PER-13, PER-19, PER-20 | carte / TROU | restent aux socles 15 et 49 (renvoi inchangé) | — |
| EDT-15 Interventions transversales (T-15, part du socle 6) | TROU | **conçu** côté personnels | PRS-124 |
| EDT-17 Présences des enseignants aux conseils | carte | reste au socle 23 ; l'équipe de classe inclut l'AESH | R-P21 |
| SAN-17 Planning des surveillants d'internat | carte | **conçu** par le planning de service | PRS-133 |

---

## 5. Manques qui relèvent de socles ultérieurs

| Socle | Manques | Ce qu'il faudra prévoir |
|---|---|---|
| 1 Sécurité | M-181, M-187, M-231 | Clé de prêt locale à durée bornée ; portée qui exclut ses propres classes ; fermeture de session sur poste partagé |
| 2 L'élève | M-199, M-200 | Port de changement d'inscription annuelle (formation) appelé par STR-101 ; statut « accueil temporaire » d'un élève non inscrit |
| 7 Documents | M-156, M-173, M-217, M-218 | Trames d'état de service, procès-verbal d'installation, attestation d'emploi ; dépôt en lot avec rattachement (T-13) |
| 9 Interopérabilité | M-147, M-165, M-198 | Constat transmis ; code national de division ; nature d'un enseignement quitté dans les exports |
| 10 Emploi du temps | M-160, M-182, M-190, M-205, M-210 | Contrôle de capacité ; créneaux occupés importés ; « où est la classe » ; trajets et fenêtres de disponibilité ; heures libérées en période d'entreprise |
| 12 Appel | M-154, M-200, M-203, M-231 | Revue des absences saisies dans un mauvais groupe ; élèves accueillis ; appel des présents un jour sans cours |
| 13 Discipline, 36 Suivi social | M-143 | Mesure d'éloignement publiée comme fait pour le socle 5, sans motif |
| 14 Accueil, sûreté | M-188, M-189 | Présents et de service, visiteurs attendus |
| 15 Absences des personnels | M-185, M-220, M-223 | Absences des agents planifiés ; circuit de validation par le responsable fonctionnel ; prolongations de suppléance |
| 19, 23 Réunions, conseils | M-184, M-197, M-226 | Convocations en conflit ; rendez-vous de carrière ; AESH convoquée |
| 21, 23 Notes, bulletins | M-201 | Moyennes de l'enseignement quitté |
| 35 Besoins particuliers | M-225 | Mission AESH alimentée par le socle 35 ; notification des changements |
| 38 Examens | M-168, M-198 | Dates nationales ; spécialité abandonnée |
| 39 Stages | M-210 | Visites en entreprise |
| 40 Restauration, 41 Internat | M-202, M-204 | Repas maintenus pour les internes ; éloignement en chambre |
| 44 Ressources | M-180 | Parc de clés de sécurité, comme le matériel prêté |
| 47 Pilotage | M-147, M-232 | Constats comparés ; agrégats du pouls (seuil) |
| 49 Paie | M-157, M-191, M-193, M-194 | Rémunération des missions, doubles employeurs, service fait, registre |
| 50 Premier degré | M-211, M-212, M-213 | Décloisonnement, décharge, agents de maternelle |

---

## 6. Synthèse

### 6.1 Manques par gravité

| Gravité | Nombre |
|---|---|
| bloquant | 6 |
| important | 53 |
| confort | 35 |
| différenciant | 4 |
| **Total** | **98** (M-139 à M-236) |

Dont **91** traités par un ajout dans les socles 5 et 6, et **7** renvoyés en entier à un socle ultérieur (M-184, M-190, M-197, M-201, M-204, M-210, M-231) ; d'autres ont une part renvoyée, listée au §5. Comptage fait par script sur les tableaux du §3. Les six bloquants : **M-150, M-151** (contradictions entre les deux conceptions), **M-187** (AED étudiante qui voit ses camarades), **M-191** (doubles employeurs impossibles), **M-202** (fermeture qui annule les repas des internes), **M-224** (élève d'ULIS absent des listes d'appel).

Par socle : **socle 5 seul : 37** manques (bloquants propres : M-202, M-224) ; **socle 6 seul : 49** (bloquants propres : M-187, M-191) ; **les deux : 5** (M-150, M-151, M-183, M-234, M-235) ; **renvoyés : 7**.

### 6.2 Ajouts par socle

| Socle | Actions métier ajoutées | Règles, cas limites, modèle | Questions ajoutées |
|---|---|---|---|
| 5 Structure (`05_STRUCTURE.md`) | **22** (STR-93 à STR-114) | 24 règles (R-S1 à R-S24), 14 cas limites (§4.5), §9.10 modèle, lots | 7 (Q-S20 à Q-S26) |
| 6 Personnels (`06_PERSONNELS.md`) | **32** (PRS-116 à PRS-147) | 26 règles (R-P1 à R-P26), 14 cas limites (P47 à P60), §8.14 modèle, lots P15 et P16 | 8 (Q-P28 à Q-P35) |
| **Total** | **54** | 50 règles, 28 cas limites | 15 |

### 6.3 Les dix ajouts les plus importants

1. **Un seul objet « cours », une seule règle de visibilité** (Q-S20, Q-P28, Q-S21 ; M-150, M-151). Sans cela, les lots S4 et P3 coderaient deux fois la même chose, et D-022 serait calculée de deux façons.
2. **Membres individuels d'un cours** (STR-102, R-S5 ; M-224, M-211, M-200). L'élève d'ULIS inclus, le décloisonnement, le correspondant accueilli, l'élève parti en échange : chacun sur la bonne liste d'appel.
3. **Fermeture à effets différenciés** (R-S1, STR-94 ; M-202, M-203, M-216). Les internes gardent leurs repas ; « ouvert sans cours » existe ; l'alerte cyclonique se lève seule.
4. **Engagements simultanés** (R-P1 ; M-191). Le privé ne peut pas fonctionner sans : maître et salarié de l'OGEC sont la même personne.
5. **Casquette élève et personnel** (R-P5 ; M-187), et **lien personnel déclaré avec déport** (PRS-137, R-P6 ; M-236). Les conflits d'intérêts quotidiens d'un établissement.
6. **Planning de service des personnels non enseignants** (PRS-133 à 135 ; M-185, M-188). Trou Charlemagne T-09 ; la vie scolaire et l'accueil savent qui est où.
7. **Arrivée complète** : prise de fonctions réelle, dossier rempli par l'agent, dossier de l'arrivant, note de passation, droits dès la prise de poste, renouvellement sans rupture (PRS-116 à 123 ; M-173 à M-179). Trou Charlemagne T-07.
8. **Bascule sûre** : annuler une ouverture, réviser la montée après appel, rouvrir une campagne validée, constat d'effectifs figé, informations de rentrée publiées avant l'ouverture (STR-95, 105, 106, 97, 96).
9. **Listes et reprise** : listes en quatre trames, liste à la demande, affichage de rentrée sans donnée sensible, reprise datée en cours d'année (STR-111, 112, 113 ; R-S9, R-S13). Ce que Martine fait chaque semaine dans Charlemagne.
10. **Service des enseignants complet** : vœux, proposition du coordinateur, publication prévisionnelle, composantes du service dû, état de service signé, rémunérations indicatives des missions (PRS-127 à 132).

Suivent de près : la correction rétroactive d'une affectation (STR-104), le dédoublement d'un groupe (STR-103), les combinaisons de spécialités et barrettes (STR-99), l'accès d'inspection (PRS-125), l'accès de secours multi-établissements (PRS-120), la vérification d'honorabilité (PRS-143).

### 6.4 Ce que la passe a montré sur les conceptions

- **Les deux conceptions sont solides chacune de son côté** (tout daté, propositions validées par un humain, droits calculés depuis les faits). Leur défaut principal est **leur jointure** : écrites en parallèle, elles se disputent le même objet (cours / service), la même fonction (périmètre) et ne nomment pas les événements de la même façon. C'est ce que la passe a trouvé de plus grave.
- **Les situations réelles débordent la forme « classe ou groupe »** : inclusions, décloisonnement, échanges, élèves accueillis. Le socle 5 avait prévu les classes à plusieurs niveaux du premier degré, pas l'élève qui suit un seul cours ailleurs.
- **Les automatismes doivent savoir s'arrêter** : une fermeture n'est pas « tout fermer » ; une prolongation n'est pas un changement ; une correction n'est pas un mouvement.
- **Le socle 6 a été pensé surtout pour l'enseignant** : le personnel non enseignant (planning, astreintes, service fait), le droit privé (doubles employeurs, motifs de fin) et les visiteurs (inspection, partenaires) y étaient minces.
- **Le fuseau horaire revient**, après les socles 2 à 4 : il faut que le socle 5 en soit la seule source et que chaque conception le dise.

---

## 6 bis. Réglages signalés et documents prêts (D-023)

### 6 bis.1 Réglages candidats à l'allègement (signalés, non supprimés)

Même lecture que `PERSONAS_02-04.md` §5 bis.2 : « version automatique » = ce que fait le logiciel quand l'établissement ne touche à rien ; le réglage reste disponible tant que Florian n'a pas tranché. **Ne sont pas signalés** : matières, groupes, options, offre, capacités, grilles, périodes, fonctions, types de missions de l'établissement, motifs locaux (données de l'établissement, D-022).

| # | Réglage | Où | Pourquoi il pèse | Version automatique proposée |
|---|---|---|---|---|
| S-24 | Activation et gravité de **chaque** contrôle de rentrée | `05` §10 | Des dizaines de lignes « bloquant / avertissement » que personne ne relira | Gravité du logiciel ; seuls trois contrôles sensibles (codes nationaux, cours sans intervenant, élèves sans classe) restent réglables |
| S-25 | Critères de constitution et leurs **poids** | `05` STR-72 | Réglage technique d'un algorithme | Deux préréglages (« équilibre standard », « équilibre avec options ») ; poids avancés masqués |
| S-26 | Année de travail par défaut « selon le rôle et la saison (paramétrable) » | `05` STR-21 | Personne ne le règle | Règle fixe : N+1 en juillet–août pour la direction et le secrétariat, N sinon |
| S-27 | Durée et périmètre par défaut de la réouverture d'une année | `05` §4.4, STR-20 | Réglage d'un geste rare | 15 jours fixes ; périmètre choisi à chaque réouverture |
| S-28 | Délai de purge de la constitution | `05` §4.3 | Décision du DPO, pas de l'établissement | Valeur fixe validée par le DPO |
| S-29 | Date limite de changement d'option « paramétrable » | `05` §4.5 | Paramètre global de plus | Une date portée par l'offre de l'année (donnée), pas un paramètre |
| S-30 | Mode des semaines types (alternance continue, reprise après vacances) | `05` §9.3 | Peu d'établissements savent le dire | Déduit du calendrier adopté ; forçage d'une semaine seulement |
| S-31 | Publication de la liste de classe aux élèves, aux familles | `05` Q-S15, §16 | Deux réglages pour une décision de principe | Un seul choix, désactivé |
| S-32 | Délai de grâce de 0 à 15 jours | `06` Q-P6 | Choix technique | 7 jours fixes |
| S-33 | Lecture du titulaire pendant une suppléance | `06` Q-P5 | Choix de principe | Lecture conservée, fixe |
| S-34 | Droits par nature d'intervention, et restriction `droits` **par intervention** | `06` §8.5 | Matrice nature × droits, plus des exceptions ligne à ligne | Valeurs du logiciel ; restriction possible seulement pour la nature `intervenant` |
| S-35 | Types de missions : périmètres admis, cardinalité, règle d'éligibilité et son effet, visibilité familles et élèves, modèle de rôle | `06` §8.6 | Six réglages par type | Mission de l'établissement = nom + périmètre + rôle facultatif ; le reste fixé par le logiciel |
| S-36 | Cardinalité et éligibilité du PP (avertir ou bloquer) | `06` §8.13 | Choix fait une fois | Deux PP au plus, avertissement ; fixe |
| S-37 | **Modèles de parcours** d'arrivée et de départ en JSON, par nature et fonction | `06` §8.10 | Construire un parcours est un travail de consultant | Trois parcours types (enseignant, personnel, intervenant) ; ajout d'une étape simple |
| S-38 | Durée des invitations, jours de vacance avant alerte, fréquence de la revue des accès | `06` §8.13 | Trois nombres sans enjeu local | Valeurs fixes (7 jours, 5 jours ouvrés, trimestre) |
| S-39 | Profils de base et modèles de rôles proposés **par fonction** | `06` §8.13 | Réglage d'installation, jamais revu | Correspondance du logiciel ; exception rare |
| S-40 | Co-enseignement au bulletin : un ou deux noms | `06` P13 | Détail de présentation | Les deux noms, toujours |
| S-41 | Séparation des tâches (second administrateur) | `06` §13.1 | Réglage de sécurité que les petits établissements désactiveront | Activée d'office au-delà d'un seuil de personnels, sans réglage |
| S-42 | **Mes ajouts** : alertes J-7 et J-1, réouverture sans réinvitation à 30 jours, accès d'inspection à 1 jour, accès de secours à 48 h, seuil du pouls, date limite des demandes de changement de classe | R-P8, R-P9, R-P12, R-P16, STR-108 | Paramètres ajoutés par cette passe | **Valeurs fixes** au départ ; un réglage n'apparaît que si un pilote le demande |
| S-43 | **Liste à la demande** (STR-112) | `05` STR-112 | Risque de « requêteur » : la liberté totale qui produit des listes laides et figées (D-023) | Douze listes prêtes ; colonnes choisies dans un catalogue court ; pas de filtres libres combinés |

### 6 bis.2 Documents : 3 ou 4 trames prêtes par type (D-023)

Règle reprise de `PERSONAS_02-04.md` §5 bis.3 : 3 ou 4 trames soignées par document ; l'établissement choisit et n'ajuste que son identité (logo, couleurs, coordonnées, signature, un ou deux textes libres encadrés) ; contenu rempli par les données ; rendu par le socle 7 et la conversion maison (D-005).

| Socle | Documents |
|---|---|
| 5 | Calendrier annuel (familles ; personnels) ; listes de classe et de groupe (liste simple, émargement, trombinoscope, étiquettes) ; liste d'affichage de rentrée (R-S9) ; tableau de structure et répartition des moyens, présentés aux instances ; constat d'effectifs ; fiche classe ; avis de fermeture exceptionnelle (texte du socle 8) ; réponse à une demande de changement de classe ou d'enseignement |
| 6 | Procès-verbal d'installation [NV] ; état de service de l'enseignant [NV] ; lettre de mission (PP, référent, coordinateur) ; convocation de pré-rentrée ; attestation d'emploi ; récapitulatif de départ (accès fermé, matériel rendu) ; dossier de l'arrivant imprimable ; organigramme ; trombinoscope des personnels ; liste des personnels pour la mise en sûreté ; dossier de contrôle ; délégation de signature ; extrait du registre des entrées et sorties |

---

## 7. Faits, hypothèses, contradictions

**Faits [F]** vérifiés dans les conceptions : `ent_cours` possédé par le socle 5 avec le service comme présence d'une personne (`05` §7.1, §9.7, Q-S3) contre `ent_service_enseignement` possédé par le socle 6 avec les interventions (`06` §6.1, §8.5, Q-P4) ; `structure::perimetre` « seule implémentation » (`05` §6.3, §8.3) contre `services::eleves_vus` (`06` §5.3) ; exclusion des engagements chevauchants (`06` §8.2) ; « Europe/Paris » dans `06` §7, §8 et le lot P0 ; public d'un cours = classe XOR groupe (`05` §9.7) ; événement de fermeture consommé par la restauration pour annuler les repas (`05` §1 principe 8, §14.1) ; « engagement suspendu » cité au cas P10 sans état dans le modèle (`06` §4.2, §8.2) ; événements consommés par le socle 6 (`annee.ouverte`…) et émis par le socle 5 (`referentiel.annee_ouverte`…) ; plages de migrations 330–349 et 350–369 entièrement attribuées (`05` §18, `06` §17).

**Hypothèses [H]** : la vie des personas, les fréquences, les pratiques de marché, le contenu des « éditions » de Charlemagne.

**À vérifier [NV]** : dotation et circuit dans le privé sous contrat ; tableau de répartition des moyens et présentation aux instances ; état de ventilation de service et sa signature ; obligations de service, pondérations, décharges, heures supplémentaires ; indemnités pour missions particulières, part modulable du PP, Pacte ; procès-verbal d'installation et date de prise de fonctions ; forfait d'externat et constats d'effectifs ; codes de division de la base nationale ; règles de changement et d'abandon de spécialité ; éducation à la sexualité et interventions extérieures ; contrôle des établissements hors contrat ; vérification d'honorabilité ; registre unique du personnel ; motifs de fin de contrat ; consultation du comité social et économique ; décharge de direction ; agents de maternelle ; astreintes des personnels logés ; encadrement en natation ; habilitations ; fermetures préfectorales (neige, cyclone).

**Contradictions signalées** (toutes notées dans les conceptions, aucune tranchée) : cours / service (Q-S20, Q-P28) ; double fonction de périmètre (Q-S21) ; fuseau du socle 6 contre le profil du socle 5 et la passe 2–4 (R-P2) ; exclusion des engagements contre le cas réel des doubles employeurs (R-P1) ; « engagement suspendu » sans modèle (R-P4) ; noms d'événements (R-S23, R-P26). Les ajouts de modèle demandent des numéros de migration hors des plages réservées, déjà pleines : **à attribuer dans `MIGRATIONS.md`** au moment du code (non fait ici, aucune migration n'est écrite).
