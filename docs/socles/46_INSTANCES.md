# Socle 46 — Instances et vie démocratique

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian). Rédigée **avant** tout code. Aucun code écrit ni modifié ; aucun accès à une base, à un secret ou à un service de production. Le code Next.js n'a été parcouru que pour vérifier l'absence de module d'élections ou d'instances (noms de fichiers et routes, `app/`).
**Branche :** `ent-socle-45-46-conception`, partie de `origin/ent-mc0hoe` (`37ce0d3b`).
**Passe personas :** `docs/socles/PERSONAS_46.md` (même branche). Ses ajouts sont marqués **« ajout personas »** et regroupés au **§31**, qui l'emporte en cas de contradiction ; rien n'a été supprimé.
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` (D-017, D-019 public et privé ensemble, D-020, D-021, D-023, D-026, D-028, **D-029**, **D-031**, **D-034**, **D-035**, **D-036**, D-037 à D-041) ; `docs/SOCLES.md` (socles 4, 6, 13, 16, 17, 18, 19, 23, 42, 46, 48) ; `docs/MATRICE_CHARLEMAGNE.md` (VOY-15) ; `docs/PRA_SAUVEGARDES.md` §18.4 ; conceptions **`19_AGENDA_RDV.md`** (§6.3 instances de l'EPLE, §6.4 privé, AGD-122, AGD-132, AGD-141, AGD-142, CAG-40 à CAG-48 : **logistique des réunions**), `13_DISCIPLINE.md` (DIS-105, §12.1 : membres élus du conseil de discipline et de la commission éducative), `23_BULLETINS.md` (délégués au conseil de classe, BUL-109, BUL-402, `lsl_engagement`), `17_ANNONCES.md` (ANN-215, R-PB36 professions de foi), `16_MESSAGERIE.md` (`PERSONAS_16.md` M-2386, boîte des représentants), `18_FORMULAIRES.md` (RF-18, R-FC31 : un sondage n'est pas un scrutin), `06_PERSONNELS.md` (port `personnels::electeurs`), `04_ADMINISTRATIF.md` (autorité parentale, droits), `42_SORTIES_VOYAGES.md` (délibération, délai de 15 jours), `24_FINANCES.md`, `IA_COEUR.md`. Modèles de forme : `42_SORTIES_VOYAGES.md`, `10_EMPLOI_DU_TEMPS.md`, `21_EVALUATIONS.md`.

**Conventions.** [F] fait vérifié dans le code ou les documents ; [V] lu sur source officielle (§5) ; [V-copie] copie académique intégrale ; [V-indirect] texte cité par une source fiable ; [NV] non vérifié (interdit de coder dessus) ; [H] hypothèse ; [R] recommandation **appliquée par défaut (D-031)** sauf mention.

**Numérotation propre au socle** (vérifiée libre sur les 134 branches distantes le 2026-11-11) :

| Élément | Numéros |
|---|---|
| Actions cibles | **IVD-100**… (« instances et vie démocratique ») |
| Règles | **R-IVD1**… |
| Cas limites | **CIV-01**… |
| Questions | **Q-IVD1**… |
| Réglages signalés | **S-4600 à S-4649** |
| Manques de la passe personas | **M46-01**… |
| Lots | **DEM0 à DEM13** |

**Plage de migrations imposée : 2920–2939** (inscrite dans `MIGRATIONS.md`).

---

## 0. En bref

1. **Rien n'existe** dans ScolIA (Rust : aucune table d'instance, d'élection, de mandat, de délibération [F] ; Next.js : aucun module [F] ; seules les **délégations de droits** du socle 1 — sans rapport — portent le mot « délégué »). Pas de lot 0 de correctifs. Mais **trois socles attendent déjà le 46** : le 13 (membres élus du conseil de discipline, à la date), le 23 (délégués au conseil de classe, engagements du livret scolaire) et le 42 (délibération du CA sur les voyages) ; le 19 attend la **règle de quorum**. D'où un premier lot (DEM0) volontairement petit : **mandats saisis à la main** et **port de lecture**, avant toute élection.
2. **Thèse** : une instance est **une composition à une date** (qui siège, à quel titre, avec voix délibérative ou consultative), **des mandats** obtenus par élection, désignation ou de droit, et **des séances** qui produisent des **délibérations** ayant des effets sur les autres domaines. Une élection est **une procédure juridique datée** : un corps électoral figé à une date, des candidatures, un scrutin secret, un dépouillement, une proclamation, un délai de recours. Le logiciel doit rendre **impossible** l'irrégularité ordinaire (électeur oublié, parent sans autorité parentale inscrit, délai non tenu, calcul des restes faux, suppléant oublié) sans prétendre remplacer le bureau des élections.
3. **Frontières** : le **19** fait la logistique de toute réunion (convocation, délais, réponses, présence, procès-verbal déposé) ; le **46** possède la **composition**, les **mandats**, les **élections**, l'**ordre du jour délibératif**, les **votes**, les **délibérations** et les **actes**. Le 13 garde le contenu des conseils de discipline, le 23 celui des conseils de classe. Le **18** fait des sondages (« consultation, sans valeur de scrutin »), jamais un vote d'instance.
4. **Public et privé ensemble** (D-019) : le public a un conseil d'administration d'EPLE réglementé, des élections nationales de parents, un CVL et un CVC ; le privé sous contrat a un **conseil d'établissement** (statut interne de l'enseignement catholique [NV]), des **instances de l'organisme de gestion** (association : assemblée générale, conseil d'administration, bureau), une **association de parents** qui est une personne morale distincte, et des délégués de classe par usage. Le modèle est **un moteur d'instances générique** + **un référentiel de types d'instances par régime**, avec les règles réglementaires attachées aux types publics.
5. **Vote électronique** (§5.6, vérifié) : **autorisé seulement pour l'élection des parents au CA de l'EPLE public** (R421-30, arrêté du 2 juillet 2024), **interdit** pour les personnels, **sans texte** pour les élèves. Recommandation : (a) le **vote papier assisté** et le **vote par correspondance** sont livrés d'abord et couvrent 100 % des cas ; (b) le vote électronique des **parents** est un lot à part (DEM10–DEM11), construit selon l'arrêté de 2024 et la recommandation CNIL du 19 mars 2026 (niveau 1 : urne et émargement isolés, secret de dépouillement **réparti avec seuil** entre les membres du bureau, scellement, récépissé), activé seulement après l'**expertise indépendante** que l'arrêté impose ; (c) aucun vote électronique d'élèves. Coût de l'expertise et calendrier (viser octobre 2028 au plus tôt pour le public) : **à Florian** (§28).
6. **Données** : la liste électorale des parents est une donnée **sensible par ce qu'elle révèle** (autorité parentale retirée, foyers séparés, adresse protégée). Elle est **calculée** depuis le socle 4, **figée** à la date réglementaire, **publiée sans adresse**, et **jamais remise** aux listes candidates au-delà de ce que le texte prévoit (R-IVD12) ; les associations reçoivent un **canal de diffusion** égal pour toutes (17), pas un fichier.
7. **Délibérations comme faits du cœur** : une délibération a un **objet** typé (voyage du 42, tarif du 24, barème du 45, budget du 48, règlement intérieur, convention…) ; elle devient **exécutoire** à une date calculée (transmission au contrôle de légalité, délai de 15 jours pour les actes qui le demandent [§5]) ; les socles consommateurs lisent `instances::deliberation_executoire(objet)` et **n'appliquent rien avant** (R-IVD20).
8. **IA** : rédige un projet de procès-verbal à partir des notes du secrétaire de séance (N0), un calendrier électoral, une profession de foi **à la demande d'une liste** (dans l'espace de cette liste seulement) ; vérifie une composition ; **ne compte jamais** les voix, ne lit jamais un bulletin, ne désigne personne, ne propose pas de « candidat idéal ».
9. **Chiffres** (après la passe personas) : **205 actions actives** (IVD-100 à IVD-305, une retirée), 46 règles, 68 cas limites, 14 lots (DEM0–DEM13), 0 JSON (D-036). Passe personas : liste électorale jamais affichée en libre accès, mandats clos par événement, accord exprès des parents recueilli à la rentrée (§31). Parcours minimal pour la V1 de février 2027 : DEM0–DEM3 + DEM5 ; élections de parents d'octobre 2027 en **doublon** dans le privé pilote (pas d'élection officielle de parents au CA dans le privé) : DEM1, DEM2, DEM4.

---

## 1. Thèse : une composition datée, des mandats, des séances, des délibérations

| Objet | Ce qu'il est | Ce qui se passe mal aujourd'hui [H] | Ce que fait ScolIA |
|---|---|---|---|
| **Instance** | Un organe (CA, CVL, conseil d'établissement…) d'un type, avec une **composition** par collège et par qualité | Composition dans un tableur du secrétariat de direction, jamais à jour | Composition déclarée par type, contrôlée (sièges, qualités), lue à la date |
| **Mandat** | Une personne siège à un titre (élu parents, élu personnels, de droit, désigné, invité), titulaire ou suppléant, du … au … | Suppléant oublié ; élu dont l'enfant est parti qui continue de recevoir les convocations ; élève majeur sorti | Mandat daté, clos par les faits du cœur (départ, perte de qualité), remplacement proposé |
| **Élection** | Procédure à étapes datées, corps électoral figé, scrutin secret, dépouillement, proclamation, recours | Liste électorale extraite à la main, parents séparés oubliés, calcul des restes faux, PV ressaisi dans l'application nationale | Calendrier calculé, corps électoral calculé et figé, calcul des sièges, PV généré, export pré-contrôlé |
| **Séance** | Réunion de l'instance (logistique du 19) avec un ordre du jour délibératif | Quorum compté à la main, procuration mal comptée | Quorum calculé à l'ouverture **et à chaque vote**, procurations bornées |
| **Délibération** | Décision votée, numérotée, avec objet, résultat, caractère exécutoire, publicité | Voyage parti avant d'être exécutoire ; tarif appliqué avant le vote | Port `deliberation_executoire` consommé par les socles concernés |

---

## 2. Évaluation franche de l'existant

### 2.1 ENT Rust [F]
- Aucune migration ni table d'instance, d'élection, de mandat ou de délibération (recherche par mots-clés sur les 231 migrations et le code, 2026-11-11).
- Le mot « délégué » n'apparaît que dans `172_delegation.sql`, `560_delegation_droits_normalises.sql` et `web/.../modules/delegations/` : **délégations de droits** entre personnels (socle 1), sans rapport ; à ne pas confondre dans le nommage (le 46 parle de **représentants** et de **mandats**, R-IVD34).
- Fonctions du socle 6 (353) : `directeur_delegue_formations` (personnel), aucune fonction d'élu [F] ; normal, un mandat n'est pas une fonction.
- Les conceptions du 13, du 19 et du 23 prévoient une **saisie locale de repli** (13 lot D5 : « à défaut de socle 46, saisie locale de la composition » ; 23 : « délégués du socle 46 ou désignés par la vie scolaire »). Le 46 **reprend** ces saisies (§27, DEM0) pour éviter trois sources de vérité.

### 2.2 Next.js [F]
Aucun module d'élections, d'instances, de conseil d'administration ou de vote (recherche par noms de fichiers et de routes ; ni le T0 ni la matrice n'en listent). Rien à reprendre.

### 2.3 Charlemagne [H]
Matrice : VOY-15 « Élections (délégués, conseil d'administration, parents), vote secret » en « P(S) » (présent chez au moins un concurrent, non vérifié). Les logiciels de vie scolaire proposent en général des listes électorales, des étiquettes et l'aide au dépouillement [H] ; aucune documentation lue.

### 2.4 Verdict
Construction neuve, avec **des consommateurs qui attendent** : l'ordre des lots est guidé par eux (DEM0 mandats et lecture ; DEM1 délégués de classe pour les conseils du 1ᵉʳ trimestre ; DEM5 délibérations pour le 42).

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps [H] | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Chef d'établissement** (président du CA dans le public) | Organise les élections, préside le bureau, convoque et préside le CA, signe les actes | Calendrier électoral refait chaque année ; composition recalculée ; actes transmis à la main | **Annulation d'une élection** ou d'une délibération pour vice de forme (délai, quorum, électeur oublié) | Un calendrier électoral calculé, des contrôles qui bloquent l'irrégularité, des actes transmis et exécutoires suivis |
| **Adjoint, secrétaire de direction** | Prépare les listes, le matériel, les PV, la saisie dans l'application nationale | Extraction des parents, publipostage des bulletins, comptage des enveloppes, ressaisie des résultats | Parent séparé sans matériel, adresse protégée imprimée sur une étiquette | Liste électorale juste du premier coup, étiquettes et enveloppes générées, résultats exportés |
| **Gestionnaire, adjoint gestionnaire (public) ; directeur et comptable de l'OGEC (privé)** | Présente le budget et les décisions modificatives ; suit les délibérations financières | Délibérations retrouvées dans des PV papier | Acte non exécutoire appliqué | Registre des actes cherchable, lien avec le 48 |
| **CPE** | Organise les élections des délégués, forme les délégués, anime le CVL et l'AG des délégués, suit l'engagement | Organisation classe par classe, résultats ramassés sur papier | Délégué jamais formé, classe sans délégué au conseil | Élections de délégués menées en une semaine, tableau de bord par classe, formation suivie |
| **Professeur principal** | Organise l'élection en classe (heure de vie de classe) | Bulletins à préparer, dépouillement à refaire | Égalité mal tranchée | Guide en 3 écrans, résultat immédiat avec la règle appliquée |
| **Élève délégué de classe** | Représente la classe au conseil de classe, à l'AG des délégués | Ne sait pas quand ont lieu les conseils ; absence non justifiée quand il siège | Être tenu responsable des propos de la classe | Agenda de ses mandats, absence justifiée automatiquement, espace d'échange avec la classe encadré |
| **Lycéen élu au CVL, vice-président du CVL** | Projets, budget de la vie lycéenne, siège au CA | Pas d'outil pour porter un projet ; budget opaque | Projets bloqués sans réponse | Suivi de projet avec réponse datée, budget lisible |
| **Collégien élu au CVC** | Idem au collège | — | — | Idem |
| **Élève majeur** | Électeur et éligible au même titre ; parfois électeur aux élections de parents ? (non : ses parents restent électeurs [NV]) | — | Recevoir le matériel de ses parents | Clarté |
| **Parent** (dont séparé, recomposé, tuteur) | Électeur, candidat, élu au CA, au conseil de classe | Ne reçoit pas le matériel ; ne sait pas qui sont ses élus | Que son adresse soit donnée à une association ; qu'on sache pour qui il vote | Vote simple ; annuaire des élus ; écrire à « ses » représentants sans exposer son adresse |
| **Parent élu** | Siège, prépare, rend compte aux familles | Documents reçus la veille ; pas de moyen d'écrire aux parents de la classe | Être sollicité sur son numéro personnel | Documents dans le délai, boîte des représentants (16), publication (17) |
| **Association de parents** (FCPE, PEEP, APEL, liste indépendante) | Dépose une liste, une profession de foi, diffuse | Inégalité de diffusion entre listes ; demande des coordonnées | Que l'établissement favorise une autre liste | Canal égal, même date, même format |
| **Personnel** (enseignant, CPE, AED, AESH, agent, administratif) | Électeur et éligible au CA dans son collège | Ne sait pas s'il est électeur (contractuel, temps partagé) | Être oublié | Statut d'électeur expliqué par le serveur (« électeur dans le collège enseignement et éducation, car… ») |
| **Membres de droit, personnalités qualifiées, représentants de la collectivité** | Siègent au CA | Convocations par courrier ; documents en pièce jointe lourde | — | Accès invité aux seuls documents de séance, sans compte complet |
| **Secrétaire de séance** | Rédige le PV | Notes à remettre en forme | PV contesté | Trame du PV préremplie (présents, quorum, votes), IA qui met en forme ses notes |
| **Collectivité, rectorat, DSDEN** (contrôle de légalité, résultats) | Reçoivent les actes et les résultats d'élections | Actes illisibles, résultats en retard | — | Actes transmis complets, résultats remontés |
| **Diocèse, DDEC, tutelle (privé)** | Approuve, nomme le chef d'établissement, suit le conseil d'établissement | — | — | Composition et PV disponibles |
| **AESH, AED** | Électeurs dans le collège des personnels d'enseignement et d'éducation [NV] | Souvent oubliés | — | Inscrits automatiquement s'ils remplissent les conditions |
| **DPO** | Contrôle listes électorales, résultats nominatifs, conservation | Listes qui circulent par e-mail | Fuite de la liste des parents | Listes jamais exportées avec coordonnées ; purges datées |

---

## 4. L'année démocratique vue du terrain [H, dates à vérifier §5]

| Période | Ce qui se passe | Ce que fait ScolIA |
|---|---|---|
| **Juin (N-1)** | Calendrier de l'année suivante ; fin des mandats annuels ; CVL renouvelé par moitié [NV] | Calendrier électoral **préparé** depuis la note de service de l'année (couche nationale tenue par l'opérateur, comme `09_INTEROP.md` ITP-152) |
| **Septembre** | Élections des délégués de classe (premières semaines) ; listes électorales des parents et des personnels arrêtées et affichées ; appel à candidatures ; AG des délégués avant la fin de la 7ᵉ semaine (R421-42, `19_AGENDA_RDV.md` §6.3 [V]) | Campagne « délégués » par classe ; corps électoraux calculés, contrôlés, **figés** ; affichage minimisé ; dépôt des listes |
| **Début octobre** | Élections des représentants des parents (dates nationales, `19_AGENDA_RDV.md` §6.8 [NV]) et des personnels au CA ; CVL, CVC | Matériel, correspondance, bureau, dépouillement, PV, export ; proclamation ; recours |
| **Octobre–novembre** | Installation du CA (élection de la commission permanente, des membres du conseil de discipline et de la commission éducative [NV]) ; premier CA ; CVL réuni avant le CA | Élections internes au CA ; compositions calculées pour le 13 ; séance du 19 avec ordre du jour délibératif |
| **Novembre–décembre** | Conseils de classe du 1ᵉʳ trimestre (délégués) ; CA budgétaire (souvent novembre–décembre) [H] | Délégués lus par le 23 ; délibération budgétaire → 48 ; transmission ; exécutoire |
| **Janvier–juin** | CA (répartition des moyens, voyages, tarifs, règlement intérieur, conventions) ; projets du CVL ; AG des délégués | Délibérations par objet ; projets suivis ; budget de la vie lycéenne |
| **Fin d'année** | Bilan des instances, archives, engagements des élèves au livret (23) | Engagements générés depuis les mandats ; compteurs « au moins 3 fois par an » (19 AGD-340) ; versement aux archives (27) |

---

## 5. Ce qui est réglementaire (vérifié le 2026-11-11)

Recherche menée le 2026-11-11 par un sous-agent. Légifrance a répondu mais a été lu **par fragments résumés** par l'outil de lecture : ces lignes sont marquées **[V-rés]** et doivent être relues mot à mot par un humain avant le code des lots DEM1 à DEM6 (Q-IVD1). education.gouv.fr (BO), eduscol et ih2ef ont renvoyé 403 ou 503.

### 5.1 Conseil d'administration de l'EPLE
| Règle | Source | Statut | Conséquence |
|---|---|---|---|
| **30 membres** (collège ou lycée) : chef d'établissement, adjoint, gestionnaire, CPE ; 2 représentants de la collectivité, 2 de la commune ; 1 personnalité qualifiée (2 si l'administration compte moins de 5 membres) ; **10 élus des personnels (7 enseignement-éducation, 3 ATSS)** ; **10 parents et élèves : collège 7 + 3, lycée 5 + 5** ; au moins 1 élève post-bac s'il y a des classes post-bac ; lycée professionnel : 2 personnalités du monde économique en plus | C. éduc. **R421-14** (version du 19-9-2016) — legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000018380776/ | [V-rés] | `ref_instance_sieges` par catégorie d'établissement |
| **Collège de moins de 600 élèves sans section d'éducation spécialisée : 24 membres** : 8 personnels (6 + 2), 8 parents et élèves (6 + 2), 2 collectivité, 1 commune, 1 personnalité qualifiée (2 si l'administration compte moins de 4 membres) | **R421-16** (3-11-2014) | [V-rés] | seuil de 600 élèves et présence d'une SEGPA lus au 5 (effectif à la date de référence [NV]) |
| Élus siégeant **un an**, jusqu'à la première réunion qui suit le renouvellement ; règles sans condition de nationalité ; **un élu ne siège que dans une catégorie** ; personnalités qualifiées 3 ans ; inéligibilité si privation de droits civiques ou de famille | **R421-29, R421-34, R421-36** | [V-rés] | mandat annuel clos à la première réunion du nouveau CA (pas au 31 août) ; contrôle de cumul de catégories ; inéligibilité = **déclaration** du candidat, aucun casier stocké (R-IVD35) |
| **Personnels** : deux collèges (1) enseignement, direction, éducation, surveillance, assistance éducative ou pédagogique, documentation ; (2) administratifs, santé, sociaux, techniques, ouvriers, service, laboratoire ; **non-titulaires électeurs s'ils travaillent au moins 150 h par an, éligibles s'ils sont nommés pour l'année** | **R421-26** (19-3-2008) | [V-rés] ; place des AED et AESH au collège 1 : interprétation [NV] | électorat calculé par le 6 (contrat, volume horaire annuel) avec l'explication |
| **Parents** : chaque parent électeur et éligible, sauf retrait de l'autorité parentale ; **une seule voix** quel que soit le nombre d'enfants | **R421-26** | [V-rés] | R-IVD6 ; port `liens::responsables_electeurs` du 4 |
| Scrutin de **liste à la proportionnelle au plus fort reste** ; **égalité de restes → la liste qui a le plus de voix, puis le candidat le plus âgé** ; liste d'**au plus le double** des sièges et **au moins 2 noms** (listes incomplètes admises) | **R421-26, R421-30** | [V-rés] | §9.2 précisé ; jeux d'or sur les égalités |
| Élections **au plus tard avant la fin de la 7ᵉ semaine** ; liste électorale arrêtée **20 jours** avant ; candidatures **10 jours francs** avant ; vote à l'urne et par correspondance, **électronique pour les parents** ; « votes personnels et secrets » ; **contestation devant le recteur sous 5 jours ouvrables** après la proclamation, réponse sous 8 jours (silence = rejet) | **R421-30** (version du 24-8-2023, décret 2023-805) | [V-rés] | calendrier calculé (R-IVD9), minuteur de recours (IVD-160) |
| **Élèves au CA** : délégués de classe (**2 par classe**, scrutin **uninominal à deux tours**, tous électeurs et éligibles) ; **collège : les délégués élisent entre eux les élus au CA, éligibles à partir de la 5ᵉ** ; lycée : élus **parmi les membres du CVL**, avec élection simultanée du **vice-président du CVL** ; suppléants dans l'ordre des voix ; aucun tirage au sort prévu | **R421-28** (19-9-2016), **R421-35** | [V-rés] | élections à deux degrés enchaînées (classe → AG des délégués ou CVL → CA) |
| Au moins **3 séances** ordinaires par an ; extraordinaire à la demande du recteur, de la collectivité, du chef d'établissement ou de la moitié des membres ; convocation **8 jours** avant (1 jour en urgence) ; **quorum : majorité des membres en exercice**, sinon nouvelle réunion sous **5 à 8 jours sans quorum** ; **séances non publiques** ; votes personnels, **vote secret de droit à la demande d'un membre**, **voix prépondérante du président** | **R421-25** (24-12-2020), R421-19, R421-24 | [V-rés] (concorde avec `19_AGENDA_RDV.md` §6.3 [V]) | `ref_instance_regle` ; voix prépondérante ajoutée au calcul d'adoption (R-IVD36) |
| Attributions : délibérations (R421-20 : budget, compte financier, règlement intérieur, projet d'établissement…) ou simples avis (R421-23) ; délégations à la commission permanente pour les points 6° à 10° et 12° de R421-20, décidées à la **première séance** | **R421-20, R421-22, R421-23** | [V-rés] | `ent_deliberation.portee` (décision / avis) et « déléguée à la commission permanente » (§31) |
| **Commission permanente** : chef d'établissement, adjoint, gestionnaire, 1 représentant de la collectivité, 4 personnels, parents (3 en collège, 2 en lycée), élèves (1 en collège, 2 en lycée) | **R421-37** (24-12-2020), R421-38 et s. (mode d'élection non lu en entier) | [V-rés] | type dérivé, élu en interne (IVD-172) |
| Visioconférence (ordonnance 2014-1329, décret 2014-1627) : application au CA non confirmée | — | [NV] | **aucune séance à distance** du CA proposée (option masquée) |
| PV du CA, secrétaire de séance : aucun article lu | — | [NV] | trame recommandée, pas de règle codée |

### 5.2 Actes et contrôle de légalité
| Règle | Source | Statut | Conséquence |
|---|---|---|---|
| Actes relatifs au fonctionnement, conventions, marchés : transmis au représentant de l'État ou à l'autorité académique, certains exécutoires dès transmission, d'autres 15 jours après ; actes relatifs à l'action éducatrice : transmis à l'autorité académique, **exécutoires 15 jours après**, annulation possible | **L421-14** I et II | [V-rés] | `ref_acte_regime` (§31) : immédiat ou + 15 jours selon la nature de l'objet |
| **Contrats, recrutements, voyages scolaires** : exécutoires **15 jours** après transmission | **R421-54** (21-8-2019) | [V-rés] | 42 : `deliberation_executoire_le = transmise_le + 15 j` (cohérent avec le 42) |
| Règlement intérieur, structure pédagogique, dotation horaire, temps scolaire, projet d'établissement : transmis au recteur | **R421-55** | [V-rés] | objets `reglement_interieur`, `structure`, `projet_etablissement` |
| Accès aux actes sur demande (État, recteur, collectivité) | **R421-56** | [V-rés] | export du registre (IVD-210) |
| Plateforme Dém'Act, affichage obligatoire des actes | — | [NV] | dépôt **déclaré** (IVD-203) ; publicité selon Q-IVD8 |

### 5.3 Élections des parents : calendrier national et associations
| Règle | Source | Statut | Conséquence |
|---|---|---|---|
| 2026-2027 : note de service du 25-6-2026 (BO n° 30 du 23-7-2026, NOR MENE2615797N) : scrutin **vendredi 9 ou samedi 10 octobre 2026** | BO 30/2026 | [NV] (403 ; connu par copies et moteurs) | couche nationale (IVD-116) |
| Report du 1ᵉʳ degré dans les académies de Nantes et Toulouse au 13 ou 14 novembre 2026 ; calendrier : liste électorale **20 jours** avant ; candidatures **10 jours francs** ; remplacement d'un candidat **8 jours francs** ; matériel de vote par correspondance **6 jours** avant ; **vote électronique de 24 h à 5 jours** ; contestations sous **5 jours** | note de service du 17-9-2026, MENE2624426N, BO n° 36 du 24-9-2026 | [V] | le calendrier national peut être **reporté par académie** (couche académique, §31) |
| Adaptations possibles des modalités de vote en cours d'opérations (mouvement lycéen) | BO n° 38 du 8-10-2026, MENE2626426C | [NV] | changement de modalité en cours de scrutin par décision tracée (IVD-282, §31) |
| **Liste des parents (noms, adresses postale et électronique) communicable aux associations seulement avec l'« accord exprès » des parents** ; panneau et boîte aux lettres ; **documents des associations distribués sans contrôle a priori** (laïcité, vie privée, pas de diffamation ni de propagande ; arbitrage de l'autorité académique sous 7 jours) ; mêmes règles pour les candidats **dans les 4 semaines** précédant le scrutin ; élus tenus à la confidentialité | **D111-7 à D111-15** (décret 2006-935) | [V-rés] | **R-IVD12 corrigée** (§31) : consentement exprès par parent, horodaté et révocable (18) ; export limité aux consentants ; **IVD-137 remplacée** : le chef d'établissement ne refuse pas une profession de foi, il saisit l'autorité académique (§31) |
| Bureau des élections, carence et tirage au sort en EPLE, remontée des résultats (note technique DGESCO du 21-7-2026, non lue) | arrêté du 13-5-1985 modifié | [NV] | Q-IVD2 ; IVD-139 reste [NV] |

### 5.4 Élèves
| Règle | Source | Statut | Conséquence |
|---|---|---|---|
| **CVL** : **10 lycéens élus pour 2 ans, renouvelés par moitié chaque année**, **scrutin plurinominal à un tour**, **égalité : le plus jeune est élu**, un suppléant par titulaire ; autant d'adultes à voix consultative (5 enseignement-éducation, 3 autres personnels, 2 parents élus par les parents du CA) ; présidé par le chef d'établissement ; vice-président lycéen ; réuni avant chaque CA ordinaire ; candidatures au moins 10 jours avant ; élection avant la fin de la 7ᵉ semaine | **R421-43** (19-9-2016), **R421-44**, **R421-45** (29-1-2010) | [V-rés] ; corps électoral exact [NV] | mode `plurinominal_1_tour` ajouté (§31) ; demi-cohortes ; **pas** de binôme imposé (le §9.3 « binômes » est corrigé) |
| **CVC** : élèves, au moins 2 personnels dont 1 enseignant, au moins 1 parent ; présidé par le chef d'établissement ; **composition, mode d'élection et fonctionnement fixés par délibération du CA** | **R421-45-1, R421-45-2** (2-12-2016) | [V-rés] | type paramétré par une délibération exécutoire (lien `ent_instance.deliberation_id`, §31) |
| **AG des délégués** : au moins 2 réunions par an, dont une avant la fin de la 7ᵉ semaine ; conseil de classe : 2 délégués élèves et 2 délégués parents | **R421-42**, **R421-50** | [V-rés] | délégués de classe élus avant l'AG ; date limite propre [NV] |
| **Éco-délégués** : au moins un éco-délégué élu par classe de collège et de lycée, plus un binôme d'établissement issu du CVC ou du CVL | circulaire du 24-9-2020, MENE2025449C, BO n° 36 | [V-copie] | type `eco_delegue` par classe (élection), plus binôme |
| CAVL, CNVL, ambassadeurs pHARe, formation des délégués | — | [NV] | instances libres |

### 5.5 Personnels, discipline, autres instances
| Règle | Source | Statut | Conséquence |
|---|---|---|---|
| **Conseil de discipline (14 membres)** : chef d'établissement, adjoint, CPE désigné par le CA, gestionnaire, 5 personnels, parents et élèves (collège 3 + 2, lycée 2 + 3) ; élus **chaque année, en leur sein, par les titulaires et suppléants du CA de leur catégorie**, à la **première réunion** du CA (proportionnelle au plus fort reste ; uninominal pour l'ATSS) ; élèves de lycée élus par les délégués à leur première AG | **R511-20** (21-5-2009), R511-21, **R511-22** (19-9-2016), R511-20-1 (18-8-2023) | [V-rés] | IVD-173 : corps électoral = mandats du CA de la catégorie ; lu par le 13 (DIS-105) |
| Commission éducative | R511-19-1 | [NV] | composition saisie |
| **Conseil pédagogique** : membres **désignés par le chef d'établissement** après consultation des équipes ; au moins 3 réunions par an | R421-9 11°, R421-41-1 et s. | [V-rés] | fondement `designation` |
| CESCE (D421-46, D421-47), CHS (D421-151, L421-25), conseil école-collège (D401-1) | — | [V-rés] / [NV] | types désignés, composition à confirmer |

### 5.6 Vote électronique — le point majeur
| Règle | Source | Statut | Conséquence |
|---|---|---|---|
| **Autorisé seulement pour l'élection des représentants des parents au CA de l'EPLE** (R421-30 modifié par le décret 2023-805) | R421-30 ; arrêté du **2-7-2024** (NOR MENE2410568A, JORF n° 0157 du 4-7-2024) — legifrance.gouv.fr/loda/id/JORFTEXT000049871171/ | [V-rés] | R-IVD27 précisée : **parents au CA (public) seulement** |
| **Personnels au CA : urne et correspondance seulement** | R421-30 ; SNES (29-9-2026) | [V-rés], [V-copie] | option électronique **interdite** pour les collèges des personnels |
| **Élèves (délégués, CVL, CA), CVC** : aucun texte trouvé qui l'autorise | R421-28, R421-43, R421-45-1 | [NV] (absence constatée, non prouvée) | **§11.3 suspendu** : pas de vote électronique d'élèves, même en présentiel, sans validation juridique (Q-IVD12, à Florian) |
| Arrêté du 2-7-2024 : **art. 5** prestataire possible, **expertise indépendante avant la mise en place et toute modification substantielle**, conformément à la recommandation CNIL du 25-4-2019 ; **art. 6** données des électeurs et données de vote en traitements **« distincts, dédiés et isolés »**, conservées jusqu'à la fin des recours puis détruites sous contrôle du bureau ; dispositif de secours ; **art. 7** bureau : chef d'établissement président, un secrétaire, **un délégué par liste** ; cellule d'assistance ; **art. 8** le bureau consulte la participation et les émargements ; **art. 9** vote de **24 h à 5 jours**, de tout lieu, **poste dédié** dans l'établissement, notice d'accès remise au moins **6 jours** avant ; **art. 10** dépouillement en présence du chef d'établissement et des délégués de liste, urne et émargement figés, horodatés et scellés, résultats (blancs compris), proposition d'attribution des sièges ; **art. 11** scrutin mixte : l'électronique **précède** l'urne, les plis de ceux qui ont déjà voté sont **écartés sans être ouverts** | arrêté du 2-7-2024 | [V-rés] | **cahier des charges du lot DEM10** (§31) : CIV-20 corrigé (l'électronique fait foi), poste dédié, notice J-6, durée bornée, délégués de liste au bureau |
| Vote électronique **exclusif** possible sur décision du chef d'établissement après consultation du CA | fiche DPO région académique Bourgogne-Franche-Comté (juillet 2025) | [V-copie] ; non lu dans l'arrêté | R-IVD26 maintenue (jamais seule modalité) par défaut ; exclusif = décision tracée **avec** repli par électeur (droit d'opposition) |
| **CNIL, recommandation du 19-3-2026** (remplace celle de 2019, applicable jusqu'à fin 2026) : **niveau 1, exemple cité : « élections de représentants de parents d'élèves dans les établissements scolaires »** ; objectifs : vote indivisible (validation + urne + émargement + récépissé), authentification, confidentialité dès le poste, étanchéité identité/vote, **secret de dépouillement réparti au sein du bureau avec seuil**, dépouillement après clôture sur toute l'urne, urne vide vérifiée à l'ouverture, dépouillement vérifiable a posteriori ; scellés jusqu'à la fin des recours puis destruction ; pas de double vote ; expertise facultative aux niveaux 1 et 2 (rapport réutilisable moins de 24 mois) | cnil.fr/sites/default/files/2026-04/recommandation_vote_electronique.pdf | [V] | architecture du §11.2 confirmée (seuil, séparation, scellement, récépissé) ; l'arrêté **impose** l'expertise (il prévaut) |
| Formulaires et sondages d'un ENT : insuffisants pour une élection | fiche DPO BFC | [V-copie] | confirme R-FC31 du 18 |
| Responsable de traitement = chef d'établissement ; contrat de sous-traitance (art. 28 RGPD), registre, **AIPD au-delà d'environ 1 000 électeurs** ; droit d'opposition pour motif exceptionnel, repli sur correspondance ou urne | fiche DPO BFC | [V-copie] | kit RGPD du scrutin (IVD-283, §31) |

### 5.7 Privé sous contrat
| Règle | Source | Statut | Conséquence |
|---|---|---|---|
| « Tout établissement catholique d'enseignement se dote d'un **conseil d'établissement** », présidé par le chef d'établissement, tutelle de droit, représentants de toute la communauté (enseignants, salariés de l'organisme, élèves, parents, organisme, propriétaire, pastorale, direction, anciens élèves) ; membres **élus ou désignés** ; au moins **2 réunions par an** ; **voix consultative** | Statut de l'Enseignement catholique (2013, édition 2018), art. 120 à 123 — copie apel.fr | [V-copie] | type `conseil_etablissement_ec` : consultatif, 2 séances par an, aucun contrôle de légalité |
| Une association de parents dans chaque école catholique ; l'Apel est l'association reconnue | Statut, art. 299 à 302 | [V-copie] | §12 ; élections internes de l'Apel hors ScolIA |
| **Aucun texte** ne fixe la composition des conseils de classe ni la participation des parents et des élèves dans le privé sous contrat (question écrite n° 17519, JO du 4-8-2026, sans réponse) | questions.assemblee-nationale.fr/q17/17-17519QE.htm | [V] | délégués, CVL et CVC **facultatifs** dans le privé, activables ; mêmes moteurs |
| Statuts types de l'OGEC, CSE, instances des maîtres contractuels | — | [NV] | §12 |

### 5.8 Archives
Tableau de tri et de conservation (encart du BO n° 24 du 16-6-2005, education.gouv.fr/bo/BoAnnexes/2005/24/tableaux_encart24.pdf) [V] : dossiers d'élections aux conseils **2 mandats** puis tri (garder les années en 0 et 5) ; **professions de foi 1 an, conservées** ; **bulletins 3 mois**, tri (un exemplaire) ; **listes électorales et circulaires 2 mandats, détruites** ; **PV de dépouillement 2 mandats**, tri (un mandat sur deux) ; **PV du CA 5 ans, conservation définitive** ; conseil de discipline 10 ans (PV seul gardé). Aucune élimination sans visa (code du patrimoine R212-14). **§17.3 remplacé** par ces durées (§31).

### 5.9 Contradictions entre sources
1. L'arrêté de 2024 renvoie à la recommandation CNIL de 2019, remplacée le 19-3-2026 : viser la conformité aux deux.
2. Expertise facultative au niveau 1 (CNIL 2026) mais imposée par l'arrêté : l'arrêté prévaut.
3. Une circulaire académique cite un « arrêté du 10 juillet 2024 » ; Légifrance donne le 2 juillet 2024.
4. Clés réparties : exigées par la CNIL, absentes de l'arrêté.
5. Dates 2026 : report du seul 1ᵉʳ degré à Nantes et Toulouse (un site commercial le présente à tort comme général).
6. R421-16 dit « section d'éducation spécialisée » ; les textes administratifs parlent de SEGPA.

### 5.10 Non vérifié (récapitulatif)
Note de service du 25-6-2026 et note du BO n° 38 ; arrêté du 13-5-1985 en vigueur ; carence de candidats parents en EPLE ; remontée des résultats (application, format) ; PV du CA ; Dém'Act et affichage des actes ; visioconférence ; corps électoral du CVL ; date limite des délégués de classe ; commission éducative, CHS, conseil école-collège ; texte de 2019 de la CNIL ; statuts OGEC et Apel, CSE ; numéro de l'instruction d'archives de 2005. **Toutes les lignes [V-rés] sont à relire mot à mot sur Légifrance.**

---

## 6. Branchements sur le cœur et frontières

### 6.1 Qui possède quoi
| Objet | Propriétaire | Rôle du 46 |
|---|---|---|
| Élèves, classes, niveaux, scolarisation, majorité | 2, 5 | lit : électeurs et éligibles élèves ; consomme `eleve.sortie`, `eleve.majeur`, changement de classe |
| Responsables, autorité parentale, droits, suspensions, adresse protégée | 4 | lit : électeurs parents **à la date** ; jamais d'adresse dans une liste affichée |
| Personnels, engagements, fonctions, temps partagé | 6 | lit `personnels::electeurs(college, date)` (port déjà prévu par le 6) |
| Documents (PV, actes, professions de foi, matériel de vote) | 7 | dépose par le kit fichiers (E5) ; signature des actes et PV par le 7 |
| Envois (convocations : 19 ; matériel de vote, résultats, rappels) | 8 | déclare ses natures (§24) |
| Échanges nationaux (résultats des élections, actes) | 9 | déclare ses flux (§26) |
| Absences d'élèves pour mandat | 12 | motif « mandat de représentant » proposé quand l'élève siège (R-IVD25) |
| Conseil de discipline, commission éducative : **contenu** | 13 | fournit la **composition à la date** et les **élus** (port) |
| Boîte des représentants de parents | 16 | fournit les membres (élus de l'année) |
| Publications des représentants, professions de foi, résultats | 17 | fournit les mandats (droit `publications.representants.rediger`) et le calendrier |
| Sondages, consultations | 18 | aucun scrutin au 18 ; renvoi au 46 |
| **Logistique des séances** (convocation, délais, réponses, présence, PV déposé) | **19** | demande la réunion (AGD-141), fournit la règle de quorum (AGD-132) et les participants (AGD-122) |
| Conseil de classe : **contenu** | 23 | fournit les délégués à la date ; engagements LSL (`lsl_engagement.mandat_id`) |
| Tarifs, barèmes, régies | 24, 45, 40 | consomment la délibération exécutoire |
| Voyages | 42 | consomme la délibération (`deliberation_transmise_le`, `deliberation_executoire_le`) |
| Budget, décisions modificatives, compte financier, conventions | 48 | consomme ; le 46 ne calcule aucun budget |
| Archives, conservation | 27 | verse |
| Indicateurs (participation, compteurs réglementaires) | 47 | agrégats |

### 6.2 Ports consommés
| Port | Socle | Usage |
|---|---|---|
| `eleves::actifs(etab, date, classe?)` | 2, 5 | corps électoraux élèves |
| `liens::responsables_electeurs(eleve, date)` | 4 | corps électoral parents (autorité parentale non retirée) — **nouveau port demandé au 4** (Q-IVD3) |
| `personnes::foyer_postal(personne)` | 4 | envoi du matériel de vote, **sans** exposition de l'adresse dans le 46 |
| `personnels::electeurs(college, date)` | 6 | corps électoraux personnels |
| `agenda::ouvrir_reunion(origine='46', ...)`, `agenda::presences(evenement)` | 19 | séances, quorum |
| `documents::deposer`, `documents::signer` | 7 | PV, actes, matériel |
| `diffusion::envoyer(nature, destinataires_regle)` | 8 | matériel, résultats |
| `echanges::exporter(connecteur)` | 9 | résultats, actes |

### 6.3 Ports exposés
| Port | Pour | Contenu |
|---|---|---|
| `instances::composition(type|instance, date)` | 13, 19, 23 | membres, qualité, collège, voix, titulaire/suppléant |
| `instances::mandats_de(personne, date)` | 12, 16, 17, 23, 29 | mandats en cours |
| `instances::delegues(classe, date)` | 23 | délégués élèves et parents de la classe |
| `instances::regle_quorum(instance, seance)` | 19 | membres en exercice, quorum requis, règle de seconde convocation |
| `instances::deliberation_executoire(objet_type, objet_id)` | 24, 40, 42, 45, 48, 5 | délibération, date d'exécution, ou « aucune » |
| `instances::engagements(eleve, annee)` | 23 | mandats d'élève (livret) |
| `instances::calendrier_electoral(etab, annee)` | 17, 19, 29 | échéances |

### 6.4 Répercussions automatiques
| Fait du cœur | Effet au 46 |
|---|---|
| `eleve.sortie` (élève élu) | mandat clos à la date ; remplacement proposé (suppléant, suivant de liste, nouvelle élection) |
| `eleve.sortie` (dernier enfant d'un parent élu) | mandat de parent **perd sa base** : clos selon la règle [NV, §5] ; remplacement par le suivant de liste |
| Changement de classe d'un délégué | mandat de délégué de l'ancienne classe clos ; aucune délégation automatique dans la nouvelle |
| `liens.autorite_retiree` / suspension | parent retiré de la liste électorale **non figée** ; si figée, signalement au bureau (pas de modification silencieuse) ; mandat éventuel examiné [NV] |
| `personnel.engagement_clos` | mandat de personnel clos ; suivant de liste |
| `eleve.decede`, décès d'un élu | mandat clos sans message automatique ; tâche au chef d'établissement |
| `eleve.majeur` | aucun effet sur les élections de parents [NV] ; l'élève est informé de ses propres droits |
| Fusion de deux fiches personne (4) | un seul électeur ; doublon d'émargement détecté |
| Fermeture de l'établissement le jour du scrutin (5) | scrutin reporté à proposer ; aucune clôture automatique |

---

## 7. Concepts et vocabulaire [R]
| Terme | Définition |
|---|---|
| **Type d'instance** | Modèle d'un organe (CA d'EPLE, commission permanente, conseil pédagogique, CVL, CVC, AG des délégués, conseil de discipline, commission éducative, CESCE, CHS, conseil école-collège, conseil d'établissement du privé, CA et AG de l'organisme de gestion, conseil de classe pour les délégués…) avec ses **règles** (composition, quorum, délais, durée des mandats) attachées au régime (public, privé sous contrat, hors contrat) — table de référence globale versionnée |
| **Instance** | Un organe réel d'un établissement (ou d'un organisme de gestion), pour une période |
| **Collège** (de l'instance) | Catégorie de sièges : membres de droit, élus des personnels d'enseignement et d'éducation, élus des personnels ATSS, élus des parents, élus des élèves, représentants de la collectivité, personnalités qualifiées… |
| **Siège** | Une place d'un collège ; titulaire et suppléant |
| **Mandat** | Une personne sur un siège, à une qualité, avec un **fondement** (élection, désignation, de droit) et des dates |
| **Corps électoral** | Ensemble des électeurs d'un scrutin, **calculé** à une date, puis **figé** (liste électorale) |
| **Scrutin** | Un vote d'élection : mode (liste proportionnelle au plus fort reste, uninominal à deux tours, binôme, désignation par une instance), modalités (urne, correspondance, électronique) |
| **Candidature** | Une liste ordonnée (titulaires, suppléants) ou un candidat, ou un binôme |
| **Bureau** | Président, assesseurs, secrétaire du scrutin |
| **Émargement** | Preuve qu'un électeur a voté, **séparée** de l'urne |
| **Séance** | Réunion d'une instance (logistique au 19) |
| **Point de l'ordre du jour** | Information, débat ou **vote** ; un vote produit une délibération |
| **Délibération** | Décision votée, numérotée, avec un objet typé, un résultat, un caractère exécutoire |
| **Acte** | Délibération ou décision du chef d'établissement soumise à transmission et publicité |
| **Consultation** | Sondage (18), sans valeur de scrutin — **pas** au 46 |

---

## 8. Instances, compositions, mandats

### 8.1 Référentiel des types [R]
Table de référence **globale et versionnée** (E12), tenue par l'opérateur à partir des textes (§5) : pour chaque type et régime, les collèges, le nombre de sièges selon la catégorie d'établissement (effectif, présence d'une SEGPA, collège ou lycée [§5]), la durée du mandat, la règle de quorum, les délais, la périodicité minimale, la publicité des séances. Un établissement **instancie** un type et ne peut pas en modifier les règles réglementaires ; il peut créer des **instances libres** (comité de pilotage, commission menus, conseil de vie de l'internat, comité de la maison des lycéens) sans règle réglementaire.

### 8.2 Composition
- Calculée à la date : membres de droit (fonctions du 6 : chef d'établissement, adjoint, gestionnaire, CPE…), élus (mandats), désignés, représentants extérieurs (personnes du 4 de type `tiers`, sans compte obligatoire).
- **Contrôles** : sièges vacants, collège incomplet, cumul interdit [NV], suppléant sans titulaire.
- **Voix** : délibérative ou consultative ; invités sans voix.
- **Changement d'un membre de droit** (nouveau chef d'établissement) : le mandat suit la fonction, sans élection.

### 8.3 Mandats
Fondement (`election`, `designation`, `de_droit`, `tirage_au_sort`), scrutin d'origine, qualité (titulaire, suppléant), rang dans la liste, du, au prévu, clos le, motif de clôture (`fin`, `demission`, `perte_qualite`, `deces`, `annulation`, `remplacement`) ; **remplacement** : le serveur propose le suppléant ou le suivant de liste selon la règle du type ; en l'absence, siège vacant ou élection partielle [NV].

### 8.4 Saisie manuelle (DEM0)
Avant les élections dans ScolIA (et pour les instances du privé), un mandat peut être **saisi** avec une pièce (PV d'élection papier) : c'est la source de vérité pour le 13 et le 23. Les saisies locales prévues par le 13 (lot D5) et le 23 sont **converties** en mandats (§27).

---

## 9. Élections : procédure commune

### 9.1 Étapes et calendrier [R]
1. **Ouverture** du scrutin (type, instance, sièges à pourvoir, modalités) ; calendrier **calculé** depuis la date du scrutin et les délais du type (§5) ; dates nationales importées de la couche nationale quand elles existent (parents).
2. **Corps électoral calculé** à la date de référence ; **contrôles** (électeurs sans adresse, parents sans lien vérifié, doublons de personne, élèves sans classe) ; **corrections par les socles propriétaires** (4, 6, 2), jamais dans le 46.
3. **Liste électorale figée** (instantané horodaté, empreinte) ; affichage et consultation **minimisés** (nom, prénom, rien d'autre ; pour les parents : sans nom d'enfant ni classe [R, Q-IVD5]) ; réclamations jusqu'à la date limite ; modification **par décision** du chef d'établissement après réclamation, tracée (R-IVD7).
4. **Candidatures** : dépôt jusqu'à la date limite ; contrôle d'éligibilité de **chaque** candidat sur la liste figée ; nombre de candidats (au plus le double des sièges pour une liste [NV]) ; ordre ; accord écrit de chaque candidat (18) ; liste incomplète permise selon le type [NV].
5. **Matériel** : professions de foi (format imposé, même date de diffusion pour toutes, 17 R-PB36), bulletins, enveloppes, notice ; pour la correspondance : double enveloppe, émargement à la réception ; envoi par le 8 (cartable, courrier, voie électronique du matériel **d'information**).
6. **Scrutin** : bureau constitué ; urne (papier) ; correspondance reçue et émargée ; électronique si ouvert (§11).
7. **Dépouillement** : papier → **saisie des résultats** par le bureau (inscrits, votants, blancs et nuls, suffrages exprimés, voix par liste ou candidat) ; double saisie par deux assesseurs, écart bloquant (R-IVD10) ; calcul automatique des sièges (§9.2) ; égalités tranchées par la règle du type, **affichée** ; tirage au sort, s'il est prévu, fait **en présence du bureau** et consigné (graine publiée, R-IVD11).
8. **Procès-verbal** généré (trame D-023), signé par le président et les assesseurs (7) ; **proclamation** ; affichage des résultats (élus, voix par liste ; aucun résultat par bureau en dessous de 10 votants, R-IVD13) ; export national (9, §26).
9. **Recours** : délai calculé ; recours enregistré ; décision de l'autorité ; annulation éventuelle → mandats annulés, nouvelle élection.
10. **Mandats** créés à la proclamation (ou à la fin du délai de recours, selon le type [NV]).

### 9.2 Modes de calcul [R, règles précises au §5]
- **Liste à la représentation proportionnelle au plus fort reste** (R421-30) : quotient électoral = suffrages exprimés / sièges ; sièges au quotient ; restes ; plus forts restes ; **égalité de restes → la liste qui a le plus de voix, puis le candidat le plus âgé** ; titulaires puis suppléants dans l'ordre de la liste ; liste qui obtient plus de sièges que de candidats → sièges restants attribués aux autres listes [NV].
- **Uninominal à deux tours** (délégués de classe, R421-28) : majorité absolue des suffrages exprimés au premier tour ; relative au second ; égalité → règle d'âge [NV, pratique : le plus jeune].
- **Plurinominal à un tour** (CVL) : les candidats ayant le plus de voix sont élus ; égalité → le plus jeune.
- **Binôme** (éco-délégués d'établissement, instances libres).
- **Désignation par une instance** (membres du conseil de discipline désignés ou élus au sein du CA par collège [NV]).
- **Carence** : aucun candidat → règle du type (tirage au sort parmi des volontaires, siège vacant) [NV].
Chaque calcul est **déterministe, testé sur jeux d'or**, rejouable et affiché pas à pas au bureau (« quotient 212,5 ; liste A : 2 sièges au quotient, reste 87 ; … »).

### 9.3 Élections par famille
| Élection | Électeurs | Éligibles | Mode | Particularités |
|---|---|---|---|---|
| Parents au CA (public) | chaque parent titulaire de l'autorité parentale (ou tiers légal), **une voix par personne** quel que soit le nombre d'enfants [§5] | idem, sauf incompatibilités [§5] | liste, proportionnelle au plus fort reste | dates nationales ; vote par correspondance ; électronique selon §11 ; résultats remontés |
| Personnels au CA (public) | deux collèges (enseignement et éducation ; administratifs, techniques, sociaux et de santé) [§5] | idem | liste, proportionnelle | conditions d'ancienneté ou de quotité [NV] |
| Délégués de classe | élèves de la classe | élèves de la classe | uninominal à deux tours (titulaires et suppléants) [§5] | en classe, heure de vie de classe ; résultats à la vie scolaire |
| CVL | lycéens [corps électoral NV] | lycéens | **plurinominal à un tour**, un suppléant par titulaire, égalité : le plus jeune (R421-44) | 10 élus pour 2 ans, renouvelés par moitié chaque année |
| CVC | selon le CA [§5] | | selon le CA | |
| Élèves au CA (lycée) | instance (CVL, AG des délégués) [§5] | | | |
| Membres du conseil de discipline, commission permanente, commission éducative | membres du CA, par collège [§5] | | | |
| Éco-délégués, délégués d'internat, ambassadeurs | selon l'établissement | | désignation ou élection libre | pas de règle nationale de scrutin [NV] |
| **Privé** : délégués de classe | élèves | élèves | même moteur, règle d'usage | pas d'obligation nationale [NV] |
| **Privé** : parents correspondants, conseil d'établissement | association de parents ou statut interne | | désignation ou élection libre | l'association est un tiers (§12) |
| **Privé** : CA de l'organisme de gestion | membres de l'association (assemblée générale) | | selon les statuts | données de l'association (§12) |

---

## 10. Séances, ordres du jour, votes, délibérations, actes

### 10.1 Séance (avec le 19)
Le 46 demande au 19 une réunion d'origine `46` ; le 19 calcule le délai de convocation, envoie, recueille les réponses (y compris **procurations** pour les instances qui l'admettent [NV]) ; le 46 fournit les participants (composition à la date) et la **règle de quorum** ; à l'ouverture, le secrétaire de séance confirme les **présents** (19) ; le 46 calcule le quorum (membres en exercice, présents, représentés) et **bloque l'ouverture** des votes sans quorum (proposition de seconde convocation dans la fenêtre du type, 19 CAG-42).

### 10.2 Ordre du jour délibératif
Point : intitulé, nature (`information`, `debat`, `vote`), **objet typé** pour un vote (`sortie` du 42, `tarif` du 24, `bareme` du 45, `budget`/`dbm`/`compte_financier` du 48, `reglement_interieur`, `convention`, `contrat`, `structure` du 5, `politique_documentaire`, `projet_etablissement`, `libre`), rapporteur, pièces (7), projet de délibération. Les socles **proposent** un point (« inscrire ce voyage au prochain CA », 42 VOY-103 ; « barème des pertes », 45 CDI-181) ; le chef d'établissement **arrête** l'ordre du jour.

### 10.3 Vote en séance
- Chaque vote : présents et représentés **au moment du vote** (un membre parti en cours de séance est déduit ; quorum recontrôlé, R-IVD17) ; déport d'un membre intéressé (conflit d'intérêts, R-IVD18) ; mode (main levée ou **secret sur demande** [§5]) ; résultat (pour, contre, abstentions, refus de vote) ; adoption selon la majorité du type.
- Vote secret en séance : bulletins papier ou vote sur tablette **anonyme** en salle (module de vote de séance, lot DEM11) ; seul le total est saisi.
- Séance à distance ou mixte : seulement si le texte le permet pour le type [NV, §5] ; sinon, refus de l'option.

### 10.4 Délibération
Numéro (compteur par établissement et par exercice, E3), objet, texte, résultat, date, **transmission** (date, accusé), **caractère exécutoire** (date calculée : immédiat, ou transmission + délai pour les actes listés [§5]), **publicité** (registre, affichage, publication au portail des familles quand c'est un acte public), éventuel **retrait** ou **annulation** (décision du contrôle de légalité, déféré) ; versions figées (D-036 : instantané permis).

### 10.5 Procès-verbal
Trame générée (présents, excusés, représentés, quorum, ordre du jour, résultats des votes, délibérations), texte des débats rédigé par le secrétaire (aide de l'IA possible, §19) ; **projet** diffusé aux membres ; **approuvé** à la séance suivante (point automatique de l'ordre du jour) ; signé ; publié selon le type ; archivé.

### 10.6 Actes et contrôle de légalité (public)
Paquet de transmission préparé (acte, annexes, métadonnées) pour la plateforme de l'État ou la collectivité [NV : format et plateforme, §5, Q-IVD8] ; dépôt **manuel déclaré** (comme `09_INTEROP.md` R-X21) tant qu'aucun connecteur n'existe ; accusé enregistré ; date exécutoire calculée ; observation ou déféré enregistré.

---

## 11. Vote électronique

### 11.1 Position [R]
Le vote électronique est une **modalité** d'un scrutin, jamais un mode imposé. Il n'est proposé pour un type d'élection que si (1) un texte l'autorise pour ce type [§5] ; (2) l'établissement le décide (et, quand le texte le demande, l'instance compétente l'a voté) ; (3) la plateforme ScolIA a passé l'**expertise indépendante** exigée par la CNIL pour le niveau de risque retenu [§5] ; (4) une **modalité de repli** (urne ou correspondance) est offerte à tout électeur. Tant que (1) ou (3) manque, l'option n'apparaît pas (R-IVD27).

### 11.2 Architecture cible (conçue maintenant, codée en DEM10 et DEM11) [R]
- **Deux bases logiques séparées** : l'**émargement** (qui a voté, quand) et l'**urne** (bulletins chiffrés, sans identifiant d'électeur) ; aucune clé commune ; rôles PostgreSQL distincts ; l'urne ne connaît qu'un jeton de session de vote à usage unique, détruit à l'émargement.
- **Chiffrement du bulletin côté client** avec la clé publique de l'élection ; **clé privée partagée** entre les membres du bureau (partage à seuil, par ex. 3 sur 5) générée lors d'une cérémonie de scellement ; dépouillement possible **seulement** après la clôture et en présence du seuil.
- **Scellement** de l'urne et des listes à l'ouverture (empreintes publiées au bureau) ; journal en ajout seul chaîné ; contrôle d'intégrité à la clôture.
- **Authentification** de l'électeur : session ScolIA forte (passkey, D-009) **plus** un secret de vote transmis par un autre canal [NV : exigence du niveau CNIL] ; un électeur sans compte reçoit un identifiant de vote à usage unique (lien nominatif, 7 C).
- **Vérifiabilité** : accusé de vote (empreinte du bulletin chiffré) que l'électeur retrouve dans l'urne publiée ; aucune preuve de **contenu** (anti-coercition).
- **Interruption** : panne → prolongation décidée par le bureau, tracée ; aucune ouverture des résultats intermédiaires (R-IVD28).
- **Bibliothèques** : primitives éprouvées (pas de cryptographie maison) ; licence et audit à vérifier (CLAUDE.md, aucune dépendance adoptée sans analyse).

### 11.3 Vote électronique des élèves — **suspendu** (correction après §5.6)
Aucun texte n'autorise le vote électronique pour les délégués, le CVL, les élèves au CA ou le CVC (R421-28, R421-43 : « scrutin » sans modalité électronique). L'idée d'un vote en classe sur tablette est **retirée** du lot DEM10 ; elle ne revient que si une validation juridique (Q-IVD12, à Florian) l'admet pour des élections internes (éco-délégués, instances libres), et jamais pour les élections réglementées. Le gain de temps attendu est obtenu autrement : assistant du PP, bulletins générés, **saisie des résultats en un écran** (DEM1).

### 11.4 Ce que ScolIA ne fait pas
Pas de vote par simple formulaire (18) pour une élection ; pas de vote par e-mail ; pas de résultat partiel ; pas d'export de l'urne en clair ; pas de lien électeur–bulletin même pour l'administrateur plateforme (test de recette dédié).

---

## 12. Associations et privé sous contrat

- **Associations de parents** (FCPE, PEEP, APEL, listes locales), **maison des lycéens**, **association sportive**, **association de l'organisme de gestion** : ce sont des **personnes morales distinctes**, responsables de leurs propres traitements. ScolIA leur offre : un **espace de représentants** (membres désignés, publication au 17, boîte au 16), un **canal égal** en période électorale, la gestion de leurs **mandats dans les instances de l'établissement**. Il ne tient **pas** leur fichier d'adhérents, leurs cotisations ni leurs élections internes, sauf convention explicite avec l'établissement et l'association (Q-IVD10, à Florian : engage des tiers).
- **Conseil d'établissement** (privé catholique) : type d'instance du privé, composition selon le statut [NV] ; mêmes mécaniques (séances, PV), délibérations **consultatives** sauf statut contraire.
- **Instances de l'organisme de gestion** (OGEC : assemblée générale, conseil d'administration, bureau) : type `organisme`, rattaché à l'organisme (entité émettrice de D-027 F) ; composition saisie (administrateurs bénévoles, membres de droit selon les statuts [NV]) ; séances, PV, délibérations (budget, tarifs, embauches) consommées par le 24 et le 48 ; vote d'AG **papier assisté** seulement.
- **Délégués des parents au conseil de classe (privé)** : souvent des « parents correspondants » désignés par l'association [H] ; le 46 enregistre une **désignation** (fondement `designation`, désignant = association), lue par le 23.
- **Représentants du personnel de l'organisme** (CSE, Code du travail) : **hors périmètre** (élections professionnelles régies par le Code du travail, prestataires spécialisés) ; seuls les **mandats** résultants peuvent être saisis pour l'information (Q-IVD11).
- **Hors contrat** : instances libres seulement.

---

## 13. Vie lycéenne, vie collégienne, engagement des élèves

- **Projets du CVL ou du CVC** : dépôt par un élu, avis, vote du CVL, transmission au CA s'il faut une délibération, **réponse datée** de la direction (aucun projet sans réponse plus de 30 jours, R-IVD30), suivi.
- **Budget de la vie lycéenne** (fonds voté au CA [NV]) : lignes, engagements, solde — lecture du 48, décisions par délibération ; aucune régie au 46.
- **Formation des délégués** : sessions (19), présence, attestation ; tableau « délégués formés / à former ».
- **AG des délégués** : au moins deux fois par an dont une avant la fin de la 7ᵉ semaine (R421-42, [V] via `19_AGENDA_RDV.md` §6.3) : alerte au tableau des instances.
- **Absences pour mandat** : quand un élève siège pendant un cours (conseil de classe d'une autre classe ? non ; CVL, CA, AG), l'absence est **proposée justifiée** au 12 avec le motif « mandat » (R-IVD25).
- **Engagements** au livret (23) : mandats de l'année transformés en engagements proposés (délégué, CVL, éco-délégué…), validés par le CPE.
- **Délégués de l'internat, éco-délégués, ambassadeurs** : instances libres avec désignation ou élection libre.

---

## 14. Communication et transparence
- **Annuaire des élus** (par instance, par classe) : nom, qualité, liste d'origine ; **jamais** de téléphone ni d'adresse ; contact par la boîte des représentants (16).
- **Résultats** publiés (portail, 17) dans la forme prévue par le texte [§5] ; minimisation des petits bureaux (R-IVD13).
- **Actes publics** : registre consultable par les membres de la communauté éducative dans la forme prévue [§5] ; délibérations nominatives (une remise gracieuse, un recrutement) **jamais** publiées en clair (R-IVD22).
- **Période électorale** : neutralité de l'établissement (aucune publication institutionnelle favorisant une liste, R-IVD23) ; professions de foi relayées à l'identique (17 R-PB36).

---

## 15. Cas limites (catalogue)

### 15.1 Corps électoraux et candidatures
| N° | Cas | Traitement |
|---|---|---|
| CIV-01 | Parents séparés, deux autorités parentales | deux électeurs, deux matériels, deux adresses (4) |
| CIV-02 | Parent avec autorité parentale retirée | non électeur ; jamais affiché comme « retiré » (la liste ne montre que les électeurs) |
| CIV-03 | Tiers digne de confiance, tuteur, ASE | électeur selon le texte [§5] ; le 4 porte la qualité |
| CIV-04 | Parent de trois enfants dans l'établissement | **une** voix (R-IVD6) |
| CIV-05 | Parent dont les enfants sont dans deux établissements | électeur dans chacun (cloisonnement : aucun lien entre les deux) |
| CIV-06 | Beau-parent sans autorité parentale | non électeur [§5] |
| CIV-07 | Adresse protégée d'un parent | matériel envoyé par l'établissement sans que l'adresse apparaisse au 46 ; jamais sur une étiquette imprimée en masse visible (R-IVD8) |
| CIV-08 | Parent sans adresse valide | matériel remis par l'enfant ou au guichet ; signalé au 4 |
| CIV-09 | Élève arrivé après la date de figement | ses parents non électeurs cette année-là [NV] ; information claire |
| CIV-10 | Élève parti après le figement | ses parents restent sur la liste [NV] |
| CIV-11 | Doublon de personne (deux fiches) | une voix ; fusion demandée au 4 avant figement |
| CIV-12 | Personnel à temps partagé sur deux établissements | électeur selon la règle de quotité [NV] dans chacun ou un seul |
| CIV-13 | AESH, AED, contractuel en CDD | électorat calculé par le 6 avec l'explication |
| CIV-14 | Candidat inéligible (personnel qui est aussi parent) | règle du type [NV] : choix d'un collège |
| CIV-15 | Liste déposée une minute après l'échéance | refus automatique, dérogation impossible |
| CIV-16 | Liste avec un candidat qui retire son accord | liste corrigée avant l'échéance ; après, règle du type [NV] |
| CIV-17 | Aucune liste déposée | carence : règle du type (tirage au sort parmi volontaires) [NV] |
| CIV-18 | Une seule liste | scrutin maintenu |
| CIV-19 | Profession de foi diffamatoire ou contenant des données d'élèves | diffusée sans contrôle a priori (D111-7 et s.) ; signalement à l'autorité académique (IVD-137) ; si elle contient des données personnelles d'élèves, le DPO est saisi |

### 15.2 Scrutin et dépouillement
| N° | Cas | Traitement |
|---|---|---|
| CIV-20 | Électeur qui vote par voie électronique et par correspondance ou à l'urne | arrêté du 2-7-2024 art. 11 : l'électronique **précède** ; les plis de ceux qui ont déjà voté sont **écartés sans être ouverts** ; correspondance et urne : émargement unique |
| CIV-21 | Enveloppe de correspondance arrivée après la clôture | non dépouillée, consignée |
| CIV-22 | Enveloppe non signée, non identifiable | nulle, consignée |
| CIV-23 | Écart entre émargements et bulletins | consigné au PV, calcul sur les bulletins |
| CIV-24 | Double saisie des résultats divergente | blocage jusqu'à recomptage |
| CIV-25 | Égalité de restes ou de voix | règle du type affichée et appliquée [NV] |
| CIV-26 | Plus de sièges gagnés que de candidats | redistribution selon la règle [NV] |
| CIV-27 | Fermeture le jour du scrutin (neige) | report proposé ; correspondance reçue conservée scellée |
| CIV-28 | Panne réseau pendant un vote électronique | prolongation par le bureau ; repli papier |
| CIV-29 | Électeur sans compte ScolIA | identifiant de vote à usage unique ou papier |
| CIV-30 | Électeur qui demande s'il a « bien voté » | accusé vérifiable, sans contenu |
| CIV-31 | Administrateur qui tente de lire l'urne avant la clôture | impossible (clé partagée) ; tentative journalisée |
| CIV-32 | Bureau de moins de 10 votants (petite classe, petit collège) | résultats détaillés non publiés (R-IVD13) |
| CIV-33 | Recours d'un candidat | délai, enregistrement, décision ; mandats suspendus si annulation [NV] |
| CIV-34 | Élection de délégués : un seul candidat | élu s'il obtient la majorité requise [NV] |
| CIV-35 | Élève absent le jour de l'élection de délégués | pas de vote par correspondance [NV] |

### 15.3 Mandats et séances
| N° | Cas | Traitement |
|---|---|---|
| CIV-36 | Parent élu dont le dernier enfant part en cours d'année | mandat clos, suivant de liste [NV] |
| CIV-37 | Délégué qui démissionne | suppléant ou nouvelle élection |
| CIV-38 | Délégué sanctionné (exclusion) | effet sur le mandat selon le règlement [NV] ; aucun effet automatique |
| CIV-39 | Élu qui est aussi membre d'un conseil de discipline concernant son enfant | déport proposé (13) |
| CIV-40 | Quorum perdu en cours de séance | votes suivants bloqués, PV le consigne |
| CIV-41 | Procuration à un membre d'un autre collège | refus si le type l'interdit [NV] |
| CIV-42 | Plus de procurations qu'autorisé | refus |
| CIV-43 | Vote secret demandé par un membre | mode secret imposé (R-IVD19) |
| CIV-44 | Délibération sur un voyage déjà parti | signalée en anomalie au 42 |
| CIV-45 | Délibération annulée par le contrôle de légalité | consommateurs avisés ; objets bloqués |
| CIV-46 | Nouveau chef d'établissement en cours d'année | mandat de droit transféré, signatures des actes futures |
| CIV-47 | Membre extérieur sans compte | accès invité aux seuls documents de séance (lien nominatif, 7 C) |
| CIV-48 | PV non approuvé à la séance suivante | reste « projet », alerte |
| CIV-49 | Délibération qui nomme une personne (remise de dette, recrutement) | publicité sans le nom (R-IVD22) |

### 15.4 Données, sécurité
| N° | Cas | Traitement |
|---|---|---|
| CIV-50 | Association qui demande le fichier des parents | refus ; canal de diffusion égal (R-IVD12) |
| CIV-51 | Élève qui tente de voter pour une autre classe | refus serveur, journalisé |
| CIV-52 | Profession de foi contenant une consigne à l'IA | donnée non fiable, aucun effet |

---

## 16. Règles transverses (R-IVD1 à R-IVD34) [R]
| N° | Règle |
|---|---|
| R-IVD1 | Les règles réglementaires d'un type d'instance (composition, quorum, délais, modes de scrutin) sont fixées par le référentiel de l'opérateur, non modifiables par l'établissement. |
| R-IVD2 | Toute composition est lue **à une date** ; aucune liste de membres n'est stockée en double chez un consommateur. |
| R-IVD3 | Un mandat a toujours un fondement, une qualité et des dates ; il n'est jamais supprimé, seulement clos avec un motif. |
| R-IVD4 | Le corps électoral est calculé par les socles propriétaires (2, 4, 6) ; le 46 ne corrige jamais une donnée de personne. |
| R-IVD5 | La liste électorale est figée par un instantané horodaté avec empreinte ; elle ne change que par décision tracée après réclamation. |
| R-IVD6 | Un électeur vote une fois par scrutin, quel que soit le nombre de ses liens. |
| R-IVD7 | Toute modification après figement porte la décision, son auteur et son motif. |
| R-IVD8 | Aucune adresse, aucun téléphone, aucun nom d'enfant dans une liste affichée ou exportée vers un tiers ; adresse protégée jamais visible du 46. |
| R-IVD9 | Les délais d'un scrutin sont calculés ; une étape hors délai est bloquée (dérogation impossible si le texte n'en prévoit pas). |
| R-IVD10 | Les résultats papier sont saisis deux fois par deux personnes du bureau ; un écart bloque le calcul. |
| R-IVD11 | Un tirage au sort est fait en présence du bureau, avec une graine publiée et consignée au PV. |
| R-IVD12 | (corrigée, D111-7 à D111-15) La liste des parents n'est communicable à une association **que pour les parents qui ont donné leur accord exprès** (consentement du 18, horodaté, révocable, par association) ; tout autre parent n'apparaît jamais dans un export ; diffusion égale par le 17 et la boîte aux lettres des associations. |
| R-IVD13 | Aucun résultat détaillé n'est publié pour un bureau de moins de 10 votants. |
| R-IVD14 | Le secret du vote : aucun lien technique entre l'émargement et le contenu d'un bulletin, pour aucun rôle. |
| R-IVD15 | Aucun résultat partiel n'est consultable avant la clôture. |
| R-IVD16 | Les calculs de sièges sont déterministes, testés sur jeux d'or, rejouables et expliqués pas à pas. |
| R-IVD17 | Le quorum est contrôlé à l'ouverture et à chaque vote. |
| R-IVD18 | Un membre en conflit d'intérêts est déporté pour le vote concerné ; le déport est consigné. |
| R-IVD19 | Le vote secret est de droit quand le type le prévoit sur demande d'un membre. |
| R-IVD20 | Aucun socle n'applique une décision soumise à délibération avant son caractère exécutoire. |
| R-IVD21 | Une délibération n'est jamais modifiée ; elle est retirée ou remplacée par une nouvelle. |
| R-IVD22 | Une délibération qui nomme une personne n'est publiée qu'anonymisée. |
| R-IVD23 | En période électorale, aucune publication institutionnelle ne favorise une candidature. |
| R-IVD24 | Les mandats d'élèves mineurs n'exposent que nom, prénom, classe et qualité. |
| R-IVD25 | Une absence d'élève pour un mandat est proposée justifiée au 12, jamais imposée. |
| R-IVD26 | Le vote électronique n'est jamais la seule modalité. |
| R-IVD27 | Le vote électronique n'est proposé pour un type que si un texte l'autorise et après expertise indépendante. |
| R-IVD28 | En cas d'incident d'un scrutin électronique, seul le bureau décide (prolongation, repli) ; aucune action automatique sur l'urne. |
| R-IVD29 | L'IA ne compte aucune voix, ne lit aucun bulletin ni émargement, ne désigne personne. |
| R-IVD30 | Un projet du CVL ou du CVC reçoit une réponse datée sous 30 jours. |
| R-IVD31 | Les associations sont des tiers : ScolIA ne traite pas leurs fichiers d'adhérents sans convention. |
| R-IVD32 | Les dates sont en Europe/Paris ; une échéance « franche » est calculée selon le texte [NV]. |
| R-IVD33 | Aucune donnée d'un établissement n'est visible d'un autre (y compris pour un parent électeur dans deux établissements). |
| R-IVD34 | Le vocabulaire du 46 (représentant, mandat) ne réutilise pas celui des délégations de droits du socle 1. |

---

## 17. Droits et minimisation

### 17.1 Catalogue d'actions proposé (D-017) [R]
| Droit | Portée | Modèles |
|---|---|---|
| `instances.lire` | établissement : compositions, calendriers, actes publics | tous les personnels ; familles et élèves : annuaire des élus et actes publics |
| `instances.gerer` | établissement | Direction ; Secrétariat de direction |
| `instances.seance` | instance | président, secrétaire de séance désigné |
| `instances.deliberer` | instance | membres avec voix délibérative (vote en séance) |
| `elections.organiser` | scrutin | Direction ; CPE (élections d'élèves) ; PP (élection de sa classe, pendant la campagne) |
| `elections.bureau` | scrutin | membres désignés du bureau |
| `elections.liste_electorale.lire` | scrutin | bureau ; électeurs (consultation minimisée, §9.1) |
| `elections.candidater` | soi | électeurs éligibles |
| `elections.liste.deposer` | sa liste | mandataire de la liste |
| `elections.voter` | soi | électeurs |
| `actes.transmettre` | établissement | Direction ; Secrétariat de direction |
| `mandats.saisir` | établissement | Direction ; CPE (mandats d'élèves) |
| `vie_lyceenne.projets` | instance | élus du CVL/CVC (déposer), CPE et direction (répondre) |
| `elections.urne.sceller`, `elections.urne.depouiller` | scrutin électronique | membres du bureau, chacun avec sa part de clé |

### 17.2 Ce que voit chacun [R]
| Information | Direction | Bureau | CPE | PP | Membres de l'instance | Personnels | Parents | Élèves |
|---|---|---|---|---|---|---|---|---|
| Compositions, élus | oui | oui | oui | oui | oui | oui | élus (annuaire) | élus (annuaire) |
| Liste électorale | oui | oui | élèves | sa classe (délégués) | non | consultation de son inscription | consultation de son inscription, liste minimisée [Q-IVD5] | sa classe (délégués) |
| Émargement | non pendant le scrutin (sauf bureau) | oui | — | sa classe | non | non | non | non |
| Bulletins | **personne** (secret) | totaux | — | totaux | — | — | — | — |
| Résultats | oui | oui | oui | oui | oui | oui | oui (forme du texte) | oui |
| Documents de séance | oui | — | si membre | si membre | oui | non | non (sauf actes publics) | non |
| PV approuvé, actes publics | oui | — | oui | oui | oui | oui | forme du texte | forme du texte |

### 17.3 Conservation [R, soumise au DPO et aux règles d'archives §5]
| Donnée | Durée | Sort |
|---|---|---|
| Liste électorale figée, émargements, bulletins papier | jusqu'à l'expiration des délais de recours et des recours | destruction (bulletins) ; liste et émargement [NV] |
| Urne électronique, clés | idem | destruction prouvée des parts de clé |
| PV d'élection, résultats | archives [NV] | versement (27) |
| PV de séance, délibérations, actes | conservation longue (archives publiques) [NV] | versement |
| Candidatures non élues | fin du délai de recours | supprimées (sauf PV) |
| Mandats | durée du mandat + archives des PV | — |
| Journaux d'accès | 12 mois | supprimés |

### 17.4 Pièges à tester
1. Un membre du bureau tente de relier un émargement à un bulletin électronique : impossible (pas de clé commune).
2. Un parent dont l'autorité parentale est retirée tente de voter : refus, message neutre.
3. Un parent tente de voter deux fois avec deux liens d'enfants : refus.
4. Une liste candidate appelle l'API de la liste électorale : refus (aucune route).
5. Un élève vote pour une autre classe : refus.
6. Un professeur lit les documents d'une séance du CA dont il n'est pas membre : refus.
7. L'IA reçoit-elle des bulletins ou des émargements dans un contexte ? test : non.
8. Le 42 lit une délibération non exécutoire : « aucune » renvoyé.
9. Un établissement voit-il un mandat d'un autre établissement d'un même parent ? non (RLS).
10. Un administrateur plateforme lit-il l'urne avant clôture ? non.

---

## 18. Événements et notifications

### 18.1 Événements émis (outbox) [R]
| Événement | Consommateurs |
|---|---|
| `instances.composition_modifiee` | 13, 19, 23 |
| `instances.mandat_ouvert` / `_clos` | 12, 16, 17, 23, 29 |
| `elections.scrutin_ouvert`, `_liste_figee`, `_candidature_deposee`, `_cloture`, `_resultats_proclames`, `_recours`, `_annule` | 17, 19, 29, 9 |
| `instances.seance_demandee` | 19 |
| `instances.point_inscrit` | 29 (porteur du point) |
| `instances.deliberation_adoptee` / `_rejetee` / `_transmise` / `_executoire` / `_retiree` / `_annulee` | 5, 24, 40, 42, 45, 48 |
| `instances.pv_approuve` | 7, 27 |
| `vie_lyceenne.projet_depose` / `_repondu` | 29 |

Consommés : `eleve.sortie`, `eleve.majeur`, `eleve.decede`, `eleve.fusionne`, changements de classe (2, 5) ; `liens.*` (4) ; `personnel.engagement_clos`, fonctions (6) ; `agenda.presence_saisie`, `agenda.compte_rendu_depose`, `agenda.evenement_annule` (19) ; `instances.point_propose` (42, 45, 24, 48).

### 18.2 Natures déclarées au socle 8
| Nature | Urgence | Destinataires | Plancher |
|---|---|---|---|
| `elections.information` (calendrier, appel à candidatures) | U3 | électeurs (règle calculée au moment de l'envoi) | non désactivable pour les élections officielles |
| `elections.materiel` | U2 | électeurs | non désactivable |
| `elections.resultats` | U3 | électeurs | désactivable |
| `instances.mandat` (élu, fin de mandat) | U3 | élu | non désactivable |
| `instances.documents_seance` | U2 | membres | non désactivable |
| `vie_lyceenne.reponse_projet` | U3 | porteurs | désactivable |
Plage de non-notification du cadre de référence (soir et week-end [V-indirect], `45_CDI.md` §5.4) respectée, sauf convocation d'urgence prévue par le texte.

---

## 19. Intelligence artificielle (`IA_COEUR.md` §5, §6.2)
### 19.1 Ce que l'IA fait [R]
- **Projet de PV** à partir des notes du secrétaire et des faits de séance (présents, votes) : N0, relu et adopté par le secrétaire.
- **Calendrier électoral** expliqué (« pourquoi le dépôt des listes est le 26 septembre ») : N0, à partir du calcul déterministe.
- **Vérification de composition** en mots simples (« il manque un suppléant parent ») : N0.
- **Aide à la rédaction** d'une profession de foi, **dans l'espace d'une liste et pour elle seule** (aucune liste ne voit l'aide d'une autre ; aucun contenu d'une autre liste dans le contexte) : N0.
- **Résumé** d'un projet de délibération ou d'un PV pour les familles (langage simple) : N0.
- **Synthèse** des projets du CVL et de leurs réponses : N0.
### 19.2 Ce que l'IA ne fait pas
Elle ne compte, ne lit, ne déduit aucun vote ; ne lit aucune liste électorale nominative, aucun émargement ; ne propose pas de candidat ; ne qualifie pas une profession de foi de « meilleure » ; ne rédige aucune délibération à adopter sans qu'un humain l'écrive ; ne publie rien en période électorale.
### 19.3 Capacités déclarées
| Capacité | Niveau | Contexte | Droits |
|---|---|---|---|
| `instances.projet_pv` | N0 | notes du secrétaire, faits de séance (nombres, intitulés) | secrétaire de séance |
| `elections.expliquer_calendrier` | N0 | dates calculées, règles du type | organisateurs |
| `instances.verifier_composition` | N0 | sièges, qualités (sans données personnelles au-delà des noms) | direction |
| `elections.aide_profession_foi` | N0 | texte de la liste | mandataire de la liste |
| `instances.resumer_acte` | N0 | texte de l'acte public | tous ceux qui voient l'acte |
| `vie_lyceenne.synthese_projets` | N0 | projets et réponses | CPE, direction, élus |

---

## 20. Actions métier (IVD-100 à IVD-281)
Acteurs : **DIR** chef d'établissement ou adjoint, **SDIR** secrétariat de direction, **CPE**, **PP**, **BUR** bureau, **PRES** président de séance, **SEC-S** secrétaire de séance, **MBR** membre, **ELEC** électeur, **CAND** candidat ou mandataire, **ELU**, **ELV**, **RESP**, **SYS**, **OPS** opérateur de plateforme.

### 20.1 Référentiel et instances
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| IVD-100 | Tenir le référentiel des types d'instances par régime | OPS | versionné, sources citées | N2 | — | DEM0 |
| IVD-101 | Créer une instance d'un type pour l'année | DIR · `instances.gerer` | régime de l'établissement | N1 | `instances.creee` | DEM0 |
| IVD-102 | Créer une instance libre (commission menus, comité MDL…) | DIR | sans règle réglementaire | N1 | idem | DEM0 |
| IVD-103 | Voir la composition attendue et réelle à une date | DIR, SDIR | §8.2 | N0 | — | DEM0 |
| IVD-104 | Saisir un mandat (avec PV papier) | DIR, CPE · `mandats.saisir` | pièce obligatoire pour un élu | N1 | `instances.mandat_ouvert` | DEM0 |
| IVD-105 | Clore un mandat (motif) | DIR, CPE | — | N1 | `instances.mandat_clos` | DEM0 |
| IVD-106 | Remplacer un titulaire par son suppléant ou le suivant de liste | DIR | règle du type | N2 | idem | DEM0 |
| IVD-107 | Déclarer un siège vacant | DIR | — | N1 | — | DEM0 |
| IVD-108 | Ajouter un représentant extérieur ou une personnalité qualifiée | SDIR | personne tiers (4) | N1 | — | DEM0 |
| IVD-109 | Transférer les mandats de droit à un nouveau titulaire de fonction | SYS | fonction du 6 | — | idem | DEM0 |
| IVD-110 | Convertir les saisies locales du 13 et du 23 en mandats | SYS | reprise (§27) | N2 | — | DEM0 |
| IVD-111 | Voir mes mandats | ELU | — | N0 | — | DEM0 |
| IVD-112 | Annuaire des élus (famille, élève, personnel) | tous | §14, sans coordonnées | N0 | — | DEM7 |
| IVD-113 | Exporter la composition (PDF) | DIR | sans coordonnées | N1 | — | DEM0 |
| IVD-114 | Vérifier les incompatibilités et cumuls | SYS | [NV] | — | — | DEM4 |
| IVD-115 | Signaler une composition incomplète au tableau des instances (19) | SYS | — | — | — | DEM0 |

### 20.2 Élections : préparation
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| IVD-116 | Publier la couche nationale du calendrier (dates des élections de parents) | OPS | note de service de l'année | N2 | — | DEM2 |
| IVD-117 | Ouvrir un scrutin (type, sièges, modalités) | DIR, CPE · `elections.organiser` | §9.1 | N2 | `elections.scrutin_ouvert` | DEM1 |
| IVD-118 | Voir et ajuster le calendrier calculé | DIR, CPE | délais réglementaires non réductibles | N1 | — | DEM1 |
| IVD-119 | Constituer le bureau | DIR | — | N1 | — | DEM1 |
| IVD-120 | Calculer le corps électoral à la date | SYS | 2, 4, 6 | — | — | DEM2 |
| IVD-121 | Voir les anomalies du corps électoral | DIR, SDIR | correction chez le propriétaire | N0 | — | DEM2 |
| IVD-122 | Expliquer l'électorat d'une personne | DIR, SDIR, la personne | « électeur car… » | N0 | — | DEM2 |
| IVD-123 | Figer la liste électorale | DIR | instantané, empreinte | N2 | `elections.liste_figee` | DEM2 |
| IVD-124 | Afficher et rendre consultable la liste (minimisée) | SYS | R-IVD8 | — | — | DEM2 |
| IVD-125 | Vérifier son inscription | ELEC | — | N0 | — | DEM2 |
| IVD-126 | Déposer une réclamation sur la liste | ELEC | délai | N1 | — | DEM2 |
| IVD-127 | Décider d'une réclamation | DIR | tracée (R-IVD7) | N2 | `elections.liste_modifiee` | DEM2 |
| IVD-128 | Lancer l'appel à candidatures | DIR, CPE | 8, 17 | N3 | — | DEM1 |
| IVD-129 | Déclarer une liste (mandataire) | CAND · `elections.liste.deposer` | — | N1 | — | DEM3 |
| IVD-130 | Ajouter des candidats et leur ordre | CAND | éligibilité contrôlée | N1 | — | DEM3 |
| IVD-131 | Recueillir l'accord de chaque candidat | SYS (18) | signature | N2 | — | DEM3 |
| IVD-132 | Déposer la liste | CAND | échéance stricte | N2 | `elections.candidature_deposee` | DEM3 |
| IVD-133 | Se porter candidat (uninominal, binôme) | ELV, ELEC | — | N1 | idem | DEM1 |
| IVD-134 | Retirer une candidature | CAND | avant échéance | N1 | — | DEM1 |
| IVD-135 | Contrôler et arrêter les candidatures | DIR, CPE | — | N2 | `elections.candidatures_arretees` | DEM1 |
| IVD-136 | Déposer une profession de foi | CAND | format, échéance | N1 | — | DEM3 |
| IVD-137 | (corrigée, D111-7 à D111-15 : **pas de contrôle a priori**) Signaler une profession de foi contestable (diffamation, atteinte à la vie privée, propagande) à l'autorité académique, qui arbitre sous 7 jours ; la diffusion n'est pas bloquée par l'établissement | DIR | — | N2 | `elections.signalement` | DEM3 |
| IVD-138 | Diffuser les professions de foi à l'identique | SYS (17) | même date | N3 | — | DEM3 |
| IVD-139 | Constater la carence et appliquer la règle | DIR | [NV] | N2 | — | DEM3 |
| IVD-140 | Générer le matériel (bulletins, enveloppes, notice) | SDIR | trames D-023 | N1 | — | DEM3 |
| IVD-141 | Envoyer le matériel aux électeurs | SDIR | 8 ; adresses jamais visibles (R-IVD8) | N3 | — | DEM3 |
| IVD-142 | Générer les listes d'émargement | SDIR | — | N1 | — | DEM3 |
| IVD-143 | Rappeler aux électeurs la date et la modalité | SYS | 8 | — | — | DEM3 |

### 20.3 Scrutin et dépouillement
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| IVD-144 | Enregistrer la réception d'un vote par correspondance (émargement) | BUR | horodaté | N1 | — | DEM3 |
| IVD-145 | Ouvrir le bureau de vote | BUR | — | N1 | — | DEM3 |
| IVD-146 | Émarger un électeur (papier assisté sur tablette) | BUR | une fois | N1 | — | DEM3 |
| IVD-147 | Clore le scrutin | BUR | heure du calendrier | N2 | `elections.cloture` | DEM1 |
| IVD-148 | Saisir les résultats (première saisie) | BUR | — | N1 | — | DEM1 |
| IVD-149 | Saisir les résultats (seconde saisie) | BUR (autre personne) | écart bloquant | N1 | — | DEM1 |
| IVD-150 | Calculer les sièges (pas à pas) | SYS | §9.2 | — | — | DEM1 |
| IVD-151 | Organiser un second tour | CPE, PP | uninominal | N1 | — | DEM1 |
| IVD-152 | Trancher une égalité selon la règle | SYS, BUR | affichée | N1 | — | DEM1 |
| IVD-153 | Tirer au sort en présence du bureau | BUR | graine publiée (R-IVD11) | N2 | — | DEM1 |
| IVD-154 | Générer le PV d'élection | SYS | trame | N1 | — | DEM1 |
| IVD-155 | Signer le PV (président, assesseurs) | BUR (7) | — | N2 | — | DEM1 |
| IVD-156 | Proclamer les résultats | DIR, CPE | R-IVD13 | N3 | `elections.resultats_proclames` | DEM1 |
| IVD-157 | Créer les mandats depuis les résultats | SYS | titulaires, suppléants | — | `instances.mandat_ouvert` | DEM1 |
| IVD-158 | Exporter les résultats pour l'application nationale | SDIR (9) | pré-contrôle | N2 | `echanges.exporte` | DEM3 |
| IVD-159 | Déclarer le dépôt des résultats | SDIR | dépôt déclaré (9 R-X21) | N1 | — | DEM3 |
| IVD-160 | Enregistrer un recours | DIR | délai | N1 | `elections.recours` | DEM3 |
| IVD-161 | Enregistrer la décision sur recours ; annuler l'élection | DIR | mandats annulés | N3 | `elections.annule` | DEM3 |
| IVD-162 | Reporter un scrutin (fermeture) | DIR | correspondance conservée | N2 | — | DEM3 |
| IVD-163 | Détruire les bulletins et pièces à l'échéance | SYS → DIR | R-IVD conservation | N2 | — | DEM13 |

### 20.4 Élections particulières
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| IVD-164 | Campagne « délégués de classe » pour toutes les classes | CPE | un scrutin par classe | N2 | — | DEM1 |
| IVD-165 | Suivre l'avancement par classe | CPE | — | N0 | — | DEM1 |
| IVD-166 | Mener l'élection en classe (assistant pas à pas) | PP | — | N1 | — | DEM1 |
| IVD-167 | Désigner les délégués des parents au conseil de classe | DIR | public : parmi les listes selon le texte [NV] ; privé : désignation par l'association | N2 | — | DEM4 |
| IVD-168 | Élire les membres du CVL (binômes) | CPE | [NV] | N2 | — | DEM4 |
| IVD-169 | Élire le vice-président du CVL | CVL | [NV] | N2 | — | DEM4 |
| IVD-170 | Organiser l'élection du CVC selon les modalités du CA | CPE | délibération du CA | N2 | — | DEM4 |
| IVD-171 | Élire les représentants des élèves au CA | AG des délégués / CVL | [NV] | N2 | — | DEM4 |
| IVD-172 | Élire la commission permanente | CA | par collège [NV] | N2 | — | DEM4 |
| IVD-173 | Élire ou désigner les membres du conseil de discipline et de la commission éducative | CA | [NV] ; lu par le 13 | N2 | — | DEM4 |
| IVD-174 | Désigner les éco-délégués, délégués d'internat, ambassadeurs | CPE | désignation libre | N1 | — | DEM4 |
| IVD-175 | Organiser une élection partielle | DIR | siège vacant | N2 | — | DEM4 |
| IVD-176 | Élection de personnels au CA, deux collèges | DIR | 6 ; non-titulaires électeurs à 150 h par an (R421-26) ; **urne et correspondance seulement** | N2 | — | DEM3 |
| IVD-177 | Élection libre (instance libre, mode choisi) | DIR | moteur commun | N2 | — | DEM4 |

### 20.5 Séances et délibérations
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| IVD-178 | Demander une séance au 19 | DIR · `instances.seance` | type, délais | N1 | `instances.seance_demandee` | DEM5 |
| IVD-179 | Proposer un point à l'ordre du jour (autre socle) | porteur (42, 45, 24, 48) | objet typé | N1 | `instances.point_propose` | DEM5 |
| IVD-180 | Arrêter l'ordre du jour | DIR | points, natures, pièces | N2 | `instances.point_inscrit` | DEM5 |
| IVD-181 | Joindre les documents (7), envoyés avec la convocation (19) | SDIR | D111-13 : mêmes documents aux parents [V] | N1 | — | DEM5 |
| IVD-182 | Ouvrir la séance, confirmer les présents | PRES, SEC-S | 19 | N1 | — | DEM5 |
| IVD-183 | Calculer et afficher le quorum | SYS | R-IVD17 | — | — | DEM5 |
| IVD-184 | Constater l'absence de quorum, préparer la seconde convocation | PRES | 19 CAG-42 | N2 | — | DEM5 |
| IVD-185 | Enregistrer une procuration | SEC-S | règle du type [NV] | N1 | — | DEM5 |
| IVD-186 | Enregistrer l'arrivée ou le départ d'un membre en séance | SEC-S | quorum recalculé | N1 | — | DEM5 |
| IVD-187 | Déporter un membre pour un point | PRES | R-IVD18 | N1 | — | DEM5 |
| IVD-188 | Mettre un point au vote (main levée) | PRES | — | N1 | — | DEM5 |
| IVD-189 | Mettre un point au vote secret (sur demande) | PRES | R-IVD19 | N1 | — | DEM5 |
| IVD-190 | Saisir le résultat d'un vote | SEC-S | pour, contre, abstentions | N1 | `instances.deliberation_adoptee` | DEM5 |
| IVD-191 | Amender un projet de délibération en séance | PRES | texte final voté | N1 | — | DEM5 |
| IVD-192 | Clore la séance | PRES | — | N1 | — | DEM5 |
| IVD-193 | Rédiger le PV (trame préremplie) | SEC-S | — | N1 | — | DEM5 |
| IVD-194 | Demander un projet de PV à l'IA | SEC-S | §19 | N0 | — | DEM12 |
| IVD-195 | Diffuser le projet de PV aux membres | SEC-S | — | N3 | — | DEM5 |
| IVD-196 | Approuver le PV à la séance suivante | instance | point automatique | N2 | `instances.pv_approuve` | DEM5 |
| IVD-197 | Signer le PV | PRES, SEC-S (7) | — | N2 | — | DEM5 |
| IVD-198 | Numéroter les délibérations par exercice | SYS | compteur | — | — | DEM5 |
| IVD-199 | Voir le registre des délibérations | DIR, membres | recherche par objet | N0 | — | DEM6 |
| IVD-200 | Retirer une délibération (nouvelle délibération) | instance | R-IVD21 | N2 | `instances.deliberation_retiree` | DEM6 |
| IVD-201 | Vote de séance sur tablette (anonyme en salle) | MBR | DEM11 | N1 | — | DEM11 |

### 20.6 Actes, transmission, publicité
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| IVD-202 | Préparer le paquet de transmission d'un acte | SDIR · `actes.transmettre` | format [NV] | N1 | — | DEM6 |
| IVD-203 | Déclarer la transmission (date, accusé) | SDIR | — | N2 | `instances.deliberation_transmise` | DEM6 |
| IVD-204 | Calculer la date exécutoire | SYS | type d'acte [§5] | — | `instances.deliberation_executoire` | DEM6 |
| IVD-205 | Enregistrer une observation ou un déféré | DIR | consommateurs avisés | N2 | `instances.deliberation_annulee` | DEM6 |
| IVD-206 | Publier un acte (registre, affichage, portail) | DIR | R-IVD22 | N3 | — | DEM6 |
| IVD-207 | Enregistrer une décision du chef d'établissement soumise à transmission | DIR | même circuit | N2 | — | DEM6 |
| IVD-208 | Lire la délibération exécutoire d'un objet (port) | 24, 40, 42, 45, 48, 5 | R-IVD20 | — | — | DEM6 |
| IVD-209 | Alerte « objet appliqué sans délibération exécutoire » | SYS | anomalie chez le consommateur | — | — | DEM6 |
| IVD-210 | Exporter le registre des actes d'un exercice | DIR | archives | N1 | — | DEM6 |

### 20.7 Communication des élus et des associations
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| IVD-211 | Ouvrir la boîte des représentants des parents d'une classe | SYS (16) | membres = élus ou désignés | — | — | DEM7 |
| IVD-212 | Publier au nom des représentants | ELU (17) | mandat en cours | N2 | — | DEM7 |
| IVD-213 | Déclarer une association (tiers) et ses représentants | DIR | personne morale (4) | N1 | — | DEM7 |
| IVD-214 | Donner un canal égal aux listes en période électorale | SYS | 17 | — | — | DEM7 |
| IVD-215 | Compte rendu d'un élu aux familles de la classe (après conseil) | ELU | 16/17 ; relu par personne ? non : responsabilité de l'élu ; aucun nom d'élève [R] | N2 | — | DEM7 |
| IVD-216 | Accès invité d'un membre extérieur aux documents de séance | SDIR | lien nominatif (7 C) | N2 | — | DEM7 |
| IVD-217 | Publier les résultats d'une élection | DIR (17) | R-IVD13 | N3 | — | DEM7 |
| IVD-218 | Calendrier électoral sur l'agenda des familles et des personnels | SYS (19, 29) | — | — | — | DEM7 |

### 20.8 Vie lycéenne et engagement
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| IVD-219 | Déposer un projet du CVL ou du CVC | ELU élève · `vie_lyceenne.projets` | — | N1 | `vie_lyceenne.projet_depose` | DEM8 |
| IVD-220 | Voter un projet en CVL | CVL | vote de séance | N1 | — | DEM8 |
| IVD-221 | Répondre à un projet (date, motif) | DIR, CPE | R-IVD30 | N2 | `vie_lyceenne.projet_repondu` | DEM8 |
| IVD-222 | Inscrire un projet au CA | DIR | point typé | N1 | — | DEM8 |
| IVD-223 | Suivre le budget de la vie lycéenne | CPE, élus, DIR | lecture du 48 | N0 | — | DEM8 |
| IVD-224 | Planifier une formation des délégués (19) | CPE | — | N1 | — | DEM8 |
| IVD-225 | Suivre les délégués formés | CPE | — | N0 | — | DEM8 |
| IVD-226 | Proposer la justification d'une absence pour mandat | SYS (12) | R-IVD25 | — | — | DEM8 |
| IVD-227 | Proposer les engagements au livret (23) | SYS → CPE | depuis les mandats | N1 | `lsl.engagement_saisi` | DEM8 |
| IVD-228 | Planifier l'AG des délégués avant la fin de la 7ᵉ semaine | CPE (19) | alerte | N1 | — | DEM8 |
| IVD-229 | Espace d'échange encadré délégués–classe | ELU élève (16) | fil encadré par un adulte | N2 | — | DEM8 |
| IVD-230 | Délégués lus par le conseil de classe (23) | SYS | port | — | — | DEM1 |

### 20.9 Privé sous contrat et organisme de gestion
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| IVD-231 | Créer le conseil d'établissement (privé) | DIR | composition selon statut [NV] | N1 | — | DEM9 |
| IVD-232 | Créer les instances de l'organisme (AG, CA, bureau) | DIR, président de l'organisme | rattachées à l'organisme | N1 | — | DEM9 |
| IVD-233 | Saisir les administrateurs et membres de droit de l'organisme | président, SDIR | — | N1 | — | DEM9 |
| IVD-234 | Tenir les séances et PV de l'organisme | président, secrétaire | — | N1 | — | DEM9 |
| IVD-235 | Délibérations de l'organisme lues par le 24 et le 48 (tarifs, budget) | SYS | port | — | — | DEM9 |
| IVD-236 | Vote d'AG papier assisté (émargement des membres, résultats) | bureau d'AG | membres déclarés | N2 | — | DEM9 |
| IVD-237 | Enregistrer une désignation par l'association de parents | DIR | fondement `designation` | N1 | — | DEM9 |
| IVD-238 | Saisir les mandats du CSE pour information | DIR | Q-IVD11 | N1 | — | DEM9 |
| IVD-239 | Tenir les instances d'un hors contrat (libres) | DIR | — | N1 | — | DEM9 |

### 20.10 Vote électronique (conditionnel, §11)
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| IVD-240 | Activer le vote électronique pour un scrutin | DIR | R-IVD27 ; décision d'instance si requise | N3 | — | DEM10 |
| IVD-241 | Cérémonie de scellement (génération des clés, parts) | BUR | seuil | N3 | — | DEM10 |
| IVD-242 | Publier les empreintes de scellement | BUR | — | N1 | — | DEM10 |
| IVD-243 | Envoyer les secrets de vote | SYS | autre canal [NV] | N3 | — | DEM11 |
| IVD-244 | Voter en ligne | ELEC | une fois ; repli offert | N2 | — | DEM10 |
| IVD-245 | Retrouver son accusé de vote | ELEC | sans contenu | N0 | — | DEM10 |
| IVD-246 | Suivre la participation (sans résultats) | BUR | — | N0 | — | DEM10 |
| IVD-247 | Prolonger ou suspendre le scrutin (incident) | BUR | R-IVD28 | N3 | — | DEM10 |
| IVD-248 | Clore et contrôler l'intégrité | BUR | — | N3 | — | DEM10 |
| IVD-249 | Dépouiller (seuil de parts) | BUR | — | N3 | — | DEM10 |
| IVD-250 | Publier l'urne chiffrée et le résultat vérifiable | BUR | — | N2 | — | DEM10 |
| IVD-251 | Détruire les parts de clé et l'urne à l'échéance | BUR → SYS | preuve | N3 | — | DEM13 |
| IVD-252 | ~~Vote d'élèves en présentiel~~ **retirée** (§11.3, aucun texte) | — | Q-IVD12 | — | — | — |
| IVD-253 | Faire réaliser l'expertise indépendante | OPS | **à Florian** (coût) | — | — | DEM11 |

### 20.11 Pilotage, conservation, système
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| IVD-254 | Tableau des instances (19 AGD-142) enrichi : compositions, mandats vacants, PV non approuvés, actes non exécutoires | DIR | — | N0 | — | DEM5 |
| IVD-255 | Compteurs « au moins 3 fois par an » | SYS | 19 AGD-340 | — | — | DEM5 |
| IVD-256 | Taux de participation (agrégé) au pilotage | SYS (47) | — | — | — | DEM13 |
| IVD-257 | Verser les PV et actes aux archives (27) | DIR | — | N2 | — | DEM13 |
| IVD-258 | Purger listes et pièces électorales à l'échéance | SYS | §17.3 | — | — | DEM13 |
| IVD-259 | Journal des accès aux listes électorales | DIR, DPO | 12 mois | N0 | — | DEM2 |
| IVD-260 | Neutraliser les envois et l'urne à la restauration d'une sauvegarde | SYS | E9 ; urne restaurée **scellée**, jamais rouverte | — | — | DEM0 |
| IVD-261 | Exporter mes données (élu, électeur) | la personne | 27 | N2 | — | DEM13 |

### 20.12 Ajouts de cohérence
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| IVD-262 | Dupliquer le calendrier électoral de l'année précédente | DIR | décalé au calendrier national | N1 | — | DEM2 |
| IVD-263 | Notice d'explication du vote aux familles (langues) | SDIR | trames, traductions [R] | N1 | — | DEM3 |
| IVD-264 | Modèle d'affichage de la liste électorale | SDIR | minimisé | N1 | — | DEM2 |
| IVD-265 | Liste des sièges à pourvoir calculée (effectif, SEGPA…) | SYS | référentiel [§5] | — | — | DEM3 |
| IVD-266 | Contrôle « personnel aussi parent » | SYS | CIV-14 | — | — | DEM3 |
| IVD-267 | Changement d'ordre du jour en séance (point ajouté) | PRES | règle du type [NV] | N2 | — | DEM5 |
| IVD-268 | Question diverse | PRES | sans vote | N1 | — | DEM5 |
| IVD-269 | Rappel aux membres des documents à lire | SYS | 8 | — | — | DEM5 |
| IVD-270 | Mobile : voter, voir mes mandats, documents de séance | ELEC, ELU | D-003 | N1 | — | DEM10 |
| IVD-271 | Dépôt de candidature sur mobile (élèves) | ELV | — | N1 | — | DEM1 |
| IVD-272 | Assistant « première élection » pour un nouvel établissement | DIR | modèles prêts | N0 | — | DEM3 |
| IVD-273 | Comparer la composition à la règle (contrôle annuel) | SYS | — | — | — | DEM4 |
| IVD-274 | Désigner le secrétaire de séance | PRES | rotation [NV] | N1 | — | DEM5 |
| IVD-275 | Délibération d'urgence (délai réduit, motif) | DIR | 19 R-AG10 | N2 | — | DEM5 |
| IVD-276 | Consigner un incident de scrutin au PV | BUR | — | N1 | — | DEM3 |
| IVD-277 | Clore les mandats de fin d'année (élèves, parents annuels) | SYS | durée du type | — | — | DEM0 |
| IVD-278 | Prévenir un élu de la fin de son mandat | SYS | 8 | — | — | DEM7 |
| IVD-279 | Lien entre politique documentaire (45) et délibération | SYS | port | — | — | DEM6 |
| IVD-280 | Lien entre règlement intérieur et délibération (13, 7) | SYS | version exécutoire | — | — | DEM6 |
| IVD-281 | Lien entre structure et répartition des moyens (5) | SYS | — | — | — | DEM6 |

**Total : 182 actions.**

---

## 21. Écrans
| Écran | Route | Contenu |
|---|---|---|
| Tableau des instances | `/web/instances` | tuiles : élections en cours et prochaines échéances, compositions incomplètes, séances à venir, PV à approuver, actes non exécutoires |
| Instance | `/web/instances/:id` | composition à date (ligne de temps), mandats, séances, délibérations |
| Scrutin | `/web/elections/:id` | étapes avec dates, corps électoral, candidatures, matériel, bureau, résultats pas à pas |
| Campagne délégués | `/web/elections/delegues` | grille des classes, avancement, résultats |
| Bureau de vote | `/web/elections/:id/bureau` | émargement (recherche), compteur, clôture, double saisie |
| Séance | `/web/instances/seance/:id` | présents, quorum en direct, ordre du jour, votes, notes du secrétaire |
| Registre des actes | `/web/instances/actes` | recherche par objet, état (adoptée, transmise, exécutoire) |
| Espace électeur | portail famille / élève / personnel | mon inscription, calendrier, professions de foi, voter (si électronique), résultats |
| Espace élu | portail | mes mandats, documents, boîte des représentants, projets (CVL) |

---

## 22. Modèle de données cible (brique `instances`) [R]

Règles E1–E14 : `etablissement_id NOT NULL` partout (sauf `ref_*` globales, E12), FK composites, UUID applicatifs, `cree_le/modifie_le/version`, RLS, aucune colonne JSON ni tableau d'identifiants (D-036), fichiers en objets. **L'urne électronique est dans un schéma séparé** (`ent_urne`), avec un rôle PostgreSQL distinct, sans FK vers l'émargement.

```sql
-- 2920_instances_referentiel.sql
ref_instance_type (id, code, regime CHECK (public|prive_contrat|hors_contrat|organisme|libre), libelle, version, du, au NULL,
                   duree_mandat_mois INT NULL, publicite CHECK (non_publique|publique), source_ref TEXT)    -- globale_reference
ref_instance_college (id, type_id, code, libelle, voix CHECK (deliberative|consultative), ordre)       -- globale_reference
ref_instance_sieges (id, type_id, college_id, categorie_etab CHECK (...), sieges INT)                   -- globale_reference
ref_instance_regle (type_id, quorum_mode CHECK (majorite_en_exercice|tiers|aucun), seconde_conv_min_jours INT, seconde_conv_max_jours INT,
                    delai_convocation_jours INT, delai_urgence_jours INT NULL, procurations_max INT NULL, vote_secret_sur_demande bool,
                    distance_autorisee bool)                                                            -- globale_reference
ref_scrutin_mode (code PK, libelle)     -- proportionnelle_plus_fort_reste, uninominal_2_tours, binome, designation, tirage_au_sort
-- 2921_instances.sql
ent_instance (id, etablissement_id, type_id NULL, organisme_id NULL, libelle, annee_id, du, au NULL, etat)
ent_instance_siege (id, etablissement_id, instance_id, college_id NULL, rang INT, qualite CHECK (titulaire|suppleant), titulaire_siege_id NULL)
ent_mandat (id, etablissement_id, instance_id, siege_id, personne_id, fondement CHECK (election|designation|de_droit|tirage_au_sort),
            scrutin_id NULL, candidature_id NULL, fonction_ref UUID NULL, designant TEXT NULL, rang_liste INT NULL,
            du DATE, au_prevu DATE NULL, clos_le DATE NULL, motif_cloture CHECK (fin|demission|perte_qualite|deces|annulation|remplacement) NULL,
            piece_id NULL, ...,
            EXCLUDE USING gist (etablissement_id WITH =, siege_id WITH =, daterange(du, COALESCE(clos_le, au_prevu), '[)') WITH &&))
ent_association (id, etablissement_id, personne_morale_id, nature CHECK (parents|mdl|sportive|organisme|autre))
ent_association_representant (etablissement_id, association_id, personne_id, du, au NULL)
-- 2922_elections_corps.sql
ent_scrutin (id, etablissement_id, instance_id NULL, type_election_code, mode_code, classe_id NULL, sieges_titulaires INT, sieges_suppleants INT,
             date_reference DATE, date_scrutin DATE, ouverture TIME, fermeture TIME, modalite_urne bool, modalite_correspondance bool,
             modalite_electronique bool, etat CHECK (ouvert|liste_figee|candidatures_arretees|en_cours|clos|proclame|annule), tour INT DEFAULT 1,
             scrutin_premier_tour_id NULL, ...)
ent_scrutin_echeance (id, etablissement_id, scrutin_id, code, date_limite timestamptz, source CHECK (calcul|national|decision), motif NULL)
ent_scrutin_bureau (etablissement_id, scrutin_id, personne_id, role CHECK (president|assesseur|secretaire))
ent_liste_electorale (id, etablissement_id, scrutin_id, figee_le timestamptz, empreinte TEXT, figee_par)
ent_electeur (id, etablissement_id, liste_id, personne_id, college_code, inscrit_par CHECK (calcul|reclamation), raison_code,
              UNIQUE (etablissement_id, liste_id, personne_id))
ent_liste_reclamation (id, etablissement_id, liste_id, auteur_personne_id, objet, decision CHECK (acceptee|rejetee) NULL, decide_par NULL, motif NULL)
ent_liste_modification (id, etablissement_id, liste_id, electeur_id, sens CHECK (ajout|retrait), reclamation_id, decide_par, decide_le)
-- 2923_elections_candidatures.sql
ent_candidature (id, etablissement_id, scrutin_id, nature CHECK (liste|individuelle|binome), libelle, mandataire_personne_id NULL,
                 association_id NULL, deposee_le NULL, etat CHECK (brouillon|deposee|recevable|irrecevable|retiree), motif NULL)
ent_candidat (id, etablissement_id, candidature_id, personne_id, rang INT, qualite CHECK (titulaire|suppleant|binome_titulaire|binome_suppleant),
              accord_formulaire_ref UUID NULL, eligible_verifie_le NULL)
ent_profession_foi (id, etablissement_id, candidature_id, objet_id, deposee_le, etat CHECK (deposee|acceptee|refusee), motif_refus NULL, diffusee_le NULL)
-- 2924_elections_emargement.sql
ent_emargement (id, etablissement_id, scrutin_id, electeur_id, modalite CHECK (urne|correspondance|electronique), a timestamptz, saisi_par NULL,
                UNIQUE (etablissement_id, scrutin_id, electeur_id))
ent_correspondance_recue (id, etablissement_id, scrutin_id, electeur_id NULL, recue_le, etat CHECK (valide|tardive|non_identifiable|doublon))
-- 2925_elections_resultats.sql
ent_resultat_saisie (id, etablissement_id, scrutin_id, tour INT, saisie_no INT CHECK (saisie_no IN (1,2)), saisi_par, inscrits INT, votants INT,
                     blancs INT, nuls INT, exprimes INT, saisi_le)
ent_resultat_voix (etablissement_id, saisie_id, candidature_id, voix INT)
ent_resultat_siege (id, etablissement_id, scrutin_id, candidature_id, candidat_id, attribution CHECK (quotient|reste|majorite|egalite|tirage), etape INT, explication TEXT)
ent_tirage (id, etablissement_id, scrutin_id, objet, graine TEXT, resultat TEXT, temoins_texte TEXT, tire_le)
ent_scrutin_pv (id, etablissement_id, scrutin_id, objet_id, signe_le NULL)
ent_scrutin_recours (id, etablissement_id, scrutin_id, depose_par TEXT, depose_le, decision CHECK (rejete|annulation|reformation) NULL, decide_le NULL, piece_id NULL)
-- 2926_seances.sql
ent_seance (id, etablissement_id, instance_id, evenement_agenda_id, numero INT, urgence bool, motif_urgence NULL, ouverte_le NULL, close_le NULL, quorum_requis INT NULL)
ent_seance_point (id, etablissement_id, seance_id, ordre INT, intitule, nature CHECK (information|debat|vote|approbation_pv|question_diverse),
                  objet_type CHECK (sortie|tarif|bareme|budget|dbm|compte_financier|reglement_interieur|convention|contrat|structure|politique_documentaire|projet_etablissement|projet_cvl|libre) NULL,
                  objet_id UUID NULL, propose_par_brique TEXT NULL, rapporteur_personne_id NULL)
ent_seance_presence (id, etablissement_id, seance_id, mandat_id, statut CHECK (present|excuse|absent|represente), represente_par_mandat_id NULL, arrivee NULL, depart NULL)
ent_seance_deport (etablissement_id, point_id, mandat_id, motif)
ent_vote (id, etablissement_id, point_id, mode CHECK (main_levee|secret), presents INT, representes INT, pour INT, contre INT, abstentions INT, refus INT,
          quorum_atteint bool, adopte bool, a timestamptz)
ent_seance_pv (id, etablissement_id, seance_id, objet_id, etat CHECK (projet|diffuse|approuve|signe), approuve_seance_id NULL)
-- 2927_deliberations_actes.sql
ent_deliberation_compteur (etablissement_id, exercice INT, dernier INT, PRIMARY KEY (etablissement_id, exercice))
ent_deliberation (id, etablissement_id, vote_id NULL, instance_id, numero TEXT, exercice INT, objet_type, objet_id NULL, intitule, texte_objet_id,
                  nominative bool, adoptee bool, etat CHECK (adoptee|transmise|executoire|retiree|annulee|rejetee),
                  transmise_le NULL, accuse_ref TEXT NULL, executoire_le DATE NULL, remplacee_par NULL, publiee_le NULL, ...)
ent_acte_decision (id, etablissement_id, auteur_personne_id, objet_type, objet_id NULL, intitule, texte_objet_id, transmise_le NULL, executoire_le NULL)
ent_acte_controle (id, etablissement_id, deliberation_id NULL, decision_id NULL, nature CHECK (observation|defere|annulation), recu_le, piece_id NULL)
-- 2928_vie_lyceenne.sql
ent_projet_eleves (id, etablissement_id, instance_id, titre, description, porteurs_texte NULL, depose_par, depose_le,
                   etat CHECK (depose|vote_instance|transmis_ca|accepte|refuse|realise|abandonne), reponse NULL, repondu_le NULL, repondu_par NULL)
ent_projet_eleves_porteur (etablissement_id, projet_id, personne_id)
ent_formation_delegues (id, etablissement_id, evenement_agenda_id, annee_id) ; ent_formation_delegues_presence (etablissement_id, formation_id, mandat_id, present bool)
-- 2929_urne.sql  (schéma ent_urne, rôle distinct, lot DEM10)
ent_urne.election_cle (id, etablissement_id, scrutin_id, cle_publique TEXT, seuil INT, parts INT, scellee_le, empreinte_scellement TEXT, detruite_le NULL)
ent_urne.part_detenue (id, etablissement_id, election_cle_id, detenteur_personne_id, empreinte_part TEXT, remise_le, detruite_le NULL)  -- la part elle-même n'est jamais stockée en clair côté serveur
ent_urne.bulletin (id, etablissement_id, scrutin_id, chiffre TEXT, empreinte TEXT, depose_le_arrondi timestamptz, chaine_precedente TEXT) -- aucun identifiant d'électeur
ent_urne.jeton_vote (empreinte_jeton TEXT PK, etablissement_id, scrutin_id, consomme bool)   -- émis par l'émargement, consommé sans lien
ent_urne.journal (id, etablissement_id, scrutin_id, a, evenement, empreinte, chaine_precedente)
-- 2930 RLS stricte, droits du rôle de l'urne ; 2931–2938 réserve ; 2939 retrait (réservé, E8)
```
Notes : `ent_resultat_siege.explication` et `ent_tirage.temoins_texte` sont des **textes figés** (instantanés, exception permise D-036). `depose_le_arrondi` arrondit l'heure (au quart d'heure) pour empêcher le rapprochement temporel avec l'émargement ; l'ordre des bulletins publiés est mélangé. Neutralisation (E9) : à la restauration, tout scrutin électronique `en_cours` est **suspendu** et signalé au bureau (IVD-260).

---

## 23. Reprise et coexistence
- Rien à reprendre de Next.js ni de Charlemagne de façon structurée. **Saisie initiale** des compositions et mandats de l'année en cours (DEM0) à partir des PV papier (pièces au 7).
- Saisies locales du 13 (lot D5) et du 23 : converties en mandats (IVD-110), puis les écrans locaux du 13 et du 23 **lisent** le 46.
- Doublon 2027-2028 (D-034) : dans le privé pilote, les élections de délégués de classe et les instances de l'organisme peuvent être en **usage réel** (aucun concurrent officiel) ; l'établissement décide ; données réelles après l'accord écrit et l'avis du DPO (D-035 point 4).

---

## 24. Lots de code (équipe Sonnet) et migrations

### 24.1 Lots
| Lot | Contenu | Dépend de | Critères de fin |
|---|---|---|---|
| **DEM0** fondation et mandats | référentiel des types (premiers types : CA, commission permanente, conseil de discipline, commission éducative, CVL, CVC, AG des délégués, conseil d'établissement, instance libre), instances, sièges, mandats saisis, clôtures et remplacements, ports `composition`, `mandats_de`, `delegues`, `regle_quorum` ; conversion des saisies du 13 et du 23 ; registre E12, RLS | 1, 2, 4, 5, 6, 7 | 13 et 23 lisent le 46 ; pièges n° 6, 9 |
| **DEM1** délégués de classe | scrutin uninominal à deux tours, campagne par classe, assistant du PP, double saisie, égalités, PV, proclamation, mandats, lecture au 23 | DEM0 | jeux d'or ; CIV-34 ; R-IVD10, R-IVD16 |
| **DEM2** corps électoraux et listes | calcul (2, 4, 6), anomalies, explication, figement, consultation minimisée, réclamations, journal, couche nationale du calendrier | DEM0 ; port nouveau du 4 (Q-IVD3) | CIV-01 à CIV-13 ; R-IVD4 à R-IVD8 |
| **DEM3** élections au CA (public) | listes, candidats, accords, professions de foi, matériel, correspondance, bureau, émargement assisté, proportionnelle au plus fort reste, carence, PV, export national, recours, report | DEM1, DEM2 ; 7, 8, 9, 17, 18 | CIV-15 à CIV-33 ; jeux d'or réels (PV d'années passées anonymisés) |
| **DEM4** élections internes et autres | CVL, CVC, élèves au CA, commission permanente, conseil de discipline, délégués des parents au conseil de classe, désignations libres, élections partielles, contrôles de cumul | DEM1, DEM3 | 13 reçoit les élus |
| **DEM5** séances et délibérations | demande au 19, ordre du jour typé, documents, présents, quorum continu, procurations, déport, votes, délibérations numérotées, PV et approbation, tableau des instances, urgence | DEM0 ; 19 AG7 | CIV-40 à CIV-48 ; R-IVD17 à R-IVD19 |
| **DEM6** actes | transmission déclarée, exécutoire calculé, contrôle, publicité, registre, port `deliberation_executoire`, alertes chez les consommateurs | DEM5 ; 42, 24, 45, 48 | R-IVD20 à R-IVD22 ; piège n° 8 |
| **DEM7** communication | annuaire des élus, boîte des représentants (16), publications (17), associations, canal égal, invités extérieurs, résultats publiés | DEM1, DEM3 ; 16, 17 | R-IVD12, R-IVD13, R-IVD23 |
| **DEM8** vie lycéenne | projets, réponses datées, budget (lecture 48), formation, AG des délégués, absences pour mandat, engagements LSL, espace encadré | DEM4, DEM5 ; 12, 23 | R-IVD25, R-IVD30 |
| **DEM9** privé et organisme | conseil d'établissement, instances de l'organisme, votes d'AG papier, désignations par l'association, mandats du CSE pour information | DEM0, DEM5 | 24 et 48 lisent les délibérations de l'organisme |
| **DEM10** vote électronique (cœur) | schéma de l'urne, scellement à seuil, jetons, vote, accusé, clôture, dépouillement, intégrité ; vote d'élèves en présentiel | DEM3 ; **texte vérifié + expertise** pour l'activation | tests cryptographiques ; R-IVD14, R-IVD15 ; piège n° 1, 10 |
| **DEM11** vote électronique à distance et vote de séance | secrets par un autre canal, parents et personnels (si texte), vote anonyme en séance sur tablette, expertise indépendante | DEM10 | rapport d'expertise |
| **DEM12** IA | capacités du §19.3 | DEM5 ; `ent-ia` | piège n° 7 ; injection dans une profession de foi |
| **DEM13** conservation | purges, destructions prouvées, versement aux archives, export des données personnelles, indicateurs | tous ; 27, 47 | §17.3 vérifié |

**Parcours minimal** : V1 de février 2027 = DEM0, DEM1, DEM2, DEM5 (+ DEM6 pour le 42) ; rentrée 2027 = DEM3, DEM4, DEM7 ; DEM10–DEM11 seulement après décision de Florian (§28.2).

### 24.2 Migrations (plage imposée 2920–2939)
| N° | Lot | Contenu |
|---|---|---|
| 2920 | DEM0 | référentiel global des types, collèges, sièges, règles, modes |
| 2921 | DEM0 | instances, sièges, mandats, associations |
| 2922 | DEM1, DEM2 | scrutins, échéances, bureau, listes électorales, électeurs, réclamations |
| 2923 | DEM1, DEM3 | candidatures, candidats, professions de foi |
| 2924 | DEM3 | émargement, correspondance |
| 2925 | DEM1, DEM3 | résultats, sièges, tirages, PV, recours |
| 2926 | DEM5 | séances, points, présences, déports, votes, PV |
| 2927 | DEM6 | délibérations, décisions, contrôle |
| 2928 | DEM8 | vie lycéenne |
| 2929 | DEM10 | schéma de l'urne (rôle distinct) |
| 2930 | DEM13 | RLS stricte, droits du rôle de l'urne |
| 2931–2938 | — | réserve (passe personas, §31) |
| 2939 | — | retrait (réservé, E8) |

## 25. Contradictions et points signalés
| # | Point | Proposition |
|---|---|---|
| C-IVD1 | `SOCLES.md` : 46 « dépend de 4, 6, 18 » | ajouter 19 (logistique) et 7 ; le 18 n'est utile que pour les accords de candidats |
| C-IVD2 | `13_DISCIPLINE.md` lot D5 et `23_BULLETINS.md` prévoient des saisies locales de composition et de délégués | convertir en mandats du 46 (IVD-110) dès DEM0 ; le 13 et le 23 lisent le port |
| C-IVD3 | `19_AGENDA_RDV.md` CAG-48 : « procuration pour une instance du 46 seulement » | cohérent ; la règle (nombre, collège) vient du référentiel du 46 |
| C-IVD4 | `17_ANNONCES.md` R-PB36 (professions de foi relayées à l'identique) | repris (IVD-138) ; le 46 fournit l'échéance |
| C-IVD5 | Mot « délégué » ambigu (délégations du socle 1) | vocabulaire R-IVD34 |

## 26. Échanges nationaux (avec le socle 9)
| Flux | Sens | Contenu | Statut |
|---|---|---|---|
| Résultats des élections de parents au CA | sortant | inscrits, votants, exprimés, sièges par liste et appartenance [NV : application et format, §5] | export pré-contrôlé, dépôt déclaré ; connecteur après obtention des spécifications |
| Résultats des élections de personnels | sortant | idem [NV] | idem |
| Actes au contrôle de légalité | sortant | acte, annexes [NV : plateforme et format] | paquet préparé, dépôt déclaré |
| Calendrier national des élections | entrant (opérateur) | dates de l'année | couche nationale (IVD-116) |

## 27. Réglages signalés (D-023) — S-4600 à S-4612
| # | Réglage | Version automatique |
|---|---|---|
| S-4600 | règles d'un type réglementaire | **non réglables** |
| S-4601 | délais des étapes d'un scrutin | calculés, non réductibles |
| S-4602 | modalités du scrutin (urne, correspondance, électronique) | urne + correspondance ; électronique masqué tant que R-IVD27 n'est pas remplie |
| S-4603 | consultation de la liste électorale par les électeurs | nom et prénom seulement |
| S-4604 | seuil de publication des résultats détaillés | 10 votants, fixe |
| S-4605 | numérotation des délibérations | `AAAA-NNN`, fixe |
| S-4606 | instances libres | autorisées, sans règle |
| S-4607 | délai de réponse aux projets du CVL | 30 jours, fixe |
| S-4608 | publication des actes au portail des familles | actes publics seulement, automatique après exécutoire |
| S-4609 | trame du PV et du matériel | 3 trames (D-023) |
| S-4610 | double saisie des résultats | obligatoire |
| S-4611 | seuil de la clé partagée | 3 sur 5 par défaut ; 2 sur 3 pour un petit bureau |
| S-4612 | arrondi de l'horodatage des bulletins | quart d'heure, fixe |

---

## 28. Questions (Q-IVD) et décisions

### 28.1 Questions (Q-IVD) et recommandation appliquée par défaut (D-031)
| N° | Question | Recommandation | État |
|---|---|---|---|
| Q-IVD1 | Relecture mot à mot des articles R421-14 à R421-45, R511-20 à R511-22, D111-7 à D111-15, L421-14, R421-54 (lus par fragments) | avant DEM1 à DEM6 | à faire (humain ou session avec accès complet) |
| Q-IVD2 | Bureau, carence, tirage au sort des parents en EPLE ; remontée des résultats (application, format) | lire l'arrêté du 13-5-1985 modifié et la note technique DGESCO du 21-7-2026 | à faire ; IVD-139 et IVD-158 restent [NV] |
| Q-IVD3 | Port `liens::responsables_electeurs(eleve, date)` au socle 4 | demandé au 4 (autorité parentale non retirée, tiers légal) | appliqué par défaut |
| Q-IVD4 | Le 46 gère-t-il les instances de l'organisme de gestion ? | oui (type `organisme`), vote d'AG papier assisté | appliqué par défaut |
| Q-IVD5 | Que voit un électeur de la liste électorale ? | sa propre inscription ; la liste complète au bureau et dans l'ENT authentifié pour le bureau et les délégués de liste (R-IVD37) | appliqué par défaut |
| Q-IVD6 | Séance du CA à distance | non (texte non confirmé) | appliqué par défaut |
| Q-IVD7 | Saisie double des résultats obligatoire ? | oui (R-IVD10) | appliqué par défaut |
| Q-IVD8 | Transmission des actes : plateforme, format, publicité | dépôt déclaré tant que non vérifié | à vérifier ; **démarche** éventuelle d'accès à la plateforme : à Florian |
| Q-IVD9 | Vote de séance anonyme sur tablette | lot DEM11, après le vote électronique des parents | appliqué par défaut |
| Q-IVD10 | Héberger les élections internes ou les adhérents des associations ? | non sans convention tripartite | **à Florian** (engage des tiers) |
| Q-IVD11 | Élections du CSE de l'organisme | hors périmètre ; mandats saisis pour information | appliqué par défaut |
| Q-IVD12 | Vote électronique d'élèves (même en présentiel) | **non** tant qu'aucun texte ne l'autorise | appliqué par défaut ; validation juridique : à Florian s'il veut le rouvrir |
| Q-IVD13 | Forme de publicité de la liste électorale arrêtée | consultation, jamais d'affichage libre | à vérifier (arrêté de 1985, note de service) |
| Q-IVD14 | Contrôle des professions de foi des élèves | aucun contrôle a priori par défaut ; signalement | à vérifier |
| Q-IVD15 | Délégués des parents au conseil de classe : désignation sur proposition des listes | moteur de désignation selon les résultats | à vérifier |
| Q-IVD16 | AESH et AED dans le collège 1 | oui, interprétation affichée, décision tracée possible | à confirmer par le rectorat |
| Q-IVD17 | Contrat de sous-traitance (art. 28) pour le vote électronique | clauses à ajouter au contrat type | **à Florian** (contrat) |
| Q-IVD18 | Expertise indépendante du vote électronique : quand, qui, combien | après DEM10, avant toute campagne réelle ; viser octobre 2028 | **à Florian** (coût) |

### 28.2 Ce qui engage Florian
Expertise indépendante et calendrier du vote électronique ; contrat de sous-traitance ; conventions avec des associations ; relecture juridique des [V-rés] et [NV] ; avec le pilote : activation des délégués et du CVL dans le privé, usage réel en 2027-2028 ; avis du DPO (durées, AIPD).

## 29. Faits, hypothèses, recommandations
- **Faits [F]** : aucun code ni module ; ports attendus par 13, 19, 23, 42, 17, 16 ; port `personnels::electeurs` prévu par le 6 ; logistique des réunions au 19.
- **Hypothèses [H]** : pratiques des secrétariats ; pratiques du privé (parents correspondants, conseil d'établissement) ; calendrier de l'année (§4).
- **Recommandations [R]** : règles R-IVD ; vote papier assisté d'abord ; vote électronique conditionnel ; associations comme tiers ; CSE hors périmètre.

## 31. Ajouts de la passe personas (2026-11-11)

**Ajout personas** (`PERSONAS_46.md`, 62 manques, 3 bloquants). **Ce paragraphe l'emporte sur les sections précédentes en cas de contradiction.** Ajouts issus du §5 (réglementaire) regroupés ici aussi.

### 31.1 Bloquants appliqués par défaut (D-031)
- **A — R-IVD37** : la liste électorale n'est jamais affichée en accès libre ; consultation de sa propre inscription par chaque électeur ; liste complète au bureau et dans l'ENT authentifié (bureau, délégués de liste) ; papier au bureau de vote seulement ; personnes à adresse ou identité protégée sous mention neutre. Remplace l'« affichage » du §9.1 (étape 3) et IVD-124.
- **B — R-IVD38** : un mandat d'instance renouvelée annuellement se clôt par **événement** (première séance de la nouvelle instance après la proclamation), pas par date ; `ent_mandat.clos_par_evenement` ; les élus du conseil de discipline de N siègent jusqu'à l'élection de ceux de N+1. IVD-277 (clôture de fin d'année) ne s'applique qu'aux mandats à durée fixe (délégués de classe : jusqu'à l'élection suivante, de même).
- **C — Accord exprès aux associations** recueilli dans la fiche de rentrée (4) ou un formulaire du 18 fin août, par parent et par association, révocable (IVD-284).

### 31.2 Règles ajoutées
| N° | Règle |
|---|---|
| R-IVD35 | L'inéligibilité (privation de droits civiques ou de famille) est une **déclaration** du candidat ; aucun casier ni pièce judiciaire stocké. |
| R-IVD36 | En cas de partage égal des voix en séance, la voix du président est prépondérante quand le type le prévoit (R421-25). |
| R-IVD39 | Une délibération a une **portée** : `decision` ou `avis` (R421-23, conseil d'établissement du privé) ; un avis n'est ni transmis ni exécutoire. |
| R-IVD40 | La commission permanente ne délibère que sur les objets **délégués** par le CA lors de sa première séance (R421-22) ; un objet non délégué est refusé. |
| R-IVD41 | Un membre élève (mineur) ne reçoit pas les documents de séance nominatifs (remise de dette, recrutement, situation individuelle) [NV] ; ils sont exclus de son dossier de séance. |
| R-IVD42 | Une convocation ne part pas si une pièce annoncée à l'ordre du jour manque (D111-13). |
| R-IVD43 | Une association ne reçoit aucun accès aux données des familles ; ses désignations sont enregistrées par la direction. |
| R-IVD44 | Le secret de vote électronique est envoyé à l'adresse **propre** de chaque électeur ; à défaut, courrier ou urne. |
| R-IVD45 | Les jetons de vote sont émis après le figement de la liste et révocables sur décision du bureau. |
| R-IVD46 | Les résultats sont scellés à la signature du PV ; toute correction est une décision tracée avec un nouveau PV. |
| R-IVD27 (précisée) | Vote électronique : **seulement** l'élection des parents au CA de l'EPLE public (R421-30, arrêté du 2-7-2024), jamais les personnels, jamais les élèves ; scrutin mixte : l'électronique précède, les plis de ceux qui ont voté sont écartés sans ouverture. |
| R-IVD24 (précisée) | Les noms des élèves élus ne sont publiés qu'à l'intérieur de l'établissement (ENT), jamais sur un site public. |

### 31.3 Actions ajoutées (IVD-282 à IVD-305)
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| IVD-282 | Changer une modalité de vote en cours d'opérations (décision nationale, incident) | DIR, BUR | décision tracée ; BO n° 38 [NV] | N3 | `elections.modalite_modifiee` | DEM3 |
| IVD-283 | Générer le kit RGPD du scrutin (registre, mentions, trame d'AIPD au-delà de 1 000 électeurs) | DIR, DPO | — | N1 | — | DEM10 |
| IVD-284 | Recueillir l'accord exprès des parents par association et exporter la liste des seuls consentants | SYS (4, 18) ; DIR pour l'export | D111-7 et s. ; révocable ; trace d'export (9) | N3 | `elections.export_associations` | DEM2 |
| IVD-285 | Poser le profil de régime de l'établissement (types visibles) | OPS | S-4613 | N2 | — | DEM0 |
| IVD-286 | Voter les délégations du CA à la commission permanente (première séance) | CA | R421-22 | N2 | `instances.delegation_votee` | DEM5 |
| IVD-287 | Tenir la couche académique du calendrier (report par académie) | OPS | BO n° 36 [V] | N2 | — | DEM2 |
| IVD-288 | Tracer la remise des plis par les élèves (par classe) | SDIR, PP | sans adresse | N1 | — | DEM3 |
| IVD-289 | Enchaîner les élections (classe → AG → conseil de discipline ; CVL → CA et vice-président) | CPE | dépendances | N1 | — | DEM4 |
| IVD-290 | Alerter si le CVL n'est pas réuni avant un CA ordinaire | SYS | R421-44 | — | — | DEM8 |
| IVD-291 | Avertir une personne protégée avant le dépôt de sa candidature | SYS | 4 | — | — | DEM3 |
| IVD-292 | Remplacer un candidat jusqu'à 8 jours francs avant | CAND | BO n° 36 [V] | N2 | — | DEM3 |
| IVD-293 | Enregistrer les parents correspondants désignés par l'association de parents | DIR | R-IVD43 | N1 | — | DEM9 |
| IVD-294 | Ouvrir un poste de vote dédié dans l'établissement (mode borne) | DIR | arrêté 2024 art. 9 | N2 | — | DEM10 |
| IVD-295 | Figer la version de l'urne pour une campagne (après expertise) | OPS | S-4616 | N3 | — | DEM11 |
| IVD-296 | Grille de conformité aux recommandations CNIL de 2019 et de 2026 | OPS | — | N0 | — | DEM11 |
| IVD-297 | Cellule d'assistance et guide de l'établissement pour le vote | DIR, OPS | arrêté 2024 art. 7 | N1 | — | DEM10 |
| IVD-298 | Remettre la notice d'accès au vote électronique au moins 6 jours avant | SYS | arrêté 2024 art. 9 | N3 | — | DEM10 |
| IVD-299 | Fixer la durée du vote électronique (24 h à 5 jours) | DIR | contrôle | N2 | — | DEM10 |
| IVD-300 | Associer un délégué de chaque liste au bureau du vote électronique | DIR | arrêté 2024 art. 7 ; détenteur d'une part de clé | N2 | — | DEM10 |
| IVD-301 | Voter la composition et le mode d'élection du CVC (délibération du CA) | CA | R421-45-1 | N2 | — | DEM4 |
| IVD-302 | Campagne « éco-délégués » (au moins un élu par classe) | CPE | circulaire 2020 [V-copie] | N2 | — | DEM4 |
| IVD-303 | Retirer un document nominatif du dossier de séance d'un membre élève | SYS | R-IVD41 | — | — | DEM5 |
| IVD-304 | Bloquer une convocation dont une pièce annoncée manque | SYS | R-IVD42 | — | — | DEM5 |
| IVD-305 | Programmer purges et versements dès la création du scrutin (tableau de tri 2005) | SYS | §5.8 | — | — | DEM13 |

**Total après la passe : 205 actions** (IVD-100 à IVD-305, dont IVD-252 retirée : 205 actives).

### 31.4 Modèle de données (dans la plage 2920–2939)
`ref_scrutin_mode` + `plurinominal_1_tour` (2920) ; `ref_acte_regime (objet_type PK, regime CHECK (immediat|plus_15_jours|avis), source_ref)` globale (2920) ; `ent_instance` + `deliberation_id NULL` (CVC paramétré par le CA) (2921) ; `ent_mandat` + `clos_par_evenement CHECK (installation_nouvelle_instance|election_suivante) NULL` (2921) ; `ent_deliberation` + `portee CHECK (decision|avis)`, `deleguee bool` (2927) ; `ent_delegation_cp (etablissement_id, instance_id, objet_type, deliberation_id)` (2927) ; `ent_accord_association (id, etablissement_id, personne_id, association_id, donne_le, retire_le NULL, annee_id, source_formulaire_ref)` (2922) ; `ent_scrutin` + `duree_electronique_heures INT NULL`, `notice_envoyee_le NULL` (2922) ; `ent_scrutin_calendrier_academie (etablissement_id NULL → globale, academie_code, type_election_code, date_scrutin, source_ref)` (2920, globale_reference) ; `ent_resultat_scelle (etablissement_id, scrutin_id, empreinte, scelle_le)` (2925). Aucun JSON.

### 31.5 Corrections de sections antérieures
- §9.1 étape 3 (affichage) → R-IVD37 ; §9.3 (CVL : binômes) → plurinominal à un tour (déjà corrigé dans le tableau) ; §11.3 → suspendu (déjà corrigé) ; §17.3 (conservation) → tableau de tri de 2005 (§5.8) : bulletins 3 mois, listes électorales et émargements 2 mandats puis destruction, PV de dépouillement 2 mandats, professions de foi 1 an, **PV du CA conservation définitive** ; §8.3 (dates des mandats) → R-IVD38 ; IVD-124 → consultation (R-IVD37) ; IVD-137 → signalement (déjà corrigé).
- Lots : DEM10 = vote électronique des **parents** (urne, scellement à seuil avec les délégués de liste, poste dédié, notice J-6, scrutin mixte) ; DEM11 = expertise, version figée, vote de séance.

### 31.6 Cas limites ajoutés
CIV-53 à CIV-68 : `PERSONAS_46.md` §4.
