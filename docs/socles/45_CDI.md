# Socle 45 — CDI et ressources documentaires

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian). Rédigée **avant** tout code. Aucun code écrit ni modifié ; aucun accès à une base, à un secret ou à un service de production. Le code Next.js n'a été lu que pour vérifier qu'il n'existe **aucun** module CDI (noms de fichiers, `app/` sur la branche de travail).
**Branche :** `ent-socle-45-46-conception`, partie de `origin/ent-mc0hoe` (`37ce0d3b`).
**Passe personas :** `docs/socles/PERSONAS_45.md` (même branche). Les ajouts qu'elle a faits ici sont marqués **« ajout personas »** et regroupés au **§30**, qui l'emporte en cas de contradiction ; rien n'a été supprimé.
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` (D-017, D-020, D-021, D-022, D-023, D-024, D-026, **D-029**, **D-031**, **D-034**, **D-035**, **D-036**, D-037 à D-041) ; `docs/SOCLES.md` (§2, socles 9, 44, 45) ; `docs/MATRICE_CHARLEMAGNE.md` (VOY-14) ; `docs/PRA_SAUVEGARDES.md` §18.4 (E1–E14) ; conceptions `09_INTEROP.md` (§11 GAR, ITP-84 à ITP-87, P16 de `PERSONAS_09.md`), `19_AGENDA_RDV.md` (réservation de lieux), `12_ASSIDUITE.md`, `10_EMPLOI_DU_TEMPS.md`, `24_FINANCES.md`, `29_PORTAILS.md` (tuile « Prêts CDI à rendre », tuile du documentaliste), `IA_COEUR.md` (§5, §6.2 niveaux N0–N3). Modèles de forme : `42_SORTIES_VOYAGES.md`, `10_EMPLOI_DU_TEMPS.md`, `21_EVALUATIONS.md`.

**Conventions.** [F] fait vérifié dans le code ou les documents ; [V] règle lue sur une source officielle (URL et date au §5) ; [V-copie] texte officiel lu sur une copie académique ; [NV] non vérifié (interdit de coder dessus sans vérification) ; [H] hypothèse de terrain ; [R] recommandation, non décidée, **appliquée par défaut (D-031)** sauf mention contraire.

**Numérotation propre au socle** (vérifiée libre sur les 134 branches distantes le 2026-11-11) :

| Élément | Numéros |
|---|---|
| Actions cibles | **CDI-100**… |
| Règles | **R-CDI1**… |
| Cas limites | **CCD-01**… |
| Questions | **Q-CDI1**… |
| Réglages signalés (D-023) | **S-4500 à S-4549** |
| Manques de la passe personas | **M45-01**… |
| Lots | **BIB0 à BIB13** |

**Plage de migrations imposée : 2900–2919** (inscrite dans `MIGRATIONS.md`).

---

## 0. En bref

1. **Rien n'existe** : ni dans l'ENT Rust (aucune table, aucune route, seule la fonction `documentaliste` du catalogue des fonctions du socle 6, migration 353 [F]), ni dans l'add-on Next.js (aucun module CDI [F]), ni dans Charlemagne de façon utile (VOY-14 « catalogue CDI » noté « N(S) » dans la matrice). Il n'y a donc **pas de lot 0 de correctifs** ; le premier lot est un lot de fondation (BIB0).
2. **Thèse** : le CDI n'est pas une petite bibliothèque. C'est **un lieu** (qui y est, à quelle heure, sous la responsabilité de qui), **un fonds** (notices, exemplaires, périodiques, manuels, ressources numériques), **un service de prêt** (avec une confidentialité des lectures que les logiciels de vie scolaire ignorent), **un enseignement** (éducation aux médias et à l'information, séances co-animées) et **une porte vers le numérique** (médiacentre et GAR). Les logiciels de CDI du marché (SIGB) font le fonds et le prêt ; les logiciels de vie scolaire font l'élève. **La valeur de ScolIA est de les réunir** : l'emprunteur est l'élève du socle 2, sa classe vient du socle 5, sa présence au CDI se lit dans l'appel (12), son départ déclenche le bilan des prêts (2), ses ressources numériques passent par le GAR (9), la séance d'EMI est une séance de l'emploi du temps (10).
3. **Confidentialité des lectures** (R-CDI1 à R-CDI4) : ce qu'un élève lit ne regarde ni la direction, ni ses professeurs, ni (au-delà du nécessaire) ses parents. L'historique nominatif est **délié** à J+30 après le retour ; les statistiques restent, anonymes. Un parent voit qu'un document est en retard, pas lequel, sauf au stade de la perte (R-CDI3) — **sauf pour un élève de moins de 15 ans**, dont les parents voient le titre dès la deuxième relance (R-CDI3 bis, passe personas).
4. **Aucune contrainte sur les documents officiels** : un retard ou une perte ne bloque jamais un bulletin, un certificat, un exeat ni une inscription (R-CDI7). La perte suit le barème **délibéré** par le conseil d'administration (public) ou fixé par l'organisme de gestion (privé), facturé par le socle 24, jamais une « amende » de retard.
5. **Le lieu est un sujet de sécurité** : les élèves présents au CDI pendant une heure de permanence sont **localisés** (R-CDI10) ; la liste est lue par la vie scolaire (12) et par l'alerte (14). Une jauge et un responsable présent sont exigés pour ouvrir.
6. **GAR** : ScolIA ne fait **pas** les affectations (console du GAR, `09_INTEROP.md` §11). Le 45 montre au documentaliste, souvent **responsable d'affectation**, l'état de l'alimentation, les ressources affectées, et le diagnostic « pourquoi cet élève ne voit pas cette ressource » ; il intègre les ressources libres hors GAR (portails publics) dans un même portail.
7. **Reprise** : le pilote a probablement un SIGB (BCDI/e-sidoc ou PMB [H, Q-CDI1]). Le 45 importe les notices et exemplaires (UNIMARC/MARC, CSV), **jamais** l'historique de prêt clos. Deux régimes : SIGB complet (cible) ou passerelle (catalogue importé chaque nuit, prêt resté dans l'ancien outil) pendant l'année de doublon.
8. **Manuels scolaires** : moteur de prêt commun, **campagnes par classe** (distribution de septembre, retour de juin, état constaté, perte selon barème). Recommandation : les manuels sont portés par le 45 (ce sont des titres et des exemplaires), le matériel (ordinateurs, tablettes, calculatrices, clés) reste au 44 qui **réutilise le même moteur d'exemplaires et de prêt** (Q-CDI2, appliqué par défaut).
9. **IA** : notices proposées depuis l'ISBN ou une photo de couverture, indexation suggérée, sélection rédigée, désherbage proposé, aide à la recherche documentaire **qui enseigne** (sources, critères), jamais une recommandation de lecture fondée sur l'historique personnel d'un mineur (R-CDI5), jamais une décision de perte ou de facturation.
10. **Chiffres** (après la passe personas) : **193 actions** (CDI-100 à CDI-292), 42 règles, 62 cas limites, 15 lots (BIB0–BIB13, BIB10 scindé en 10a/10b), 0 JSON (D-036). Parcours minimal pour la démo et la V1 de février 2027 : BIB0–BIB4 ; usage réel du pilote à la rentrée 2027 : BIB0–BIB7 + BIB10a.

---

## 1. Thèse : un lieu, un fonds, un service, un enseignement, une porte

| Facette | Ce qu'elle est | Ce que les outils actuels font mal [H] | Ce que fait ScolIA |
|---|---|---|---|
| **Lieu** | Une salle ouverte à des heures, avec une jauge, sous la responsabilité d'un adulte, où la vie scolaire envoie des élèves | Billet papier « autorisé au CDI », cahier de présence, aucune trace dans l'appel ; en cas d'alerte, personne ne sait qui est au CDI | Présence au CDI = **localisation** lue par la vie scolaire et l'alerte ; créneaux réservables par les élèves de permanence ; jauge ; ouverture liée à la présence d'un responsable |
| **Fonds** | Notices, exemplaires, périodiques, manuels, ressources numériques, expositions, jeux | SIGB séparé, comptes élèves importés à la main chaque septembre, classes fausses en octobre | Emprunteurs = personnes du cœur, classes à jour, départs propagés |
| **Service de prêt** | Prêter, rendre, prolonger, réserver, relancer, constater une perte | Relances papier par le professeur principal, listes de retards épinglées, titres exposés à la classe | Relances graduées **sans exposer les titres**, bilan de fin d'année au PP sans titres, perte facturée par le 24 selon barème délibéré |
| **Enseignement** | Éducation aux médias et à l'information, séances co-animées, projets (presse, prix littéraires) | Séances hors emploi du temps, non comptées dans le service, non vues par les familles | Séance = séance du 10 (co-intervention), cahier de textes (20), compétences (22) |
| **Porte numérique** | Médiacentre, GAR, ressources libres, sitothèque | « Je ne vois pas Europresse » sans diagnostic ; mots de passe partagés au tableau | Portail unique, diagnostic GAR, **aucun identifiant partagé** stocké en clair |

Principes structurants [R] :
- **Un emprunteur est une personne** (D-020), pas un compte : l'élève sans compte (connexion en retard) emprunte au comptoir avec sa carte.
- **Un exemplaire est un objet physique** ; une **notice** décrit l'œuvre ; un **titre de manuel** est une notice de nature `manuel` ; le matériel du 44 est un exemplaire sans notice bibliographique (même table de prêt, autre nature).
- **Le prêt est un fait daté** (prêté, prolongé, rendu, perdu) en ajout seul ; le lien nominatif est **délié** après le délai de R-CDI2, le fait reste pour la statistique.

---

## 2. Évaluation franche de l'existant

### 2.1 ENT Rust [F]
- Aucune table `cdi`, `pret`, `notice`, `exemplaire` dans `ent-rust/migrations/` (231 fichiers parcourus par recherche de mots-clés, 2026-11-11).
- `353_agents_fonctions.sql` : fonction fournie `documentaliste` (famille `enseignement`, profil proposé `prof`) [F]. Conséquence : le documentaliste reçoit aujourd'hui le modèle de rôle « professeur » (D-017) ; il lui faut un **modèle de rôle « Documentaliste »** (§21.1), car la portée « mes classes » ne correspond pas à son métier (il voit **tous** les élèves comme emprunteurs, mais aucune de leurs notes).
- `ent-rust/web/` : aucun module CDI ; la tuile « Prêts CDI à rendre » du portail élève et la tuile du documentaliste sont décrites par `29_PORTAILS.md` (lignes 556 et 579) mais non codées.
- `09_INTEROP.md` §15.4 prévoit `ent_gar_parametres` et `ent_gar_responsable_affectation` (3912) ; ce sont des tables du socle 9, **lues** par le 45 (§6). Remarque hors périmètre : `ent_idp_identifiant_service.personne_ref JSONB` (3911) contredit D-036 ; signalé au §28 (C-CDI4), pas corrigé ici.

### 2.2 Next.js [F]
Aucun module CDI, bibliothèque, prêt de livres ou manuels dans `app/` (recherche par noms de fichiers et de routes). Ni le T0 (`TRANSITION_T0_INVENTAIRE.md` §3.1, M01–M20) ni la matrice n'en listent. **Rien à reprendre, rien à ne pas perdre.** L'espace documentaire des personnels (M19) relève du socle 32, pas du CDI.

### 2.3 Charlemagne et SIGB du marché [H]
La matrice ne lit pas la documentation de Charlemagne (VOY-14, « N(S) »). Les CDI utilisent en pratique un SIGB dédié (BCDI et son portail e-sidoc, ou PMB) [H, Q-CDI1]. La parité à viser est donc celle d'un **SIGB scolaire**, pas de Charlemagne : catalogue avec import de notices, prêts, réservations, périodiques, inventaire et récolement, statistiques, portail de recherche.

### 2.4 Verdict
Construction neuve. Le risque n'est pas de « mal reprendre », c'est de **sous-estimer** : un CDI qui doit garder BCDI parce que ScolIA ne sait pas récoler ou bulletiner une revue ne bascule jamais. D'où un parcours qui atteint la **parité SIGB** dès BIB5, avant les fonctions différenciantes.

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps [H] | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Professeur documentaliste** (souvent seul, parfois à temps partagé sur deux établissements) | Gère le fonds, prête, accueille, enseigne l'EMI, co-anime, conseille, achète, désherbe ; souvent responsable d'affectation GAR | Import des élèves dans le SIGB chaque rentrée ; classes fausses ; relances une par une ; récolement au tableur ; GAR qui « ne marche pas » sans savoir pourquoi | Être seul responsable de 40 élèves envoyés par la vie scolaire sans prévenir ; perdre l'historique du fonds à la reprise ; exposer la lecture d'un élève | Emprunteurs toujours à jour ; présence au CDI tenue par le logiciel ; scan au téléphone ; relances automatiques et discrètes ; diagnostic GAR en un clic |
| **AED ou assistant affecté au CDI** | Tient le comptoir et la salle en l'absence du documentaliste | Ne sait pas les règles ; pas de droits sur le SIGB | Prêter à un élève déjà en retard ; ne pas savoir qui est là | Écran comptoir simple, règles appliquées par le logiciel |
| **Élève de collège** (11–15 ans) | Emprunte, lit sur place, fait ses devoirs, cherche pour un exposé | Ne sait pas si le livre est là ; perd son billet de permanence | Être grondé devant la classe pour un retard ; que ses parents voient ce qu'il lit | Réserver le tome suivant depuis son téléphone ; voir ses prêts et la date de retour |
| **Lycéen, élève majeur** | Idem, plus presse, orientation, ressources numériques, mémoire, TPE/grand oral | Mots de passe des ressources affichés sur un post-it | Que la famille d'un majeur reçoive ses relances | Accès aux ressources numériques depuis chez lui, sans mot de passe |
| **Élève à besoins particuliers** (DYS, déficience visuelle, allophone, ULIS) | Lit en version adaptée, livre audio, gros caractères, FALC | Ne sait pas ce qui existe en version adaptée | Être identifié comme « différent » à la vue de ses camarades | Filtre « version adaptée », lecture audio, police adaptée sur le portail |
| **AESH** | Accompagne un élève au CDI, cherche des ressources adaptées | Pas d'accès au catalogue | — | Voir les versions adaptées et réserver pour l'élève |
| **Professeur** | Envoie ses élèves, réserve le CDI pour une séance, emprunte une série de classe, demande des achats | Réservation par mail ; série de 30 romans notée sur papier | Que la série revienne incomplète sans savoir qui l'a | Réserver la salle et la série en même temps ; voir qui n'a pas rendu **son** exemplaire de série |
| **Professeur principal** | Relance les retards de fin d'année, fait le bilan avant le conseil | Liste papier avec titres ; relance au milieu de la classe | Être le « gendarme des livres » | Un bilan « 3 élèves ont un document non rendu », message prêt |
| **CPE, vie scolaire** | Envoie des élèves de permanence au CDI ; tient la présence | Billets papier ; deux appels contradictoires (permanence et CDI) | Élève « disparu » entre la permanence et le CDI | Présence au CDI visible dans l'appel, élève en route signalé s'il n'arrive pas |
| **Direction** | Fixe les horaires, le budget documentaire, la politique documentaire ; préside le CA qui vote le barème | Pas d'indicateur ; budget voté sans bilan | Un CDI fermé sans responsable, des élèves sans surveillance | Indicateurs anonymes de fréquentation et de prêt ; rapport annuel automatique |
| **Intendance, comptabilité (public) ; comptable de l'OGEC (privé)** | Paie les commandes, les abonnements ; facture les pertes | Factures sans bon de commande ; abonnements renouvelés tacitement | Facturer une perte sans délibération | Commandes rattachées au budget, pertes en facture du 24 avec la référence du barème |
| **Secrétariat** | Gère les départs | Ne sait pas qu'un élève qui part a des livres | — | Bilan des prêts proposé au départ, sans blocage |
| **Familles** (dont parents séparés) | Reçoivent les relances, paient une perte | Relances au mauvais parent ; montant inconnu | Payer un livre déjà rendu | Relance au parent qui suit l'élève, montant, preuve de retour |
| **Parents bénévoles du CDI** (fréquent dans le privé [H]) | Aident à l'équipement, à la mise en rayon, au comptoir | — | — | Droits limités au comptoir, sans accès aux fiches |
| **Psychologue, infirmière** | Orientent vers des lectures, documents d'orientation au CDI | — | — | Fonds orientation à jour (socle 37) |
| **Internat** | CDI ou salle d'étude ouverte le soir [H] | — | — | Prêt possible par le surveillant d'internat |
| **Personnels** | Empruntent pour eux ou pour leurs cours | — | — | Prêt long, sans relance agressive |
| **Délégué à la protection des données** | Contrôle la conservation et la minimisation | Historiques de lecture conservés depuis 15 ans dans le SIGB [H] | Profilage de mineurs | Déliement automatique, registre clair |

---

## 4. L'année du CDI vue du terrain [H, à confirmer avec Florian]

| Période | Ce qui se passe | Ce que ScolIA fait |
|---|---|---|
| **Juin–juillet (N-1)** | Retour général des prêts et des manuels ; récolement ; désherbage ; commandes de rentrée | Bilan par classe aux PP ; campagne de retour des manuels avec état ; récolement au scan ; propositions de désherbage ; paniers de commande |
| **Fin août** | Équipement des nouveautés ; préparation des manuels par classe | Notices par ISBN, étiquettes ; lots de manuels préparés à partir des **effectifs prévisionnels** (socles 3, 5) |
| **Septembre** | Distribution des manuels ; accueil des 6ᵉ et des secondes au CDI ; horaires publiés ; responsables d'affectation GAR | Campagne de distribution par classe (scan ou attribution en lot) ; séances d'accueil dans l'EDT (10) ; diagnostic de l'alimentation GAR avant l'ouverture du médiacentre |
| **Octobre–mai** | Prêts, réservations, séances EMI, projets (semaine de la presse en mars, prix littéraires, Printemps des poètes [H]), clubs | Relances, réservations, séances, sélections, actualités (17), clubs (43) |
| **Toute l'année** | Arrivées et départs d'élèves | Emprunteur créé à l'arrivée (2), bilan au départ |
| **Fin d'année** | Rapport d'activité, budget de l'année suivante | Rapport généré (indicateurs anonymes), proposition de budget, barème pour le CA |

---

## 5. Ce qui est réglementaire (vérifié le 2026-11-11)

Recherche menée le 2026-11-11 par un sous-agent, sur sources publiques. **Légifrance, education.gouv.fr (BO) et eduscol ont renvoyé 403 ou un contrôle anti-robot au client automatisé** : aucun article de code n'a pu être lu directement. D'où les marques [V-copie] (copie académique intégrale) et [V-indirect] (texte cité par une juridiction, un contrat ou une réponse ministérielle). Les [NV] sont interdits de code tant qu'ils ne sont pas vérifiés ; une relecture sur Légifrance avant les lots BIB7, BIB8 et BIB13 est demandée (Q-CDI7).

### 5.1 Missions du professeur documentaliste
| Règle | Source | Statut | Conséquence |
|---|---|---|---|
| Trois axes : enseignant et maître d'œuvre de la culture de l'information et des médias ; maître d'œuvre de l'organisation des ressources pédagogiques et documentaires et de leur mise à disposition ; acteur de l'ouverture de l'établissement | Circulaire **2017-051** du 28-3-2017, NOR MENE1708402C, BO n° 13 du 30-3-2017 ; copie : espacedocweb.enseigne.ac-lyon.fr/spip/IMG/pdf/circulaire_mission_prof_doc2017-2.pdf | [V-copie] | modèle de rôle « Documentaliste » propre (§21.1) |
| « Sous l'autorité du chef d'établissement », responsable du CDI et du fonds ; élabore une **politique documentaire validée par le conseil d'administration**, inscrite au volet pédagogique du projet d'établissement | idem, axe 2 | [V-copie] | document « politique documentaire » versionné, avec la délibération du 46 (CDI-271, §30) |
| CDI « ouvert à tous les membres de la communauté éducative » ; usage articulé avec la vie scolaire (études, internat) ; le documentaliste peut organiser un espace CDI dans l'ENT | idem | [V-copie] | §12 (lieu), §14 (portail) |
| Service : **30 h** d'information et documentation + **6 h** de relations avec l'extérieur ; heures d'enseignement, avec l'accord de l'intéressé, comptées **pour 2 h** chacune | décret **2014-940**, art. 2, III ; circulaire 2015-057 du 29-4-2015 (copie académie de Bordeaux) | [V-copie] (circulaires), décret [NV] | les séances EMI du documentaliste sont signalées au 6 et au 10 avec la pondération (§30, R-CDI35) |
| Privé sous contrat : documentaliste sous contrat « dans les mêmes conditions que les maîtres » | devenirenseignant.gouv.fr | [V-indirect] | même rôle dans les deux secteurs |

### 5.2 Manuels scolaires
| Règle | Source | Statut | Conséquence |
|---|---|---|---|
| L'État prend en charge la fourniture des manuels dans les **collèges** ; un cahier d'exercices individuel n'est pas un manuel | C. éduc. **L211-8 5°**, **D211-15** ; CE 27-4-2012 n° 352844 | [V-indirect] | nature `manuel` (prêté) distincte de `consommable` (acheté par la famille, hors 45) |
| Caution et responsabilité financière des familles admises en cas de perte ou d'usure anormale, montant modulé par l'ancienneté ; tarif voté en CA en pratique | réponses ministérielles AN QE 3765 (10ᵉ lég.) et QE 38590 (11ᵉ lég.) | [V-indirect], anciennes | barème lié à une délibération (R-CDI17) ; décote par ancienneté (§10.6) |
| Lycée : pas de gratuité nationale, aides des régions | idem | [V-indirect] | paramètre « financeur des manuels » (établissement, région, famille) |
| Collège privé sous contrat : prise en charge des manuels | — | [NV] | Q-CDI8 |

### 5.3 Droit d'auteur et droit de prêt
| Règle | Source | Statut | Conséquence |
|---|---|---|---|
| Exception pédagogique ; protocole de 2016 (extraits limités, manuels et partitions exclus) ; avenant du 26-12-2019 : reconduction tacite dans la limite de 48 mois | CPI L122-5 3° e ; protocole du 22-7-2016 (BO n° 35 du 29-9-2016) ; avenant NOR MENE2000032X (BO n° 7 du 13-2-2020) | [V-indirect] ; version après 2023 [NV] | aucun quota codé ; rappel du droit d'auteur dans les espaces de dépôt (sélections, ressources, travaux d'élèves) |
| Reprographie : accord avec le CFC 2023-2025 | BOEN n° 44 du 23-11-2023 | [V-indirect] ; renouvellement [NV] | hors périmètre |
| Droit de prêt : la première part (forfait par inscrit) est calculée **à l'exception des bibliothèques scolaires** ; la seconde (6 % du prix public) vise les bibliothèques accueillant du public, versée par les fournisseurs | CPI **L133-3** (loi 2003-517) ; décret 2004-921 | [V-indirect] | aucune statistique d'inscrits à déclarer ; classement des achats d'un CDI au regard des 6 % [NV] ; **plafond légal de remise** sur les achats de livres [NV] : le panier ne calcule pas de remise « légale », il reprend celle du devis |

### 5.4 GAR et médiacentre
| Règle | Source | Statut | Conséquence |
|---|---|---|---|
| Traitement « GAR » créé par arrêté publié au JORF n° 0299 du 23-12-2017 ; avis CNIL délibération 2017-253 du 21-9-2017 ; date de l'arrêté donnée « 18 décembre » par le site et « 18 novembre » par le contrat (contradiction) | gar.education.fr/cadre-juridique/ ; Contrat-GAR_v2026.pdf | [V] sites GAR ; texte de l'arrêté [NV] | cité avec réserve |
| Ouvert aux EPLE et aux **établissements privés sous contrat** ; généralisation visée « d'ici la rentrée 2027 » | Contrat d'adhésion GAR v2026, §2.4 | [V] | le GAR est attendu partout, privé compris |
| Un ENT compatible : export quotidien des identités ; accès à la **console d'affectation** pour les « responsables affectations » ; **médiacentre** conforme au SDET, alimenté par un webservice du GAR ; accès par le module d'accès du GAR | idem | [V] | §13 : lien vers la console pour les responsables, médiacentre servi par le 9, diagnostic |
| Attributs en 4 catégories ; **identifiant opaque propre à chaque ressource** ; aucune donnée nominative remise directement aux éditeurs ; identifiant conservé lors d'un changement de prestataire | Contrat GAR v2026, annexe §1.3–1.5 | [V] | R-CDI14 bis (§30) : aucune donnée d'élève ne va à un éditeur hors du GAR ; continuité des identifiants à la reprise (9) |
| Conservation : données scolaires pour la durée de la ressource ; journaux 12 mois | gar.education.fr/mentions-informatives-rgpd/ | [V] | aligné (§21.3) |
| Les éditeurs livrent leurs ressources **exclusivement via le GAR**, y compris gratuites | Contrat GAR v2026 | [V] | les « ressources libres » du 45 sont les **sites publics hors GAR** (portails publics, sites d'institutions), jamais un contournement d'une ressource référencée au GAR |
| Éduthèque fermé le 30-6-2022, ressources reprises dans Lumni Enseignement via le GAR ; fin du marché BRNE « langues » à l'été 2024 | pages académiques | [V-indirect] | aucun connecteur Éduthèque |
| Cadre de référence du 7-9-2026 (JORF du 23-9-2026) : ENT, **suspension des notifications le soir et le week-end** ; aucune mention trouvée du GAR ou du CDI | Banque des Territoires (compte rendu) | [V-indirect] | R-CDI8 étendue : aucune relance le soir, le week-end ni pendant les vacances (§30) |

### 5.5 SDET
Médiacentre **[UTI-CPP-MED]** : ressources du GAR par catégories (référence, manuels numériques, presse, orientation…), recherche, titre, éditeur, vignette, identifiant ARK, favoris, un onglet par établissement. **Espace des ressources documentaires [UTI-CPP-ERD]** : affichage, recherche, filtre et tri des ressources du centre documentaire, « réservation et consultation d'une ressource du centre documentaire », animation par le documentaliste. Aucun produit nommé. Source : SDET, document principal, version AAC 2024 (CC BY-SA 3.0), §3.2.5.2-3 — eduscol.education.gouv.fr/sites/default/files/document/sdetdocument-principalversionaac2024vdpdf-105189.pdf [V] ; version 2025 [NV]. **Conséquence** : le §14 (portail, réservation) et le §13 (médiacentre) couvrent ces deux services ; favoris = listes personnelles (CDI-221) ; onglet par établissement = cloisonnement (R-CDI25).

### 5.6 Données personnelles des prêts
| Règle | Source | Statut | Conséquence |
|---|---|---|---|
| Ancienne norme simplifiée n° 9 (délibération 99-27 du 22-4-1999, prêts de livres) : historique **4 mois après le retour** ; les normes n'ont plus de valeur juridique depuis le RGPD mais « peuvent inspirer » ; e-sidoc/BCDI appliquent encore 4 mois | fiche registre RGPD e-sidoc (Canopé, copie ac-lyon) ; page CNIL | [V-copie], [V] | **R-CDI2 retient 30 jours**, plus protecteur que 4 mois ; le DPO peut porter jusqu'à 4 mois (S-4502), jamais au-delà |
| Code de déontologie de l'ABF (révisé 2020) : confidentialité des usages ; Manifeste IFLA-UNESCO bibliothèque scolaire 2025 : liberté intellectuelle | abf.asso.fr ; enssib.fr | [V-indirect], [V] | fonde R-CDI1 (choix de conception, pas une obligation légale) |
| Aucun référentiel CNIL spécifique aux bibliothèques après 2018 trouvé | — | [NV] | Q-CDI6 au DPO |

### 5.7 Notices, formats, outils
| Point | Source | Statut | Conséquence |
|---|---|---|---|
| Métadonnées de la BnF sous **Licence ouverte Etalab** depuis 2014 (source et date à mentionner), **sauf autorités Dewey** ; SRU, Z39.50, OAI ; SRU privilégié | bnf.fr/fr/conditions-de-reutilisations-des-donnees-de-la-bnf | [V-indirect] | import par SRU, source et date tracées (`origine`, `importee_le`) ; aucune autorité Dewey redistribuée |
| BCDI (Canopé, propriétaire, payant), alimente e-sidoc ; aucune fin de vie annoncée trouvée | comparatif ac-normandie | [V-indirect] ; statut 2026 [NV] | import depuis BCDI/e-sidoc prioritaire (§26) |
| PMB : licence CeCILL 2.0 ou GPLv2 selon les sources | — | [NV], contradiction | **aucun code repris** ; interopérabilité par formats seulement |
| Motbis (Canopé), Dewey abrégée (OCLC) : licences introuvables | — | [NV] | **aucun thésaurus ni classification embarqué** ; vocabulaires importables par l'établissement (Q-CDI3) |

### 5.8 Lieu, internet, EMI
| Règle | Source | Statut | Conséquence |
|---|---|---|---|
| Usage de l'internet et protection des mineurs : filtrage et charte | circulaire **2004-035** du 18-2-2004, BO n° 9 du 26-2-2004 | [V-indirect] | le filtrage relève du réseau, pas de l'ENT ; la charte est au règlement (§12.4) |
| Surveillance des élèves : régime général ; aucune règle propre au CDI sans documentaliste | circulaire 96-248 du 25-10-1996 modifiée par 2004-054 | [V-indirect] ; règle propre [NV] | R-CDI11 (aucun accueil libre sans responsable présent) est une **recommandation** |
| EMI : circulaire du 24-1-2022 ; priorité de la circulaire de rentrée 2025 ; projet de programme EMI cycle 4 (CSP, 22-5-2025) sans arrêté trouvé ; attestation Pix de sensibilisation en 6ᵉ depuis la rentrée 2024 | pages académiques, CSP | [V-indirect] | §15 : séances et compétences liées au 10, 20, 22 ; pas d'obligation logicielle ferme |

### 5.9 Non vérifié (récapitulatif)
Texte consolidé du décret 2014-940 ; L211-8 et D211-15 en vigueur ; facturation des manuels perdus (réponse récente) ; protocole d'exception pédagogique après 2023 ; plafond de remise sur les livres ; arrêté GAR (date, NOR, attributs) ; NOR et contenu de l'arrêté du 7-9-2026 ; SDET 2025 ; référentiel CNIL bibliothèques ; licences PMB, Motbis, Dewey ; règle d'accueil au CDI sans documentaliste ; manuels du privé ; exception handicap (CPI L122-5 7°, organismes habilités, plateforme de la BnF) non recherchée ; sortie d'inventaire d'un bien d'EPLE.

---

## 6. Branchements sur le cœur et frontières

### 6.1 Qui possède quoi

| Objet | Propriétaire | Rôle du 45 |
|---|---|---|
| Élève, identité, scolarisation, sortie | 2 | lit ; consomme `eleve.sortie`, `eleve.fusionne`, `eleve.reintegre` |
| Responsables, liens, droits, adresse protégée | 4 | lit les destinataires des relances par la règle du socle 4 (droit `suivi_scolaire`) |
| Classes, cours, groupes, années, calendrier, salles du CDI | 5 | lit ; le CDI est une **salle** typée `cdi` du socle 5 |
| Agents, fonctions (`documentaliste`), engagements, temps partagé | 6 | lit ; le responsable présent du lieu est un agent |
| Pièces (couvertures, PDF d'exposition, factures) | 7 | dépose par le kit fichiers (E5) |
| Envois (relances, réservation disponible) | 8 | déclare ses natures (§23) |
| GAR, médiacentre, identifiants de service, alimentation | 9 | lit l'état, ouvre le diagnostic ; **n'affecte rien** |
| Séances (accueil, EMI, co-intervention) | 10 | crée des séances par le port du 10 (nature `cdi`) ; le documentaliste est intervenant |
| Appel, localisation pendant une heure sans cours | 12 | écrit la **présence au CDI** par le port de localisation du 12 (R-CDI10) |
| Liste des présents en alerte | 14 | expose les présents au CDI |
| Actualités du CDI | 17 | publie par le 17 |
| Réservation des espaces du CDI par les professeurs | **19** (moteur de lieux) | le CDI déclare ses espaces réservables et ses règles (créneaux d'ouverture) |
| Cahier de textes d'une séance EMI | 20 | lien |
| Compétences EMI, Pix | 22 | lien ; aucune évaluation tenue au 45 |
| Facture de perte ou de dégradation | 24 | demande une facturation ponctuelle avec référence au barème |
| Orientation (fonds ONISEP, documents) | 37 | le fonds est au 45, le conseil au 37 |
| Clubs de lecture, ateliers | 43 | lit le club, le 45 fournit les sélections |
| Matériel prêté (ordinateurs, tablettes, calculatrices) | **44** | le 44 **réutilise** le moteur d'exemplaires et de prêt du 45 (Q-CDI2) |
| Délibération du barème de perte, du règlement du CDI (public) | **46** | lit la délibération exécutoire avant d'appliquer un barème |
| Budget documentaire, commandes, factures fournisseurs | 48 | rattache commandes et abonnements à une ligne budgétaire |
| Indicateurs | 47 | expose des agrégats anonymes |

### 6.2 Ports consommés

| Port | Socle | Usage |
|---|---|---|
| `eleves::actifs(etab, date)`, `eleves::classe_a_date` | 2, 5 | emprunteurs, campagnes de manuels |
| `liens::destinataires_suivi(eleve, nature)` | 4 | relances (parents séparés, majeur, interdictions) |
| `structure::salles(type='cdi')`, `calendrier::ouvertures` | 5 | lieu, horaires |
| `personnels::presents(lieu, plage)` | 6 | responsable présent pour ouvrir |
| `gar::etat_alimentation(etab)`, `gar::ressources_affectees(personne)` [NV : API du GAR] | 9 | portail, diagnostic |
| `edt::creer_seance(nature='cdi', ...)` | 10 | séances |
| `assiduite::localiser(eleve, plage, lieu)` | 12 | présence au CDI |
| `agenda::lieux::declarer_espace`, `agenda::lieux::reserver` | 19 | espaces réservables |
| `facturation::ponctuelle(payeurs, montant, motif, ref)` | 24 | perte |
| `instances::deliberation_executoire(objet)` | 46 | barème, règlement |
| `budget::ligne(etab, exercice, code)` | 48 | commandes |

### 6.3 Ports exposés

| Port | Pour | Contenu |
|---|---|---|
| `cdi::prets_en_cours(eleve)` (nombre, échéances, **sans titre** sauf appelant documentaliste) | 2 (départ), 29 (tuile), 23 (rien, voir R-CDI7) | |
| `cdi::presents(lieu, instant)` | 12, 14 | liste des présents au CDI |
| `exemplaires::*`, `prets::*` (moteur) | 44 | exemplaires sans notice, prêt, retour, perte |
| `cdi::indicateurs(periode)` | 47 | agrégats anonymes (seuil k ≥ 5, R-CDI6) |
| `cdi::selection(id)` | 17, 20, 43 | sélection publiée (titres, sans emprunteurs) |

### 6.4 Répercussions automatiques

| Fait du cœur | Effet au 45 |
|---|---|
| `eleve.sortie` | bilan des prêts : tâche au documentaliste et au secrétariat (« 2 documents non rendus ») ; relance finale ; **aucun blocage** (R-CDI7) ; à J+60 sans retour, proposition de perte |
| `eleve.fusionne` | prêts et réservations rattachés à la fiche conservée |
| `eleve.reintegre` | emprunteur rouvert |
| Changement de classe | statistiques recalculées sur la classe à la date du prêt ; aucune relance au mauvais PP |
| `liens.*` (suspension, garde exclusive) | destinataires recalculés à chaque envoi (4) |
| `eleve.majeur` | relances à l'élève seul, sauf accord de l'élève (socle 4) |
| `eleve.decede` | arrêt immédiat de toutes les relances ; prêts clos « sans suite » ; aucune facture |
| Agent qui part (6) | prêts de personnel : relance, puis perte ; séries de classe à rendre |
| Fermeture de l'établissement (5) | CDI fermé ; réservations d'espaces annulées par le 19 ; échéances de prêt **reportées** (R-CDI8) |

---

## 7. Concepts et vocabulaire [R]

| Terme | Définition | Ce n'est pas |
|---|---|---|
| **Notice** | Description d'une œuvre ou d'un titre (livre, BD, revue, DVD, manuel, ressource numérique, exposition, jeu) | un exemplaire |
| **Exemplaire** | Objet physique prêtable, avec code-barres, cote, localisation, état | une notice ; pour le 44, un exemplaire sans notice |
| **Périodique** | Notice d'un titre de presse ; **numéro** = notice fille datée ; **abonnement** = engagement d'achat | |
| **Emprunteur** | Personne (élève, personnel, intervenant autorisé) avec une **catégorie** (règles) | un compte |
| **Prêt** | Fait daté liant un exemplaire et un emprunteur jusqu'au retour | une réservation |
| **Série de classe** | Prêt d'un lot d'exemplaires d'un même titre à un professeur, puis **attribution** facultative de chaque exemplaire à un élève | 30 prêts individuels non liés |
| **Campagne de manuels** | Distribution, puis retour, d'un lot de manuels par classe, avec état constaté aux deux bouts | un prêt ordinaire |
| **Présence au CDI** | Localisation d'un élève au CDI pendant une plage | un appel de cours |
| **Créneau d'accueil** | Plage d'ouverture avec jauge, ouverte aux élèves de permanence | une séance |
| **Séance CDI** | Séance de l'emploi du temps au CDI (EMI, co-intervention, accueil) | un créneau d'accueil |
| **Sélection** | Liste ordonnée de notices publiée (thème, niveau, prix littéraire) | un panier de commande |
| **Récolement** | Contrôle physique du fonds (scan des rayons) | un inventaire comptable |
| **Désherbage** | Retrait raisonné de documents du fonds, avec motif et devenir | une suppression |
| **Barème de perte** | Règle de remplacement ou de remboursement, **délibérée** (public) ou fixée par l'organisme (privé) | une amende de retard |

---

## 8. Catalogue et notices

### 8.1 Natures de notice [R]
`livre`, `bd`, `manga`, `album`, `livre_audio`, `gros_caracteres`, `falc`, `periodique`, `numero`, `article` (dépouillement, option), `dvd`, `cd`, `ressource_numerique`, `exposition`, `jeu`, `kit`, `manuel`, `document_orientation`, `travail_eleve` (exposé, mémoire, avec accord). La nature est un **code** d'une table de référence globale (E12, `globale_reference`), pas un texte libre.

### 8.2 Contenu d'une notice [R]
Titre, sous-titre, titre de série et rang (tome), auteurs avec **rôle** (auteur, illustrateur, traducteur, scénariste…) en table d'autorités, éditeur, collection, date, ISBN-10/13 ou ISSN, langue, nombre de pages, public (niveau de lecture : `cycle3`, `cycle4`, `lycee`, `adulte`), genre (code), sujets (mots-clés d'un thésaurus, table d'autorités), résumé, couverture (objet E5), **accessibilité** (version adaptée : gros caractères, audio, FALC, DAISY [NV]), lien vers une ressource en ligne, avertissement de contenu (sensible pour l'âge) [R], origine de la notice (saisie, import BnF, import SIGB, IA proposée et validée).

### 8.3 Sources de notices [R, licences à vérifier §5]
1. **Scan de l'ISBN** (douchette ou appareil photo du mobile) → recherche dans les sources ouvertes déclarées (BnF par SRU sous licence ouverte [NV]) → notice **proposée**, validée par le documentaliste.
2. **Import d'un SIGB** (UNIMARC, MARC21, CSV) par le moteur d'échanges du 9 : lot, prévisualisation, rejets ligne à ligne, annulation (§26).
3. **Saisie** à la main, avec contrôle de doublon (ISBN, titre normalisé + auteur).
4. **Proposition de l'IA** depuis une photo de la couverture et de la quatrième (§18), toujours validée.

### 8.4 Indexation [R]
Cote (Dewey simplifiée ou cote maison : la **cote est un texte libre** de l'établissement, D-022 : paramétrage libre des données), sujets (thésaurus de l'établissement, avec un **jeu de départ** fourni si sa licence le permet [NV, Q-CDI3]), genre, niveau. Aucune classification sous licence restrictive n'est livrée par défaut (§5).

### 8.5 Périodiques [R]
Abonnement (titre, fournisseur, période, coût, renouvellement **explicite**, jamais tacite dans le logiciel), périodicité, bulletinage numéro par numéro (reçu, manquant, réclamé), alerte « numéro attendu non reçu » à J+10 de la date prévue, durée de conservation des numéros (ex. 2 ans, puis désherbage proposé), dépouillement d'articles (option).

---

## 9. Exemplaires, inventaire, récolement, désherbage

- **Exemplaire** : code-barres (unique par établissement, compteur par établissement, E3), cote, localisation (CDI, réserve, salle de classe, BCD annexe, internat, salle des professeurs, **site** du socle 5), état (`neuf`, `bon`, `use`, `abime`, `hors_service`), statut (`disponible`, `en_pret`, `reserve`, `exclu_pret` (consultation sur place), `en_reparation`, `perdu`, `retire`), date et mode d'acquisition (`achat`, `don`, `specimen`, `abonnement`, `transfert`), prix, fournisseur, commande (48).
- **Étiquettes** : planche d'étiquettes code-barres et cotes (PDF du socle 7, trames D-023).
- **Récolement** : campagne (rayon ou fonds entier) ; scan au mobile ; liste des manquants, des « en prêt mais présents » (retour oublié), des « mal rangés » (localisation) ; clôture avec décisions (rechercher, déclarer perdu) ; aucun statut changé sans décision (R-CDI12).
- **Désherbage** : critères proposés (âge, état, dernier prêt, obsolescence, doublon) ; liste proposée par le serveur ou par l'IA, **décision humaine document par document** ; devenir (`pilon`, `don`, `vente`, `transfert`, `archive`) ; motif ; sortie d'inventaire datée ; la notice reste si d'autres exemplaires existent. Public : la sortie d'un bien de l'inventaire peut demander une décision de l'ordonnateur [NV, §5] — l'action le signale.
- **Inventaire** : registre exportable (exemplaires entrés et sortis par exercice), avec valeur d'acquisition.

---

## 10. Prêts, retours, réservations

### 10.1 Règles de prêt [R]
Par **catégorie d'emprunteur** (`eleve_college`, `eleve_lycee`, `eleve_interne`, `personnel`, `intervenant`), et par **nature** : nombre maximum, durée, prolongations, réservation permise. Valeurs par défaut prêtes (D-023) : élève 3 documents, 3 semaines, 1 prolongation ; personnel 10, 8 semaines ; DVD et revues récentes 1 semaine ; usuels et dernier numéro en consultation sur place. L'établissement ajuste les **valeurs**, pas la mécanique.

### 10.2 Prêt
- **Au comptoir** : scan de la **carte d'emprunteur** (jeton opaque imprimé en code-barres, jamais l'INE ni un numéro d'élève lisible, R-CDI9) ou recherche par nom au comptoir (documentaliste, AED du CDI) ; scan de l'exemplaire ; contrôles (statut, maximum, retard bloquant **le prêt**, jamais autre chose, R-CDI7) ; dérogation motivée par le documentaliste.
- **Sur le mobile de l'élève** [R, lot BIB9] : l'élève scanne l'exemplaire dans le CDI pendant un créneau ouvert ; le prêt est **en attente** jusqu'à validation par le responsable présent (anti-fraude, CCD-17).
- **Série de classe** : prêt du lot au professeur ; attribution facultative élève par élève ; au retour, contrôle exemplaire par exemplaire ; un exemplaire manquant est rattaché à l'élève attributaire s'il existe, sinon au professeur.
- **Hors les murs** : prêt par l'internat (surveillant habilité, catégorie restreinte) ou à une annexe.

### 10.3 Retour
Scan, n'importe où dans l'établissement (boîte de retour) ; état constaté ; si réservé, mise de côté et avis à l'emprunteur suivant ; un retour sans prêt connu ouvre une anomalie, jamais une erreur bloquante ; **le retour efface toute relance en attente** dans la même transaction.

### 10.4 Prolongation et réservation
Prolongation par l'élève (portail) ou au comptoir, refusée si réservé par quelqu'un d'autre. Réservation sur une notice (premier exemplaire disponible) ; file d'attente ordonnée ; mise de côté 7 jours ; annulation automatique à l'expiration ; l'élève voit son rang, pas les autres réservataires.

### 10.5 Relances [R, natures au §23]
| Étape | Quand | Destinataire | Contenu |
|---|---|---|---|
| Rappel | J-2 avant l'échéance | emprunteur | titre, date, prolonger en un geste |
| Retard 1 | J+7 | emprunteur | titre, « rapporte-le ou prolonge » |
| Retard 2 | J+21 | emprunteur **et** responsables ayant le suivi (élève mineur) | **sans titre** pour la famille : « 1 document du CDI à rendre depuis le … » |
| Retard 3 | J+45 | idem + information du PP (nombre, sans titre) | proposition de rendez-vous ou de perte |
| Perte proposée | J+60, ou départ | documentaliste décide | facture du 24 selon barème, **avec** le titre et le prix pour la famille (R-CDI3) |
Les vacances suspendent le décompte (R-CDI8). Un élève majeur reçoit seul ses relances (4). Aucune relance n'est lue à voix haute ni affichée à la classe : aucune liste publique de retardataires n'existe dans le logiciel (R-CDI4).

### 10.6 Perte et dégradation
Constat (documentaliste) → choix : remplacement à l'identique par la famille (délai), remboursement selon **barème délibéré** (valeur à neuf, décote par ancienneté [R, valeurs proposées au CA]), ou **remise gracieuse** motivée (situation sociale, décision de la direction ; lien au fonds social du 26 sans en révéler la nature) ; facture ponctuelle au 24 ; retrouvé après facture → **avoir** (D-027 E) et exemplaire remis en circulation.

---

## 11. Manuels scolaires (campagnes)

- **Titres** de manuels (notice `manuel`) rattachés à un **enseignement** et à un **niveau** du socle 5 (et option, LV2, groupe) : la liste des manuels d'une classe est **calculée** depuis les enseignements suivis par chaque élève (un élève d'allemand LV2 reçoit le manuel d'allemand, pas celui d'espagnol, D-022).
- **Préparation** : nombre d'exemplaires nécessaires par classe à partir des effectifs prévisionnels ; manque affiché (à commander).
- **Distribution** : par classe, en salle ; scan de chaque exemplaire remis, ou **attribution en lot** (le professeur principal coche, puis les codes sont associés au scan) ; état constaté à la remise (photo facultative, R-CDI13) ; accusé de réception de la famille par le socle 18 (option, lot BIB8).
- **En cours d'année** : élève arrivé → kit à lui remettre ; élève qui change de groupe → échange de manuel ; départ → retour demandé.
- **Retour de juin** : par classe ; état constaté ; écart d'état significatif → dégradation proposée selon barème ; non rendu → perte proposée.
- **Lycée et région** : si les manuels sont fournis par la région ou en numérique (via le GAR), la campagne se limite au suivi des exemplaires papier éventuels [NV, §5].
- **Privé** : manuels achetés par l'organisme ou par les familles (location, bourse aux livres de l'association de parents [H]) ; le 45 gère le cas « manuels propriété de l'établissement », pas la vente.

---

## 12. Le CDI comme lieu

### 12.1 Ouverture [R]
Horaires par jour de la semaine (calendrier du 5), fermetures ponctuelles (absence du documentaliste, réunion, séance réservée), **jauge** (places), **responsable présent** (agent habilité : documentaliste, AED affecté, professeur qui réserve). Sans responsable déclaré présent, le CDI est « fermé à l'accueil libre » : aucun créneau n'est proposé aux élèves de permanence (R-CDI11). Absence du documentaliste saisie au socle 15 → fermeture proposée ou AED désigné.

### 12.2 Venue des élèves de permanence
- La vie scolaire **envoie** un élève (ou un groupe) au CDI pour une plage : l'élève est **localisé** « en route vers le CDI » (12) ; le CDI voit l'arrivée attendue.
- Ou l'élève **réserve** une place sur un créneau d'accueil (portail, mobile), dans la jauge, sur une plage où il n'a pas cours (le serveur vérifie l'EDT de l'élève, 10).
- Arrivée : scan de la carte ou pointage dans la liste ; **non-arrivé à H+10 min** → alerte à la vie scolaire (R-CDI10).
- Départ avant la fin : pointage ; l'élève redevient « en permanence » ou « sorti » selon son régime (14).
- La présence est une **localisation**, pas une absence : elle n'entre jamais dans les compteurs d'assiduité.

### 12.3 Séances et réservations
Séance EMI ou co-intervention : séance du 10 avec le documentaliste comme intervenant et le CDI comme lieu ; le créneau d'accueil libre correspondant est fermé ou réduit automatiquement. Un professeur réserve un **espace** du CDI (îlot, salle de travail, postes) par le moteur du 19 ; règles : préavis, durée maximale, priorité aux séances (19 R-AG21).

### 12.4 Règlement du CDI
Document du socle 7, annexé au règlement intérieur (délibéré au CA dans le public, §5) ; affiché sur le portail ; aucune « acceptation » à cliquer par l'élève (le règlement intérieur s'applique).

---

## 13. Ressources numériques et GAR

- **Médiacentre** : liste des ressources **affectées** à la personne par le GAR (lecture par le 9) + ressources **libres** déclarées par le CDI (portails publics, sites, sitothèque) dans un portail unique, avec un marqueur « via le GAR » ou « accès libre ».
- **Diagnostic** (ITP-87 du 9, écran au 45) : pour une personne et une ressource : affectation présente ? personne dans l'alimentation ? groupe ou classe à jour ? alimentation en retard ? ressource expirée ? Réponse en mots simples et action proposée (« ajoutez la classe 4ᵉB à l'affectation dans la console du GAR »).
- **Responsables d'affectation** : désignés au 9 (`ent_gar_responsable_affectation`) ; le 45 affiche qui ils sont.
- **Abonnements numériques hors GAR** : jamais d'identifiant ou de mot de passe partagé stocké dans ScolIA (R-CDI14) ; si un éditeur impose un code d'établissement, il est stocké **chiffré** (champs chiffrés existants, E11), affiché aux seuls personnels habilités, et signalé comme à remplacer par un accès GAR.
- **Ressources déposées par le CDI** (sélections de liens, guides méthodologiques) : documents du 7, publiés au portail.
- Dans le public derrière l'ENT d'une collectivité (`PERSONAS_09.md` M-428), le médiacentre reste à l'ENT : le 45 n'affiche alors que le catalogue, les prêts et un lien vers le médiacentre de l'ENT (mode « service raccordé »).

---

## 14. Portail documentaire

- **Recherche** tolérante (accents, fautes, pluriels ; `pg_trgm` et `unaccent` déjà présents, migration 290 [F]) sur titre, auteur, sujet, série, ISBN ; facettes : nature, niveau, genre, disponibilité, version adaptée, langue ; tri pertinence ou nouveauté.
- **Fiche** : notice, couverture, disponibilité par localisation (« 2 au CDI, 1 en prêt jusqu'au 14 »), réserver, ajouter à une liste personnelle, avis.
- **Pour l'élève** : mes prêts et échéances, mes réservations et mon rang, prolonger, mes listes, **mon historique visible de moi seul et supprimable** (opt-in, R-CDI2 bis).
- **Pour le professeur** : sélections pour un cours (publiables au cahier de textes, 20), série de classe, demande d'achat.
- **Pages** : horaires, actualités (17), nouveautés, sélections, prix littéraires, règlement.
- **Avis et coups de cœur d'élèves** : prénom et classe, ou anonyme ; **modérés** avant publication (R-CDI15) ; jamais visibles hors de l'établissement.
- **Catalogue public** (option, S-4505) : le catalogue **sans aucune donnée d'emprunteur** peut être consultable sans connexion (familles, futurs élèves) ; la disponibilité y est agrégée (« disponible / en prêt »), jamais nominative.
- **Accessibilité** (loi 2005-102 art. 47, `19_AGENDA_RDV.md` §6.6) : clavier, lecteur d'écran, police adaptée à la dyslexie au choix de l'utilisateur, synthèse vocale du navigateur, contraste.

---

## 15. Enseignement et projets (EMI)

- Séances (10) avec objectifs, compétences visées (22, référentiel de l'établissement), traces au cahier de textes (20).
- **Parcours de recherche** : fiches méthodes, grilles d'évaluation de sources (documents du 7), sitothèque par projet.
- **Projets** : semaine de la presse et des médias, prix littéraires, concours, expositions (exposition prêtée = notice `exposition` + exemplaire + prêt **entrant** d'un partenaire, avec dates de retour au prêteur) ; inscriptions d'élèves au projet par le 43 ou par liste.
- **Travaux d'élèves** déposés au fonds (exposés, journaux) : **accord** de l'élève (et des responsables si mineur) par le 18, droit à l'image et au nom (R-CDI16).
- Le documentaliste **n'évalue pas** dans le 45 : compétences au 22, notes au 21 s'il enseigne.

---

## 16. Acquisitions et budget

- **Suggestions d'achat** (élèves, professeurs) : file de suggestions, réponse motivée.
- **Paniers** (listes de commande) par fournisseur, avec prix public, remise [NV, §5 : plafond légal de remise aux collectivités], total, ligne budgétaire (48).
- **Commande** : envoi par le documentaliste après **visa** de l'ordonnateur ou de la personne habilitée (public : intendance ; privé : OGEC) selon le circuit du 48 ; suivi (commandé, reçu partiellement, reçu, facturé) ; réception au scan (ISBN → notice proposée → exemplaires créés).
- **Abonnements** : échéances, renouvellement explicite, alerte J-60.
- **Dons** : entrée avec origine `don`, sans prix.
- Le 45 ne paie rien : les factures fournisseurs sont au 48.

---

## 17. Cas limites (catalogue)

### 17.1 Emprunteurs et familles
| N° | Cas | Traitement |
|---|---|---|
| CCD-01 | Élève sans compte (connexion pas encore faite) | emprunte au comptoir avec sa carte ; relances au comptoir et aux responsables |
| CCD-02 | Élève qui part avec des livres | bilan au départ, relance finale, perte proposée à J+60, aucun blocage (R-CDI7) |
| CCD-03 | Élève qui revient l'année suivante | prêts anciens visibles seulement s'ils sont toujours ouverts |
| CCD-04 | Fusion de deux fiches élève | prêts rattachés à la fiche conservée |
| CCD-05 | Parents séparés, un seul a le suivi | relance au seul responsable qui a le droit (4) |
| CCD-06 | Parent dont les droits sont suspendus | jamais destinataire |
| CCD-07 | Élève majeur | relances à lui seul ; perte facturée à lui ou à son payeur selon le 4 |
| CCD-08 | Élève décédé | arrêt de toutes les relances, prêts clos sans suite |
| CCD-09 | Élève protégé (identité protégée, adresse protégée) | jamais dans une liste affichée ; carte sans nom lisible au comptoir ? non : nom affiché au comptoir seulement au personnel habilité |
| CCD-10 | Interne qui emprunte à l'internat le soir | catégorie interne, surveillant habilité, même moteur |
| CCD-11 | Élève de deux sites | localisation de retour libre ; prêt sur le site |
| CCD-12 | Personnel qui part en congé long | échéances suspendues sur demande ; rien d'automatique |
| CCD-13 | Intervenant extérieur (comédien, auteur en résidence) | catégorie `intervenant`, période bornée, aucune donnée hors nom |

### 17.2 Fonds et prêts
| N° | Cas | Traitement |
|---|---|---|
| CCD-14 | Livre rendu dans la boîte un dimanche, scanné lundi | date de retour = date de dépôt déclarée par le scanneur si antérieure (option) ; relance du week-end annulée |
| CCD-15 | Exemplaire scanné au retour mais jamais prêté | anomalie « retour sans prêt », statut inchangé |
| CCD-16 | Deux exemplaires même code-barres (étiquette dupliquée) | refus à la création ; à l'import, file de correction |
| CCD-17 | Élève qui « s'auto-prête » sur mobile hors du CDI | prêt en attente, non validé, expiré à la fin du créneau |
| CCD-18 | Série de 30, 28 rendus, pas d'attribution | 2 manquants rattachés au professeur ; il peut les réattribuer |
| CCD-19 | Livre rendu après facture de perte | avoir et remise en circulation |
| CCD-20 | Livre retrouvé au récolement alors qu'« en prêt » | retour enregistré, relances annulées |
| CCD-21 | Réservation d'un titre dont le seul exemplaire est perdu | réservation suspendue, documentaliste avisé (commande ?) |
| CCD-22 | Fermeture d'urgence (neige, crise) | échéances reportées, relances suspendues (R-CDI8) |
| CCD-23 | Notice importée en double (BCDI + BnF) | proposition de fusion, exemplaires réunis |
| CCD-24 | Manuel d'une option abandonnée en cours d'année | échange proposé au changement de groupe |
| CCD-25 | Manuel rendu dégradé par usage normal | pas de facture (usure) ; seuil d'écart d'état au barème |
| CCD-26 | Élève qui arrive en mars | kit de manuels proposé dès l'inscription (3) |
| CCD-27 | Numéro de revue jamais reçu | réclamation au fournisseur proposée |
| CCD-28 | Abonnement renouvelé par l'éditeur sans commande | alerte, pas d'enregistrement automatique |

### 17.3 Lieu et sécurité
| N° | Cas | Traitement |
|---|---|---|
| CCD-29 | Élève envoyé au CDI qui n'arrive pas | alerte vie scolaire à H+10 |
| CCD-30 | CDI plein (jauge) quand la vie scolaire envoie | refus avec proposition du créneau suivant |
| CCD-31 | Documentaliste absent, AED malade | CDI fermé à l'accueil libre, créneaux annulés, élèves avisés |
| CCD-32 | Alerte incendie, confinement | liste des présents au CDI au 14, en un geste |
| CCD-33 | Élève présent au CDI qui a cours (erreur) | conflit signalé au pointage (EDT du 10) |
| CCD-34 | Séance réservée qui ferme l'accueil d'élèves déjà inscrits | les inscrits sont avisés et replacés (vie scolaire) |
| CCD-35 | Élève exclu temporairement de l'établissement | pas de créneau réservable pendant l'exclusion (13), prêts inchangés |
| CCD-36 | Élève interdit de CDI (mesure du règlement) | mesure du socle 13 lue : pas de réservation, prêt au comptoir à la décision du documentaliste |

### 17.4 Numérique, IA, données
| N° | Cas | Traitement |
|---|---|---|
| CCD-37 | Élève qui ne voit pas une ressource GAR | diagnostic (§13) |
| CCD-38 | Alimentation GAR partie avant les groupes | pré-contrôle du 9 (M-489) ; avertissement au 45 |
| CCD-39 | Éditeur qui impose un mot de passe partagé | stocké chiffré, visible aux personnels habilités seulement (R-CDI14) |
| CCD-40 | Avis d'élève injurieux ou qui nomme un camarade | modération, refus motivé, jamais publié |
| CCD-41 | Résumé importé contenant une consigne pour l'IA | donnée non fiable, aucun effet (§18) |
| CCD-42 | Parent qui demande ce que lit son enfant (mineur) | réponse : prêts **en cours** visibles par le responsable qui a le suivi (dates, nombre ; titres à la demande auprès du documentaliste, qui en juge [R, Q-CDI5]) ; **pas** d'historique |
| CCD-43 | Police ou justice qui demande l'historique d'un élève | historique délié après 30 jours : rien à fournir au-delà ; procédure du 27 pour les prêts en cours |
| CCD-44 | Statistique « classe de 4 élèves » | masquée sous le seuil k = 5 (R-CDI6) |
| CCD-45 | Documentaliste sur deux établissements | deux accès, deux cloisonnements ; aucun prêt d'un établissement visible de l'autre |
| CCD-46 | Reprise d'un SIGB contenant 15 ans d'historique | historique clos **non repris** (R-CDI18) |
| CCD-47 | Contenu sensible pour l'âge (livre pour adultes au CDI de lycée) | avertissement de contenu sur la notice ; aucun blocage automatique par âge (choix pédagogique) |
| CCD-48 | Livre audio ou version adaptée sous exception handicap | prêt réservé aux bénéficiaires déclarés [NV, §5] sans révéler le motif au comptoir |

---

## 18. Intelligence artificielle (contrat IA, `IA_COEUR.md` §5, §6.2)

### 18.1 Ce que l'IA fait [R]
- **Propose une notice** depuis une photo de couverture et de quatrième de couverture, ou complète une notice importée (résumé, sujets, niveau) : N1, validée par le documentaliste.
- **Suggère l'indexation** (sujets du thésaurus de l'établissement, genre, niveau) avec raison.
- **Propose un désherbage** à partir de critères agrégés (âge, état, dernier prêt **sans emprunteur**) : liste N1, décision document par document.
- **Rédige** une sélection thématique, une actualité, un rapport d'activité à partir des **seuls agrégats** : N0.
- **Aide à la recherche documentaire** de l'élève : reformule une question en mots-clés, explique comment évaluer une source, propose des notices **du fonds** et des ressources libres ; elle **n'écrit pas l'exposé** et cite toujours ses sources (N0).
- **Répond** au documentaliste (« combien de retards en 5ᵉ ? », « quels numéros de revue manquent ? ») dans ses droits.
- **Prépare** les relances et réclamations : brouillons N3 envoyés par un humain (la campagne automatique de relances du §10.5 est un automatisme déterministe, pas de l'IA).

### 18.2 Ce que l'IA ne fait pas [R]
- Elle ne lit **jamais** l'historique de lecture d'un élève, ni ses prêts nominatifs, ni ses réservations, ni ses avis signés, ni une donnée de santé ou de handicap (exception handicap, besoins particuliers).
- Elle ne **recommande pas** de lectures fondées sur le profil personnel d'un mineur (R-CDI5) ; une suggestion « si tu as aimé ce livre » se fonde sur **la notice consultée** et les co-emprunts **agrégés et anonymes** (k ≥ 5), jamais sur l'historique de l'élève.
- Elle ne décide ni perte, ni facture, ni remise gracieuse, ni exclusion du prêt, ni désherbage.
- Elle ne génère pas de couverture, d'image d'auteur ou de résumé présenté comme celui de l'éditeur (le résumé proposé est marqué « résumé ScolIA »).

### 18.3 Capacités déclarées (registre `ent-ia`)
| Capacité | Niveau | Contexte transmis | Droits |
|---|---|---|---|
| `cdi.proposer_notice` | N1 | photo(s) de couverture, ISBN, notice partielle | documentaliste |
| `cdi.suggerer_indexation` | N1 | notice, thésaurus de l'établissement | documentaliste |
| `cdi.proposer_desherbage` | N1 | agrégats par exemplaire (âge, état, nombre de prêts sur 3 ans, dernière date) | documentaliste |
| `cdi.rediger_selection` | N0 | notices choisies | documentaliste, professeurs |
| `cdi.rediger_rapport` | N0 | indicateurs anonymes (k ≥ 5) | documentaliste, direction |
| `cdi.aide_recherche` | N0 | question de l'élève, notices du fonds | élèves, tous |
| `cdi.repondre_gestion` | N0 | agrégats et listes autorisées | documentaliste |
| `cdi.brouillon_relance` | N3 | nombre de documents, échéances ; destinataires **calculés par le serveur** | documentaliste |

### 18.4 Cadre
Fournisseur Mistral à l'étude, sous garanties contractuelles (CLAUDE.md) ; aucune clé côté client ; contexte minimisé par capacité ; journal ; tout fonctionne sans IA ; textes importés (résumés, notices d'un SIGB, quatrièmes de couverture) = **données non fiables** encadrées (IA-P6) ; règlement européen sur l'IA : l'aide à la recherche d'un élève n'est ni une évaluation ni une orientation (pas de système à haut risque [NV, étude juridique de D-032]).

---

## 19. Actions métier (CDI-100 à CDI-270)

Colonnes : acteur · droit (D-017, catalogue §21.1) ; règles ; niveau de confirmation (N1–N3, valable aussi sans IA) ; événement ; lot. Acteurs : **DOC** documentaliste, **AEDC** assistant du CDI, **ELV** élève, **PROF** professeur, **PP** professeur principal, **VS** vie scolaire, **DIR** direction, **CPT** intendance ou comptable, **SEC** secrétariat, **RESP** responsable légal, **SYS** système.

### 19.1 Paramétrage et lieu
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| CDI-100 | Activer le module CDI | DIR · `cdi.parametrer` | salle de type `cdi` au 5 | N2 | `cdi.active` | BIB0 |
| CDI-101 | Déclarer les localisations (CDI, réserve, annexes, internat) | DOC · `cdi.parametrer` | rattachées à un site ou une salle du 5 | N1 | — | BIB0 |
| CDI-102 | Choisir les règles de prêt (valeurs par défaut prêtes) | DOC · `cdi.parametrer` | §10.1 | N1 | `cdi.regles_modifiees` | BIB0 |
| CDI-103 | Définir les catégories d'emprunteurs et leurs règles | DOC | catégories calculées (élève collège, lycée, interne, personnel) | N1 | idem | BIB0 |
| CDI-104 | Publier les horaires d'ouverture | DOC | calendrier du 5, fermetures | N1 | `cdi.horaires_publies` | BIB6 |
| CDI-105 | Fixer la jauge par créneau | DOC | — | N1 | — | BIB6 |
| CDI-106 | Déclarer les espaces réservables au moteur du 19 | DOC | règles de préavis, durée | N1 | `agenda.lieu_declare` | BIB6 |
| CDI-107 | Déclarer sa présence ou celle d'un AED comme responsable | DOC, AEDC | agent habilité, plage | N1 | `cdi.responsable_present` | BIB6 |
| CDI-108 | Fermer le CDI à l'accueil libre (absence, réunion) | DOC, DIR | créneaux annulés, inscrits avisés | N2 | `cdi.ferme` | BIB6 |
| CDI-109 | Déposer le règlement du CDI | DOC, DIR | document du 7 ; public : annexe au RI délibéré (46) | N1 | — | BIB6 |
| CDI-110 | Choisir les trames d'étiquettes et de cartes (D-023) | DOC | 3 ou 4 trames | N1 | — | BIB1 |
| CDI-111 | Habiliter un AED, un surveillant d'internat ou un bénévole au comptoir | DIR · `cdi.habiliter` | rôle « Comptoir CDI » restreint ; bénévole : socle 6 (P21) | N2 | `roles.attribue` | BIB2 |
| CDI-112 | Activer le catalogue public sans connexion | DIR | aucune donnée d'emprunteur (S-4505) | N2 | — | BIB10 |
| CDI-113 | Déclarer les sources de notices (BnF, autres) | DOC | licence vérifiée (§5) | N1 | — | BIB1 |

### 19.2 Catalogue
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| CDI-114 | Créer une notice par scan d'ISBN | DOC · `cdi.cataloguer` | notice proposée depuis la source, validée | N1 | `cdi.notice_creee` | BIB1 |
| CDI-115 | Créer une notice à la main | DOC | contrôle de doublon | N1 | idem | BIB1 |
| CDI-116 | Modifier une notice | DOC | verrou optimiste, journal | N1 | `cdi.notice_modifiee` | BIB1 |
| CDI-117 | Fusionner deux notices | DOC | exemplaires et réservations réunis | N2 | `cdi.notices_fusionnees` | BIB1 |
| CDI-118 | Retirer une notice sans exemplaire | DOC | état daté, pas de `DELETE` (E6) | N1 | — | BIB1 |
| CDI-119 | Gérer les autorités (auteurs, rôles) | DOC | fusion d'auteurs | N1 | — | BIB1 |
| CDI-120 | Gérer le thésaurus de sujets de l'établissement | DOC | jeu de départ si licence (Q-CDI3) | N1 | — | BIB1 |
| CDI-121 | Indexer (cote, sujets, genre, niveau) | DOC | cote libre | N1 | — | BIB1 |
| CDI-122 | Marquer une version adaptée (gros caractères, audio, FALC) | DOC | — | N1 | — | BIB1 |
| CDI-123 | Ajouter un avertissement de contenu | DOC | — | N1 | — | BIB1 |
| CDI-124 | Ajouter une couverture | DOC | objet E5 ; source déclarée | N1 | — | BIB1 |
| CDI-125 | Créer une série (tomes ordonnés) | DOC | — | N1 | — | BIB1 |
| CDI-126 | Demander une notice à l'IA depuis une photo | DOC | §18 | N1 | — | BIB12 |
| CDI-127 | Accepter ou corriger l'indexation suggérée par l'IA | DOC | — | N1 | — | BIB12 |
| CDI-128 | Lier une ressource en ligne à une notice | DOC | lien interne ou externe déclaré | N1 | — | BIB10 |
| CDI-129 | Déposer un travail d'élève au fonds | DOC | accord du 18 (R-CDI16) | N2 | `cdi.travail_depose` | BIB11 |
| CDI-130 | Exporter le catalogue (UNIMARC, CSV) | DOC · `cdi.exporter` | sans emprunteurs ; trace d'export (9) | N2 | `echanges.exporte` | BIB5 |

### 19.3 Exemplaires, inventaire, récolement, désherbage
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| CDI-131 | Créer des exemplaires (un ou plusieurs) | DOC · `cdi.exemplaires` | code-barres par compteur (E3) ou saisi | N1 | `cdi.exemplaire_cree` | BIB1 |
| CDI-132 | Imprimer des étiquettes (code-barres, cote) | DOC | trame (D-023) | N0 | — | BIB1 |
| CDI-133 | Changer localisation, état, statut | DOC, AEDC | journal | N1 | `cdi.exemplaire_modifie` | BIB1 |
| CDI-134 | Exclure du prêt (consultation sur place) | DOC | — | N1 | idem | BIB1 |
| CDI-135 | Envoyer en réparation, revenir de réparation | DOC | — | N1 | idem | BIB1 |
| CDI-136 | Ouvrir une campagne de récolement (rayon, fonds) | DOC | — | N1 | `cdi.recolement_ouvert` | BIB5 |
| CDI-137 | Scanner un rayon au mobile | DOC, AEDC, bénévole habilité | hors ligne possible, synchronisé | N1 | — | BIB5 |
| CDI-138 | Voir manquants, « en prêt mais présents », mal rangés | DOC | — | N0 | — | BIB5 |
| CDI-139 | Clore le récolement avec décisions | DOC | aucun statut changé sans décision (R-CDI12) | N2 | `cdi.recolement_clos` | BIB5 |
| CDI-140 | Obtenir une liste de désherbage proposée | DOC | critères (§9) ; IA option | N0 | — | BIB5 |
| CDI-141 | Désherber (motif, devenir) document par document | DOC · `cdi.desherber` | sortie d'inventaire datée ; public : visa si requis [NV] | N2 | `cdi.exemplaire_retire` | BIB5 |
| CDI-142 | Exporter le registre d'inventaire d'un exercice | DOC, CPT | entrées, sorties, valeurs | N1 | — | BIB5 |
| CDI-143 | Transférer des exemplaires vers une autre localisation ou un site | DOC | — | N1 | — | BIB5 |
| CDI-144 | Enregistrer un don entrant | DOC | origine `don` | N1 | — | BIB1 |
| CDI-145 | Enregistrer une exposition prêtée par un partenaire (prêt entrant) | DOC | date de retour au prêteur, alerte | N1 | — | BIB11 |

### 19.4 Prêts, retours, réservations
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| CDI-146 | Prêter au comptoir (scan carte + exemplaire) | DOC, AEDC · `cdi.preter` | §10.2 ; contrôles | N1 | `cdi.pret_ouvert` | BIB2 |
| CDI-147 | Rechercher l'emprunteur par nom au comptoir | DOC, AEDC | personnel habilité ; résultat minimal (nom, classe, prêts en cours) | N0 | — | BIB2 |
| CDI-148 | Déroger à une règle (maximum, retard) avec motif | DOC | journal | N1 | `cdi.derogation` | BIB2 |
| CDI-149 | Enregistrer un retour | DOC, AEDC, surveillant habilité | relances annulées dans la transaction | N1 | `cdi.pret_rendu` | BIB2 |
| CDI-150 | Constater l'état au retour | DOC, AEDC | — | N1 | — | BIB2 |
| CDI-151 | Prolonger au comptoir | DOC, AEDC | refus si réservé | N1 | `cdi.pret_prolonge` | BIB2 |
| CDI-152 | Prolonger soi-même | ELV, personnel · `cdi.mes_prets` | idem | N1 | idem | BIB3 |
| CDI-153 | Réserver une notice | ELV, personnel, AESH pour l'élève accompagné | file ordonnée | N1 | `cdi.reservation_creee` | BIB3 |
| CDI-154 | Annuler sa réservation | ELV | — | N1 | — | BIB3 |
| CDI-155 | Mettre de côté un exemplaire réservé au retour | SYS | 7 jours, avis | — | `cdi.reservation_disponible` | BIB3 |
| CDI-156 | Expirer une réservation non retirée | SYS | suivant avisé | — | — | BIB3 |
| CDI-157 | Prêter une série de classe à un professeur | DOC | §10.2 | N1 | `cdi.serie_pretee` | BIB4 |
| CDI-158 | Attribuer les exemplaires d'une série aux élèves | PROF emprunteur, DOC | élèves du cours du professeur (D-022) | N1 | — | BIB4 |
| CDI-159 | Rendre une série et voir les manquants par élève | PROF, DOC | — | N1 | `cdi.serie_rendue` | BIB4 |
| CDI-160 | Prêter depuis l'internat le soir | surveillant habilité | catégorie interne | N1 | `cdi.pret_ouvert` | BIB4 |
| CDI-161 | Prêt par l'élève sur mobile, en attente de validation | ELV | créneau ouvert, responsable présent | N1 | `cdi.pret_demande` | BIB9 |
| CDI-162 | Valider ou refuser les prêts en attente | DOC, AEDC | — | N1 | `cdi.pret_ouvert` | BIB9 |
| CDI-163 | Voir mes prêts, échéances, réservations et rang | ELV, personnel | — | N0 | — | BIB3 |
| CDI-164 | Voir les prêts en cours de mon enfant (nombre, échéances) | RESP · `suivi_scolaire` (4) | sans titre (R-CDI3) | N0 | — | BIB3 |
| CDI-165 | Demander au documentaliste les titres en cours | RESP | décision du documentaliste (Q-CDI5) | N1 | — | BIB3 |
| CDI-166 | Liste des retards (documentaliste) | DOC | — | N0 | — | BIB2 |
| CDI-167 | Annuler une relance (erreur, retour non scanné) | DOC | — | N1 | — | BIB3 |
| CDI-168 | Suspendre les échéances (fermeture, vacances, congé) | DOC, SYS | R-CDI8 | N2 | `cdi.echeances_reportees` | BIB3 |
| CDI-169 | Activer mon historique de lecture personnel | ELV, personnel | opt-in, supprimable, visible de soi seul (R-CDI2 bis) | N1 | — | BIB10 |
| CDI-170 | Supprimer mon historique | ELV | immédiat | N1 | — | BIB10 |

### 19.5 Relances, pertes, dégradations
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| CDI-171 | Envoyer les rappels et relances graduées | SYS | §10.5, natures du 8 | — | `cdi.relance_envoyee` | BIB3 |
| CDI-172 | Relancer à la main un emprunteur | DOC | brouillon N3, destinataires calculés | N3 | idem | BIB3 |
| CDI-173 | Informer le PP (nombre sans titre) | SYS | à J+45 et en fin d'année | — | — | BIB3 |
| CDI-174 | Constater une perte ou une dégradation | DOC · `cdi.pertes` | barème exécutoire requis (46 ou organisme) | N2 | `cdi.perte_constatee` | BIB7 |
| CDI-175 | Proposer remplacement à l'identique | DOC | délai ; famille avisée | N3 | — | BIB7 |
| CDI-176 | Demander la facturation au 24 | DOC | payeurs du 4, montant du barème, référence | N3 | `facturation.ponctuelle_demandee` | BIB7 |
| CDI-177 | Accorder une remise gracieuse motivée | DIR · `cdi.remise` | motif non communiqué au comptoir | N2 | `cdi.remise_accordee` | BIB7 |
| CDI-178 | Enregistrer un remplacement reçu | DOC | nouvel exemplaire | N1 | — | BIB7 |
| CDI-179 | Retrouver un document facturé | DOC | avoir demandé au 24 (D-027 E) | N2 | `cdi.perte_annulee` | BIB7 |
| CDI-180 | Clore « sans suite » (décès, situation particulière) | DIR | aucune facture | N2 | — | BIB7 |
| CDI-181 | Préparer le barème pour le CA | DOC, DIR | projet de délibération transmis au 46 | N1 | `instances.point_propose` | BIB7 |

### 19.6 Manuels scolaires
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| CDI-182 | Associer des manuels aux enseignements et niveaux | DOC, DIR · `manuels.gerer` | socle 5 | N1 | — | BIB8 |
| CDI-183 | Calculer les besoins par classe (effectifs prévisionnels) | DOC | manque affiché | N0 | — | BIB8 |
| CDI-184 | Préparer les kits par élève | DOC | enseignements suivis (D-022) | N1 | — | BIB8 |
| CDI-185 | Ouvrir une campagne de distribution | DOC | classes, dates, salles | N1 | `manuels.campagne_ouverte` | BIB8 |
| CDI-186 | Distribuer (scan ou attribution en lot) | DOC, PP habilité | état constaté | N1 | `manuels.remis` | BIB8 |
| CDI-187 | Recueillir l'accusé de réception de la famille | SYS (18) | option | N3 | — | BIB8 |
| CDI-188 | Remettre un kit à un élève arrivé en cours d'année | DOC | — | N1 | — | BIB8 |
| CDI-189 | Échanger un manuel au changement de groupe | DOC | événement du 5 | N1 | — | BIB8 |
| CDI-190 | Ouvrir la campagne de retour | DOC | — | N1 | `manuels.campagne_retour` | BIB8 |
| CDI-191 | Constater l'état au retour, proposer une dégradation | DOC | seuil d'écart (CCD-25) | N2 | — | BIB8 |
| CDI-192 | Bilan des manuels non rendus par classe | DOC, PP | nombre ; titres au DOC | N0 | — | BIB8 |
| CDI-193 | Commander les manuels manquants | DOC | panier (16) | N1 | — | BIB8 |

### 19.7 Le CDI comme lieu
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| CDI-194 | Envoyer un élève ou un groupe au CDI | VS · `assiduite.localiser` (12) | jauge, responsable présent | N1 | `assiduite.localise` | BIB6 |
| CDI-195 | Réserver une place sur un créneau d'accueil | ELV | pas de cours à cette plage (10), jauge | N1 | `cdi.place_reservee` | BIB6 |
| CDI-196 | Annuler sa place | ELV | — | N1 | — | BIB6 |
| CDI-197 | Pointer une arrivée, un départ | DOC, AEDC | scan carte ou liste | N1 | `cdi.presence` | BIB6 |
| CDI-198 | Alerter la vie scolaire d'un élève non arrivé | SYS | H+10 min (R-CDI10) | — | `cdi.non_arrive` | BIB6 |
| CDI-199 | Voir les présents en temps réel | DOC, AEDC, VS | — | N0 | — | BIB6 |
| CDI-200 | Fournir la liste des présents en alerte | SYS (14) | — | — | — | BIB6 |
| CDI-201 | Signaler un incident au CDI | DOC, AEDC | vers le 13 (fait d'incident) | N2 | `discipline.incident_signale` | BIB6 |
| CDI-202 | Réserver un espace du CDI pour une classe | PROF (19) | règles du CDI | N1 | `agenda.lieu_reserve` | BIB6 |
| CDI-203 | Accepter ou refuser une réservation d'espace | DOC | motif au refus | N1 | — | BIB6 |
| CDI-204 | Voir la fréquentation (agrégée) | DOC, DIR | k ≥ 5 | N0 | — | BIB6 |

### 19.8 Ressources numériques et GAR
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| CDI-205 | Ouvrir le médiacentre (ressources GAR + libres) | ELV, personnels | SSO GAR (9) | N0 | — | BIB10 |
| CDI-206 | Déclarer une ressource libre ou un lien au portail | DOC | — | N1 | — | BIB10 |
| CDI-207 | Organiser la sitothèque par projet ou discipline | DOC, PROF | — | N1 | — | BIB10 |
| CDI-208 | Diagnostiquer « je ne vois pas cette ressource » | DOC (ITP-87) | lecture du 9 | N0 | — | BIB10 |
| CDI-209 | Voir l'état de l'alimentation GAR et les rejets | DOC, DIR (ITP-85) | — | N0 | — | BIB10 |
| CDI-210 | Voir les responsables d'affectation | DOC, DIR | — | N0 | — | BIB10 |
| CDI-211 | Enregistrer un abonnement numérique hors GAR | DOC | code chiffré si imposé (R-CDI14) | N2 | — | BIB10 |
| CDI-212 | Afficher le code d'accès d'une ressource à un personnel habilité | DOC | journal de lecture | N2 | `cdi.secret_lu` | BIB10 |
| CDI-213 | Basculer en mode « service raccordé » (médiacentre de l'ENT de la collectivité) | ADM | M-428 du 9 | N2 | — | BIB10 |

### 19.9 Portail, sélections, avis
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| CDI-214 | Rechercher dans le catalogue | tous | §14 | N0 | — | BIB3 |
| CDI-215 | Créer une sélection | DOC, PROF | — | N1 | — | BIB10 |
| CDI-216 | Publier une sélection au cahier de textes ou au club | PROF, DOC | 20, 43 | N1 | `cdi.selection_publiee` | BIB10 |
| CDI-217 | Publier les nouveautés | DOC | 17 | N1 | — | BIB10 |
| CDI-218 | Publier une actualité du CDI | DOC | 17 | N2 | `annonces.publiee` | BIB10 |
| CDI-219 | Rédiger un avis ou un coup de cœur | ELV | modération | N1 | `cdi.avis_soumis` | BIB10 |
| CDI-220 | Modérer un avis | DOC | refus motivé | N1 | — | BIB10 |
| CDI-221 | Créer une liste personnelle | ELV, personnels | privée | N1 | — | BIB10 |
| CDI-222 | Suggérer un achat | ELV, PROF | — | N1 | `cdi.suggestion` | BIB10 |
| CDI-223 | Répondre à une suggestion | DOC | motif | N1 | — | BIB10 |
| CDI-224 | Choisir la police adaptée, la synthèse vocale | tous | préférence personnelle | N1 | — | BIB10 |
| CDI-225 | Filtrer les versions adaptées | tous | — | N0 | — | BIB10 |
| CDI-226 | Demander l'aide de l'IA pour une recherche | ELV | §18 | N0 | — | BIB12 |

### 19.10 Enseignement et projets
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| CDI-227 | Programmer une séance EMI ou une co-intervention | DOC, PROF | port du 10 | N1 | `edt.seance_creee` | BIB11 |
| CDI-228 | Programmer l'accueil des nouveaux élèves par classe | DOC | séances du 10 | N1 | idem | BIB11 |
| CDI-229 | Déposer une fiche méthode, une grille d'évaluation de sources | DOC | 7 | N1 | — | BIB11 |
| CDI-230 | Créer un projet (presse, prix littéraire, exposition) | DOC | — | N1 | `cdi.projet_cree` | BIB11 |
| CDI-231 | Inscrire des élèves à un projet | DOC, PROF | 43 si atelier | N1 | — | BIB11 |
| CDI-232 | Organiser un vote d'élèves pour un prix littéraire | DOC | consultation du 18 (« sans valeur de scrutin ») | N1 | — | BIB11 |
| CDI-233 | Lier une séance aux compétences EMI | DOC | 22 | N1 | — | BIB11 |

### 19.11 Acquisitions et budget
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| CDI-234 | Créer un panier de commande | DOC · `cdi.acquerir` | fournisseur, ligne budgétaire (48) | N1 | — | BIB11 |
| CDI-235 | Soumettre la commande au visa | DOC | circuit du 48 | N2 | `achats.commande_soumise` | BIB11 |
| CDI-236 | Viser la commande | CPT, DIR (48) | — | N2 | `achats.commande_visee` | BIB11 |
| CDI-237 | Réceptionner au scan (notices et exemplaires créés) | DOC | — | N1 | `cdi.reception` | BIB11 |
| CDI-238 | Suivre les abonnements et leurs échéances | DOC | alerte J-60 | N0 | — | BIB11 |
| CDI-239 | Bulletiner un numéro de périodique | DOC, AEDC | — | N1 | `cdi.numero_recu` | BIB5 |
| CDI-240 | Réclamer un numéro manquant | DOC | brouillon au fournisseur | N3 | — | BIB5 |
| CDI-241 | Voir l'exécution du budget documentaire | DOC, DIR, CPT | lecture du 48 | N0 | — | BIB11 |

### 19.12 Départs, fin d'année, indicateurs
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| CDI-242 | Bilan des prêts au départ d'un élève | SYS → DOC, SEC | sans blocage | — | `cdi.bilan_depart` | BIB3 |
| CDI-243 | Campagne de retour de fin d'année | DOC | relances groupées | N3 | `cdi.campagne_fin_annee` | BIB7 |
| CDI-244 | Bilan par classe pour le PP (nombre, sans titre) | DOC, PP | — | N0 | — | BIB7 |
| CDI-245 | Générer le rapport d'activité annuel | DOC, DIR | agrégats k ≥ 5 | N1 | — | BIB11 |
| CDI-246 | Indicateurs au pilotage | SYS (47) | anonymes | — | — | BIB11 |
| CDI-247 | Délier les historiques échus | SYS | R-CDI2, quotidien | — | `cdi.historique_delie` | BIB2 |

### 19.13 Reprise et système
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| CDI-248 | Importer un catalogue d'un SIGB (UNIMARC, MARC21, CSV) | DOC · `cdi.importer` | moteur du 9, lot annulable | N2 | `echanges.lot_applique` | BIB5 |
| CDI-249 | Importer les exemplaires et les prêts **en cours** | DOC | rapprochement emprunteur par INE puis nom + naissance, sinon file humaine | N2 | idem | BIB5 |
| CDI-250 | Refuser l'historique clos à l'import | SYS | R-CDI18 | — | — | BIB5 |
| CDI-251 | Mode passerelle : import nocturne du catalogue d'un SIGB conservé | DOC | prêt resté dans l'ancien outil (§26) | N2 | — | BIB5 |
| CDI-252 | Annuler un lot d'import | DOC | 9 | N2 | — | BIB5 |
| CDI-253 | Générer les cartes d'emprunteur d'une classe | DOC | jeton opaque, trame D-023 | N1 | — | BIB2 |
| CDI-254 | Révoquer une carte perdue | DOC, ELV | nouveau jeton | N1 | `cdi.carte_revoquee` | BIB2 |
| CDI-255 | Journal des actions du CDI | DOC, DIR | 12 mois pour les accès | N0 | — | BIB2 |
| CDI-256 | Exporter mes données de prêt (droit d'accès) | ELV, RESP, personnel | 27 | N2 | — | BIB13 |

### 19.14 Ajouts de cohérence (mobile, accessibilité, exploitation)
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| CDI-257 | Scanner un ISBN avec l'appareil photo (mobile) | DOC | appli unique (D-003), droit documentaliste | N1 | — | BIB9 |
| CDI-258 | Comptoir hors ligne (coupure réseau) | DOC, AEDC | file locale chiffrée, synchronisation, conflits signalés | N1 | — | BIB9 |
| CDI-259 | Voir « Prêts à rendre » sur l'accueil élève (29) | ELV | tuile | N0 | — | BIB3 |
| CDI-260 | Tuile documentaliste (séances, retards, réservations) | DOC (29) | — | N0 | — | BIB3 |
| CDI-261 | Notifier « ta réservation est disponible » | SYS (8) | — | — | — | BIB3 |
| CDI-262 | Paramétrer la boîte de retour (date déclarée) | DOC | CCD-14 | N1 | — | BIB2 |
| CDI-263 | Fusionner deux autorités d'auteur | DOC | — | N1 | — | BIB1 |
| CDI-264 | Exporter la liste d'une sélection (PDF) pour affichage | DOC | sans emprunteurs | N1 | — | BIB10 |
| CDI-265 | Imprimer un ticket de prêt (dates) | DOC, AEDC | option, sans données autres | N0 | — | BIB2 |
| CDI-266 | Transférer le rôle de responsable d'affectation (vue 9) | DIR | 9 | N2 | — | BIB10 |
| CDI-267 | Déclarer un bénéficiaire de l'exception handicap | DOC | sur justificatif vu, motif non stocké [NV] | N2 | — | BIB13 |
| CDI-268 | Contrôler la conformité des durées (DPO) | DPO, DIR | rapport de déliement | N0 | — | BIB13 |
| CDI-269 | Neutraliser les envois à la restauration d'une sauvegarde | SYS | E9 | — | — | BIB0 |
| CDI-270 | Retirer le module (fin d'usage) | DIR | export, archive | N3 | — | BIB13 |

**Total : 171 actions.**

---

## 20. Règles transverses (R-CDI1 à R-CDI30) [R]

| N° | Règle |
|---|---|
| R-CDI1 | **Confidentialité des lectures** : le lien nominatif emprunteur ↔ document n'est lisible que du documentaliste et des agents du comptoir habilités, de l'emprunteur, et (prêts **en cours**, sans titre) du responsable qui a le suivi. Ni la direction, ni les professeurs, ni la vie scolaire ne lisent les prêts d'un élève. |
| R-CDI2 | Le lien nominatif d'un prêt clos est **délié** (emprunteur remplacé par une catégorie, une classe et un niveau à la date) **30 jours** après le retour, sauf perte ou litige en cours [R, durée soumise au DPO]. |
| R-CDI2 bis | Un historique personnel n'existe que sur demande de l'emprunteur (opt-in), visible de lui seul, supprimable à tout moment. |
| R-CDI3 | La famille voit qu'un document est en retard, pas son titre ; le titre et le prix lui sont communiqués au stade de la perte facturée. |
| R-CDI4 | Aucune liste nominative de retardataires n'est affichable, imprimable pour la classe ou envoyable à un groupe. |
| R-CDI5 | Aucune recommandation fondée sur l'historique personnel d'un mineur ; co-emprunts seulement agrégés (k ≥ 5). |
| R-CDI6 | Toute statistique publiée ou au pilotage est agrégée avec un seuil k ≥ 5. |
| R-CDI7 | Un retard ou une perte ne bloque **que** le prêt suivant (dérogeable) : jamais un bulletin, un certificat, un exeat, une inscription, un examen, une sortie. |
| R-CDI8 | Les échéances sont reportées pendant les vacances et les fermetures ; aucune relance n'est envoyée pendant les vacances. |
| R-CDI9 | La carte d'emprunteur porte un **jeton opaque** révocable, jamais l'INE, le numéro d'élève ni la date de naissance. |
| R-CDI10 | Un élève envoyé ou inscrit au CDI est **localisé** (12) ; non arrivé à H+10 min, alerte à la vie scolaire. |
| R-CDI11 | Sans responsable présent déclaré, aucun accueil libre n'est proposé. |
| R-CDI12 | Aucun récolement ne change un statut sans décision humaine. |
| R-CDI13 | Les photos d'état d'un manuel ne montrent que l'objet ; supprimées à la clôture de la campagne de retour. |
| R-CDI14 | Aucun identifiant ou mot de passe partagé d'éditeur stocké en clair ; codes imposés chiffrés (E11), lecture journalisée. |
| R-CDI15 | Avis d'élèves modérés avant publication, jamais visibles hors de l'établissement. |
| R-CDI16 | Un travail d'élève n'entre au fonds qu'avec l'accord de l'élève et, s'il est mineur, de ses responsables (18). |
| R-CDI17 | Un barème de perte ne s'applique qu'après délibération exécutoire (public, 46) ou décision de l'organisme (privé) ; à défaut, seul le remplacement à l'identique est proposé. |
| R-CDI18 | La reprise n'importe jamais l'historique de prêt clos. |
| R-CDI19 | Le prêt et les relances d'un élève majeur ne concernent que lui (4). |
| R-CDI20 | Les destinataires d'une relance sont calculés par le serveur au moment de l'envoi (4), jamais stockés à la création. |
| R-CDI21 | Le documentaliste a le droit de lire **tous** les élèves comme emprunteurs (identité minimale : nom, prénom, classe, photo si activée) et **aucune** autre donnée scolaire. |
| R-CDI22 | Un professeur ne voit que les exemplaires de **ses** séries et les élèves de **ses** cours (D-022). |
| R-CDI23 | Un code-barres d'exemplaire est unique par établissement et jamais réutilisé. |
| R-CDI24 | Les dates sont en Europe/Paris ; une échéance tombe sur un jour d'ouverture du CDI. |
| R-CDI25 | Aucune donnée d'un établissement n'est visible d'un autre, même pour un documentaliste à temps partagé (deux accès). |
| R-CDI26 | Toute action d'écriture est journalisée (E7) ; les accès aux listes nominatives sont journalisés 12 mois. |
| R-CDI27 | Les textes importés (notices, résumés) sont des données non fiables pour l'IA. |
| R-CDI28 | La présence au CDI n'entre dans aucun compteur d'assiduité. |
| R-CDI29 | Le décès d'un élève arrête toute relance dans la transaction de l'événement. |
| R-CDI30 | Aucune « amende » de retard n'existe dans le logiciel. |

---

## 21. Droits et minimisation

### 21.1 Catalogue d'actions proposé (D-017) [R]
| Droit | Portée | Modèles de rôle |
|---|---|---|
| `cdi.parametrer` | établissement | Documentaliste ; Direction |
| `cdi.cataloguer`, `cdi.exemplaires`, `cdi.desherber`, `cdi.importer`, `cdi.exporter` | établissement | Documentaliste |
| `cdi.preter` | établissement (comptoir) | Documentaliste ; **Comptoir CDI** (AED affecté, bénévole, surveillant d'internat : `eleve_interne` seulement) |
| `cdi.emprunteurs.lire` | établissement, identité minimale | Documentaliste ; Comptoir CDI |
| `cdi.pertes` | établissement | Documentaliste |
| `cdi.remise` | établissement | Direction |
| `cdi.habiliter` | établissement | Direction |
| `cdi.acquerir` | établissement | Documentaliste |
| `cdi.mes_prets` | soi | tous les emprunteurs |
| `cdi.series` | mes séries | Professeur |
| `manuels.gerer` | établissement | Documentaliste ; Direction ; (option) Secrétariat |
| `manuels.distribuer` | mes classes | Professeur principal (pendant une campagne) |
| `cdi.presence` | lieu CDI | Documentaliste ; Comptoir CDI ; lecture : Vie scolaire |
| `cdi.indicateurs` | établissement, agrégés | Documentaliste ; Direction |
| `cdi.secret.lire` | ressource | Documentaliste ; personnels désignés |

Nouveau **modèle de rôle « Documentaliste »** : les droits ci-dessus, plus les droits ordinaires d'un enseignant pour ses séances (appel de **ses** séances, cahier de textes), **sans** `notes.lire` ni `vie_scolaire.lire` hors de ses séances.

### 21.2 Ce que voit chacun [R]
| Information | Documentaliste | Comptoir | Élève | Responsable | Professeur | PP | Vie scolaire | Direction |
|---|---|---|---|---|---|---|---|---|
| Catalogue, disponibilité agrégée | oui | oui | oui | oui | oui | oui | oui | oui |
| Prêts en cours d'un élève, titres | oui | oui (au comptoir) | les siens | non (Q-CDI5) | séries seulement | non | non | non |
| Prêts en cours, nombre et échéances | oui | oui | les siens | oui (suivi) | séries | fin d'année : nombre | non | non |
| Historique clos nominatif | non (délié) | non | le sien si opt-in | non | non | non | non | non |
| Présence au CDI | oui | oui | la sienne | non (sauf localisation du 12 selon ses règles) | non | non | oui | oui (liste d'alerte) |
| Pertes facturées | oui | non | les siennes | oui | non | non | non | remise : oui |
| Indicateurs | oui | non | non | non | non | non | non | oui |

### 21.3 Conservation [R, durées soumises au DPO]
| Donnée | Durée | Sort |
|---|---|---|
| Lien nominatif d'un prêt clos | 30 jours après retour | délié (R-CDI2) |
| Prêt perdu ou en litige | jusqu'au solde de la facture + 1 an | délié |
| Présence au CDI | fin de l'année scolaire | supprimée (agrégats gardés) |
| Réservation close | 30 jours | supprimée |
| Avis signés | tant que l'élève est scolarisé, puis anonymisés | — |
| Historique personnel opt-in | jusqu'à suppression par l'emprunteur ou départ + 3 mois | supprimé |
| Notices, exemplaires, inventaire | durée de vie du fonds ; registre d'inventaire 10 ans [H, archives] | archive (27) |
| Journal d'accès aux listes nominatives | 12 mois (kit juridique SDET, `19_AGENDA_RDV.md` §6.5) | supprimé |
| Emprunteur parti | 3 mois après le départ (arrêté ENT, `19_AGENDA_RDV.md` §6.5), sauf prêt ouvert | supprimé |

### 21.4 Pièges à tester
1. Un professeur appelle `cdi::prets_en_cours(eleve)` d'un élève de son cours : **refusé** (R-CDI1).
2. Un parent sans droit de suivi appelle la liste des prêts de l'enfant : refusé.
3. Un élève appelle les prêts d'un autre élève en changeant l'identifiant : refusé, journalisé.
4. Le PP obtient le bilan de fin d'année : nombre seulement, jamais de titre.
5. Le rapport d'activité sur une classe de 4 élèves : agrégat masqué.
6. Un AED du comptoir tente d'exporter le catalogue avec les emprunteurs : refusé (aucune route ne le permet).
7. Un surveillant d'internat prête à un externe : refusé.
8. J+31 après retour : la requête nominative ne retrouve plus l'emprunteur.
9. L'IA reçoit-elle un identifiant d'élève dans `cdi.proposer_desherbage` ? test : non.
10. Un documentaliste à temps partagé voit-il, dans l'établissement B, un prêt de l'établissement A ? non (RLS).

---

## 22. Événements (outbox, dans la transaction du fait) [R]
| Événement | Charge (identifiants seulement) | Consommateurs |
|---|---|---|
| `cdi.active`, `cdi.regles_modifiees` | établissement | 29 |
| `cdi.notice_creee` / `_modifiee`, `cdi.notices_fusionnees` | notice | recherche, 17 |
| `cdi.exemplaire_cree` / `_modifie` / `_retire` | exemplaire | 47 |
| `cdi.pret_ouvert` / `_prolonge` / `_rendu`, `cdi.pret_demande` | prêt, exemplaire, emprunteur | 29, 8 |
| `cdi.reservation_creee` / `_disponible` | réservation | 8 |
| `cdi.relance_envoyee` | prêt, étape | 8 (preuve) |
| `cdi.perte_constatee` / `_annulee` | prêt | 24 |
| `cdi.bilan_depart` | élève, nombre | 2, secrétariat |
| `cdi.presence`, `cdi.non_arrive` | élève, plage | 12, 14 |
| `cdi.ferme`, `cdi.responsable_present` | lieu, plage | 12, 19 |
| `manuels.campagne_ouverte` / `_retour`, `manuels.remis` | campagne | 29 |
| `cdi.historique_delie` | nombre | 27 |
| `cdi.serie_pretee` / `_rendue` | série | — |
| `cdi.secret_lu` | ressource, lecteur | journal de sécurité |

Consommés : `eleve.sortie`, `eleve.fusionne`, `eleve.reintegre`, `eleve.majeur`, `eleve.decede` (2) ; `liens.*` (4) ; `referentiel.membre_cours_*` (5) ; `personnel.engagement_clos`, `absences_personnels.declaree` (6, 15) ; `structure.fermeture` (5) ; `discipline.mesure_*` (13 : exclusion, interdiction de CDI) ; `instances.deliberation_executoire` (46) ; `gar.alimentation_*` (9).

## 23. Notifications (natures déclarées au socle 8) [R]
| Nature | Urgence | Sensibilité | Destinataires | Plancher |
|---|---|---|---|---|
| `cdi.rappel_echeance` | U3 | faible | emprunteur | désactivable |
| `cdi.retard` (étapes 1–3) | U3 | moyenne (pas de titre aux familles) | emprunteur ; responsables suivi (étape 2+) | étape 2+ non désactivable |
| `cdi.reservation_disponible` | U2 | faible | emprunteur | désactivable |
| `cdi.perte_facturee` | U2 | moyenne | payeurs (24) | non désactivable |
| `cdi.non_arrive` | U1 | moyenne | vie scolaire | non désactivable |
| `cdi.ferme` | U2 | faible | élèves inscrits | non désactivable |
| `manuels.a_rendre` | U3 | faible | élève, responsables | non désactivable |
| `cdi.bilan_depart` | U3 | moyenne | documentaliste, secrétariat | non désactivable |
Texte hors ENT (e-mail, SMS) : enveloppe neutre du 8, **jamais** de titre de livre dans un SMS ou un objet d'e-mail.

## 24. Écrans (`DESIGN_SYSTEM.md`, `FRONTEND_CIBLE.md`, Leptos, `FRONTEND_LEPTOS.md`)
| Écran | Route | Contenu |
|---|---|---|
| Comptoir | `/web/cdi/comptoir` | champ de scan unique (carte ou exemplaire, détection), fiche minimale de l'emprunteur, prêter/rendre/prolonger, retards en rouge discret ; plein écran, clavier seul |
| Tableau du documentaliste | `/web/cdi` | tuiles Bento : présents maintenant, séances du jour, retards, réservations à mettre de côté, numéros attendus, alimentation GAR |
| Catalogue (gestion) | `/web/cdi/catalogue` | tableau dense, filtres, édition en panneau latéral, scan ISBN |
| Récolement | `/web/cdi/recolement` (mobile surtout) | scan continu, compteur, écarts |
| Présence | `/web/cdi/presence` | liste des présents, attendus, non arrivés ; jauge |
| Manuels | `/web/cdi/manuels` | campagnes, classes, avancement, état |
| Portail | `/web/cdi/portail` | recherche, nouveautés, sélections, mes prêts ; accessible |
| Médiacentre | `/web/mediacentre` | ressources GAR et libres ; diagnostic pour le documentaliste |
| Acquisitions | `/web/cdi/acquisitions` | paniers, commandes, abonnements |
| Mobile (D-003) | appli unique | élève : mes prêts, réserver, scan en attente, place au CDI ; documentaliste : scan ISBN, récolement, comptoir de secours |

---

## 25. Modèle de données cible (brique `cdi`) [R]

Toutes les tables : `etablissement_id UUID NOT NULL` (E1), clé `(etablissement_id, id)` et FK composites (E2), UUID générés par l'application (E3), `cree_le`, `cree_par`, `modifie_le`, `version` (E6), RLS standard (E13), aucune colonne JSON ni tableau d'identifiants (D-036, E10), fichiers en objets (E5). Les codes (nature, genre, rôle d'auteur, niveau, état, statut) sont des tables de référence ; les tables à `etablissement_id NULL` sont classées `globale_reference` au registre (E12).

```sql
-- 2900_cdi_catalogue.sql
ref_cdi_nature_notice (code PK, libelle)                          -- globale_reference
ref_cdi_role_auteur (code PK, libelle)                            -- globale_reference
ref_cdi_genre (code PK, libelle)                                  -- globale_reference
ent_cdi_parametres (etablissement_id PK, actif, catalogue_public bool, delai_deliement_jours INT DEFAULT 30,
                    mode CHECK (sigb|passerelle), version, ...)
ent_cdi_autorite (id, etablissement_id, nom, prenom, nom_normalise, fusionnee_dans NULL)
ent_cdi_sujet (id, etablissement_id, libelle, parent_id NULL, source CHECK (etablissement|jeu_depart))
ent_cdi_notice (id, etablissement_id, nature_code, titre, sous_titre, titre_normalise, serie_id NULL, rang_serie INT NULL,
                editeur, collection, annee INT NULL, isbn13 TEXT NULL, issn TEXT NULL, langue, pages INT NULL,
                niveau CHECK (cycle3|cycle4|lycee|adulte|tous), genre_code NULL, resume, resume_origine CHECK (editeur|documentaliste|ia_valide),
                cote, adaptation CHECK (aucune|gros_caracteres|audio|falc|daisy) DEFAULT 'aucune', avertissement TEXT NULL,
                couverture_objet_id NULL, lien_ressource TEXT NULL, origine CHECK (saisie|import_bnf|import_sigb|ia_valide),
                parent_notice_id NULL,  -- numéro de périodique -> titre
                date_parution DATE NULL, etat CHECK (active|retiree), fusionnee_dans NULL, ...)
ent_cdi_serie (id, etablissement_id, titre)
ent_cdi_notice_auteur (etablissement_id, notice_id, autorite_id, role_code, ordre)
ent_cdi_notice_sujet (etablissement_id, notice_id, sujet_id)
-- 2901_cdi_exemplaires.sql
ent_cdi_localisation (id, etablissement_id, libelle, site_id NULL, salle_id NULL, actif)
ent_cdi_compteur (etablissement_id PK, dernier_code BIGINT)
ent_exemplaire (id, etablissement_id, domaine CHECK (cdi|manuel|materiel_44), notice_id NULL, code_barres TEXT,
                cote, localisation_id, etat CHECK (neuf|bon|use|abime|hors_service), statut CHECK (disponible|en_pret|reserve|exclu_pret|en_reparation|perdu|retire),
                acquisition_mode CHECK (achat|don|specimen|abonnement|transfert), acquis_le DATE, prix_centimes BIGINT NULL,
                commande_ligne_id NULL, retire_le NULL, retrait_motif NULL, retrait_devenir CHECK (pilon|don|vente|transfert|archive) NULL, ...,
                UNIQUE (etablissement_id, code_barres),
                CHECK (domaine <> 'cdi' OR notice_id IS NOT NULL))
ent_exemplaire_fait (id, etablissement_id, exemplaire_id, a, type, avant TEXT, apres TEXT, acteur_personne_id, motif)  -- ajout seul
-- 2902_cdi_emprunteurs.sql
ent_pret_categorie (id, etablissement_id, code, libelle, max_documents INT, duree_jours INT, prolongations INT, reservation bool)
ent_pret_regle_nature (etablissement_id, categorie_id, nature_code, duree_jours INT NULL, pretable bool)
ent_pret_carte (id, etablissement_id, personne_id, jeton_empreinte TEXT, emise_le, revoquee_le NULL,
                UNIQUE (etablissement_id, jeton_empreinte))  -- jeton opaque, seule l'empreinte est stockée
-- 2903_cdi_prets.sql
ent_pret (id, etablissement_id, exemplaire_id, emprunteur_personne_id NULL, categorie_id,
          classe_id_a_date NULL, niveau_a_date NULL,                   -- gardés après déliement
          origine CHECK (comptoir|mobile|serie|internat|campagne_manuel|reprise), serie_id NULL, campagne_id NULL,
          etat CHECK (demande|ouvert|rendu|perdu|sans_suite), ouvert_le, echeance DATE, rendu_le NULL,
          etat_retour NULL, delie_le NULL, ...,
          CHECK (delie_le IS NULL OR emprunteur_personne_id IS NULL))
CREATE UNIQUE INDEX ... ON ent_pret (etablissement_id, exemplaire_id) WHERE etat IN ('demande','ouvert');
ent_pret_fait (id, etablissement_id, pret_id, a, type CHECK (ouverture|prolongation|retour|relance|derogation|perte|annulation), etape INT NULL, acteur_personne_id NULL, motif)
ent_pret_serie (id, etablissement_id, notice_id, professeur_personne_id, cours_id NULL, ouverte_le, close_le NULL)
ent_pret_serie_attribution (etablissement_id, serie_id, exemplaire_id, eleve_id, du, au NULL)
ent_pret_reservation (id, etablissement_id, notice_id, personne_id, pour_eleve_id NULL, rang INT, etat CHECK (attente|disponible|retiree|annulee|expiree),
                      exemplaire_id NULL, disponible_jusqu_au DATE NULL, ...)
ent_pret_perte (id, etablissement_id, pret_id, nature CHECK (perte|degradation), decision CHECK (remplacement|facture|remise|sans_suite),
                bareme_id NULL, montant_centimes BIGINT NULL, facture_ref UUID NULL, avoir_ref UUID NULL, decide_par, decide_le, motif)
ent_pret_bareme (id, etablissement_id, libelle, mode CHECK (valeur_neuf|decote), decote_bp_par_an INT NULL, plancher_centimes BIGINT,
                 deliberation_ref UUID NULL, decision_organisme_ref UUID NULL, applicable_du DATE, applicable_au DATE NULL)
ent_cdi_historique_personnel (id, etablissement_id, personne_id, notice_id, emprunte_le DATE)  -- opt-in seulement
-- 2904_cdi_periodiques.sql
ent_cdi_abonnement (id, etablissement_id, notice_id, fournisseur_id NULL, du, au, periodicite CHECK (...), cout_centimes, renouvellement_decide_le NULL)
ent_cdi_numero_attendu (id, etablissement_id, abonnement_id, date_prevue, etat CHECK (attendu|recu|manquant|reclame), numero_notice_id NULL)
-- 2905_manuels.sql
ent_manuel_affectation (id, etablissement_id, notice_id, annee_id, niveau_id NULL, enseignement_id NULL, option_id NULL)
ent_manuel_campagne (id, etablissement_id, annee_id, sens CHECK (distribution|retour), du, au, etat)
ent_manuel_campagne_classe (etablissement_id, campagne_id, classe_id, etat, ouverte_par)
ent_manuel_etat_photo (id, etablissement_id, pret_id, objet_id, prise_le, supprimee_le NULL)   -- R-CDI13
-- 2906_cdi_lieu.sql
ent_cdi_ouverture (id, etablissement_id, localisation_id, jour_semaine INT, debut TIME, fin TIME, jauge INT, du DATE, au DATE NULL)
ent_cdi_fermeture (id, etablissement_id, localisation_id, debut timestamptz, fin timestamptz, motif_code)
ent_cdi_responsable_present (id, etablissement_id, localisation_id, agent_id, debut, fin)
ent_cdi_place (id, etablissement_id, localisation_id, eleve_id, debut, fin, origine CHECK (eleve|vie_scolaire|seance), etat CHECK (prevue|arrivee|partie|non_arrivee|annulee),
               arrivee_le NULL, depart_le NULL)
-- 2907_cdi_acquisitions.sql
ent_cdi_fournisseur (id, etablissement_id, nom, tiers_ref NULL)       -- tiers du 48 quand il existe
ent_cdi_panier (id, etablissement_id, fournisseur_id, ligne_budgetaire_ref NULL, etat CHECK (brouillon|soumis|vise|commande|recu_partiel|recu|annule), ...)
ent_cdi_panier_ligne (id, etablissement_id, panier_id, isbn13 NULL, titre, quantite INT, prix_public_centimes, remise_bp INT, notice_id NULL, recue INT DEFAULT 0)
ent_cdi_suggestion (id, etablissement_id, auteur_personne_id, texte, etat, reponse NULL, repondu_par NULL)
-- 2908_cdi_numerique.sql
ent_cdi_ressource_libre (id, etablissement_id, titre, url, public CHECK (eleves|personnels|tous), discipline_id NULL, actif)
ent_cdi_acces_editeur (id, etablissement_id, ressource, code_chiffre (type du champ chiffré existant `ent_db::champs_chiffres`), kid, lu_journalise bool, ...)  -- E11 ; pas un contenu de fichier
-- 2909_cdi_portail.sql
ent_cdi_selection (id, etablissement_id, titre, description, auteur_personne_id, publique bool, cours_id NULL, club_ref UUID NULL)
ent_cdi_selection_notice (etablissement_id, selection_id, notice_id, ordre, commentaire)
ent_cdi_avis (id, etablissement_id, notice_id, auteur_personne_id NULL, signature CHECK (anonyme|prenom_classe), texte, note INT NULL,
              etat CHECK (soumis|publie|refuse), modere_par NULL, motif_refus NULL)
ent_cdi_liste_personnelle (id, etablissement_id, personne_id, titre) ; ent_cdi_liste_notice (...)
-- 2910_cdi_recolement.sql
ent_cdi_recolement (id, etablissement_id, perimetre_localisation_id NULL, ouvert_le, clos_le NULL)
ent_cdi_recolement_scan (id, etablissement_id, recolement_id, exemplaire_id NULL, code_lu, localisation_id, scanne_le, scanne_par)
ent_cdi_recolement_decision (etablissement_id, recolement_id, exemplaire_id, decision CHECK (retrouve|rechercher|perdu|retour_enregistre|replace), decide_par)
-- 2911_cdi_projets.sql
ent_cdi_projet (id, etablissement_id, titre, nature_code, du, au) ; ent_cdi_projet_participant (etablissement_id, projet_id, eleve_id)
ent_cdi_pret_entrant (id, etablissement_id, preteur, notice_id, recu_le, a_rendre_le, rendu_le NULL)
ent_cdi_travail_eleve (id, etablissement_id, notice_id, eleve_id, accord_formulaire_ref UUID, retire_le NULL)
-- 2912 RLS stricte et tests ; 2913–2918 réserve ; 2919 retrait (réservé, E8)
```

Notes :
- `ent_cdi_acces_editeur.code` utilise le **champ chiffré** existant (`ent_db::champs_chiffres`) avec identifiant de clé (E11) ; ce n'est pas un contenu de fichier (E5 respectée). Si les champs chiffrés sont en `BYTEA`, c'est l'exception déjà admise pour les champs chiffrés, pas un nouveau stockage de fichier.
- `ent_exemplaire` et `ent_pret*` sont **le moteur commun** avec le 44 (`domaine = 'materiel_44'`) ; le 44 ajoute ses propres tables (caution, numéro de série, maintenance) dans sa plage, avec FK vers `ent_exemplaire`.
- `ent_exemplaire_fait.avant/apres` sont des **textes d'instantané** (exception permise de D-036 : historique figé), pas du JSON relu.
- Aucune table sans `etablissement_id` sauf les `ref_cdi_*` (`globale_reference`).
- Neutralisation à la restauration (E9) : les relances programmées sont des avis du 8 (déjà neutralisés) ; le 45 n'a pas de file propre.

---

## 26. Reprise depuis un SIGB et coexistence pendant le doublon

1. **Pas de Charlemagne, pas de Next.js** : le CDI n'est concerné ni par le régime « doublon » ni par « usage réel de l'add-on » de D-035. Il est un **nouvel usage**. Recommandation : mettre le CDI du pilote en **usage réel** dès que BIB0–BIB5 sont recettés (parité SIGB), avec la reprise de son SIGB ; ScolIA n'a pas de concurrent officiel à doubler.
2. **Import** (moteur du 9) : notices (UNIMARC ou MARC21 ; CSV en dernier recours), exemplaires (code-barres repris tels quels, collisions en file), prêts **en cours** (emprunteur rapproché par INE, puis nom + date de naissance, sinon file humaine) ; **jamais** l'historique clos (R-CDI18) ; lecteurs adultes non scolarisés **non repris** (minimisation).
3. **Mode passerelle** (si le documentaliste veut garder son SIGB quelques mois) : import nocturne du catalogue pour la recherche du portail, prêt resté dans l'ancien outil, aucun emprunteur échangé ; fin de la passerelle à une date fixée.
4. **Formats** : connaître le format d'export du SIGB du pilote (Q-CDI1) ; [NV] sur les exports exacts de BCDI et PMB.
5. **Données réelles** : aucune avant l'accord écrit de l'établissement et l'avis du DPO (D-034, D-035 point 4) ; essais sur un export anonymisé fourni par Florian.

---

## 27. Lots de code (équipe Sonnet) et migrations

### 27.1 Lots
| Lot | Contenu | Dépend de | Critères de fin |
|---|---|---|---|
| **BIB0** fondation | paramètres, localisations, catégories et règles, modèle de rôle Documentaliste et Comptoir, références globales, registre E12, RLS | socles 1, 2, 5, 6 | tests d'isolation ; registre complet |
| **BIB1** catalogue et exemplaires | notices, autorités, sujets, séries, exemplaires, étiquettes, scan ISBN (source déclarée) | BIB0 ; 7 | CCD-16, CCD-23 |
| **BIB2** prêt au comptoir | cartes à jeton, prêt, retour, prolongation, dérogation, retards, déliement à J+30, journal | BIB1 | pièges §21.4 n° 1, 3, 6, 8 ; R-CDI1, R-CDI2, R-CDI9 |
| **BIB3** élève et familles | portail minimal, mes prêts, réservations, relances graduées (8), vue famille sans titre, bilan au départ, tuiles du 29 | BIB2 ; 4, 8, 29 | CCD-02, 05–08 ; R-CDI3, R-CDI7, R-CDI8, R-CDI20 |
| **BIB4** séries et internat | séries de classe, attribution, prêt à l'internat | BIB2 ; 5 | CCD-18 ; R-CDI22 |
| **BIB5** parité SIGB | import UNIMARC/MARC/CSV, mode passerelle, export, périodiques et bulletinage, récolement, désherbage, inventaire | BIB1 ; 9 (moteur) | jeu d'or d'un export réel anonymisé ; CCD-20, CCD-27 |
| **BIB6** le lieu | ouvertures, jauge, responsable présent, places, envoi par la vie scolaire, non-arrivés, présents en alerte, espaces au 19, règlement | BIB0 ; 10, 12, 14, 19 | CCD-29 à CCD-36 ; R-CDI10, R-CDI11, R-CDI28 |
| **BIB7** pertes et fin d'année | barèmes liés aux délibérations, constat, facture 24, avoir, remise, campagne de fin d'année, bilan PP | BIB3 ; 24, 46 | R-CDI17 ; CCD-19 |
| **BIB8** manuels | affectations aux enseignements, besoins, kits, campagnes de distribution et retour, état, échanges | BIB2 ; 5 | CCD-24 à CCD-26 |
| **BIB9** mobile | scan ISBN et récolement au mobile, prêt en attente, comptoir hors ligne | BIB2, BIB5 ; 11 | CCD-17 ; conflit hors ligne |
| **BIB10** portail complet et numérique | recherche avancée, sélections, avis modérés, listes, suggestions, accessibilité, catalogue public, médiacentre, diagnostic GAR, ressources libres, codes chiffrés | BIB3 ; 9 X11–X13 pour le GAR | R-CDI14, R-CDI15 ; diagnostic sur 5 causes |
| **BIB11** enseignement, projets, acquisitions | séances, projets, prêts entrants, travaux d'élèves, paniers, visa, réception, abonnements, rapport annuel, indicateurs | BIB1 ; 10, 18, 20, 22, 47, 48 | R-CDI16 |
| **BIB12** IA | capacités du §18.3 | BIB1 ; `ent-ia` | aucune donnée nominative dans un contexte ; test d'injection dans un résumé |
| **BIB13** conformité | droit d'accès, exception handicap [après vérification], rapport DPO, retrait | tous ; 27 | §21.3 vérifié |

**Parcours minimal** : démo de décembre et V1 de février 2027 = BIB0–BIB4 ; usage réel du pilote = BIB0–BIB7 (+ BIB8 pour un collège).

### 27.2 Migrations (plage imposée 2900–2919)
| N° | Lot | Contenu |
|---|---|---|
| 2900 | BIB0, BIB1 | catalogue, références globales, paramètres |
| 2901 | BIB1 | localisations, compteur, exemplaires (moteur commun avec le 44), faits |
| 2902 | BIB0, BIB2 | catégories, règles, cartes |
| 2903 | BIB2–BIB4, BIB7 | prêts, faits, séries, réservations, pertes, barèmes, historique opt-in |
| 2904 | BIB5 | périodiques |
| 2905 | BIB8 | manuels |
| 2906 | BIB6 | lieu (ouvertures, fermetures, responsables, places) |
| 2907 | BIB11 | acquisitions |
| 2908 | BIB10 | numérique |
| 2909 | BIB10 | portail |
| 2910 | BIB5 | récolement |
| 2911 | BIB11 | projets, prêts entrants, travaux d'élèves |
| 2912 | BIB13 | RLS stricte, contrôles de conservation |
| 2913–2918 | — | réserve (passe personas, §30) |
| 2919 | — | retrait (réservé, sur décision, E8) |

## 28. Contradictions et points signalés
| # | Point | Proposition |
|---|---|---|
| C-CDI1 | `SOCLES.md` §44 met les **manuels scolaires** au 44 | les manuels passent au 45, le 44 réutilise le moteur (Q-CDI2, appliqué par défaut) ; `SOCLES.md` à corriger à la validation |
| C-CDI2 | `SOCLES.md` : le 45 « dépend de 44 » | inversé : le 44 dépend du moteur du 45 |
| C-CDI3 | Fonction `documentaliste` du 6 proposée au profil `prof` (353) | nouveau modèle de rôle « Documentaliste » (§21.1) ; le profil proposé du 6 reste un point de départ |
| C-CDI4 | `09_INTEROP.md` §15.4 : colonnes `personne_ref JSONB` | contraire à D-036 ; à normaliser par le 9 (hors de ce socle) |

## 29. Réglages signalés (D-023) — S-4500 à S-4512
| # | Réglage | Version automatique |
|---|---|---|
| S-4500 | règles de prêt par catégorie et nature | valeurs par défaut du §10.1 |
| S-4501 | étapes et délais des relances | fixes (§10.5) ; seul le texte d'accompagnement est libre |
| S-4502 | délai de déliement | 30 jours, modifiable par le DPO seulement (plateforme) |
| S-4503 | cote Dewey ou maison | texte libre, aucun réglage |
| S-4504 | jauge par créneau | une jauge par défaut = capacité de la salle du 5 |
| S-4505 | catalogue public sans connexion | désactivé |
| S-4506 | photo d'état des manuels | désactivée |
| S-4507 | prêt sur mobile par l'élève | désactivé jusqu'à BIB9 |
| S-4508 | avis d'élèves | activés, modérés |
| S-4509 | délai d'alerte de non-arrivée | 10 min, fixe |
| S-4510 | boîte de retour à date déclarée | désactivée |
| S-4511 | accusé de réception des manuels | désactivé |
| S-4512 | barème de perte | « remplacement à l'identique » tant qu'aucune délibération |

---

## 30. Ajouts de la passe personas (2026-11-11)

**Ajout personas** (`PERSONAS_45.md`, 58 manques, 4 bloquants regroupés en trois points). **Ce paragraphe l'emporte sur les sections précédentes en cas de contradiction.**

### 30.1 Bloquants appliqués par défaut (D-031)
- **A — R-CDI3 bis** : pour un élève de **moins de 15 ans**, le titre d'un document en retard est visible des responsables qui ont le suivi **dès l'étape 2** (J+21) ; à partir de 15 ans, titre seulement au stade de la perte ; élève majeur : jamais aux parents. L'élève voit dans son portail ce que voient ses parents. Remplace R-CDI3 pour les moins de 15 ans ; §10.5, §21.2 et la nature `cdi.retard` (§23) suivent.
- **B — Mode « suivi » des pertes** (S-4513) tant que les finances sont dans un autre outil (D-034) : montant calculé selon le barème, **avis de somme à payer** (7), liste exportable pour la comptabilité, état `facturee_hors_scolia` avec référence ; CDI-176 ne s'exécute qu'en mode « facturation ScolIA ».
- **C — BIB10 scindé** : **BIB10a** portail complet sans GAR (recherche, sélections, avis, listes, suggestions, accessibilité, catalogue public, ressources libres, lien vers le médiacentre existant) ; **BIB10b** médiacentre servi par le 9, diagnostic GAR, responsables d'affectation (après X11–X13 du 9). Le parcours minimal du CDI ne dépend plus du GAR.

### 30.2 Règles ajoutées
| N° | Règle |
|---|---|
| R-CDI14 bis | Aucune donnée d'élève n'est transmise à un éditeur de ressources hors du GAR (contrat GAR v2026 [V]) ; pas de connecteur direct élève → éditeur. |
| R-CDI31 | Après déliement, un prêt ne garde que le **niveau** et la catégorie (plus la classe) ; `classe_id_a_date` est effacée au déliement. |
| R-CDI32 | L'exemplaire porte `dernier_pret_le` et `nb_prets` (compteurs sans emprunteur) mis à jour au retour ; ils survivent au déliement. |
| R-CDI33 | Un bénévole n'a pas le droit de prêter par défaut ; s'il est habilité au comptoir (décision de la direction, engagement de confidentialité signé), son écran ne montre ni les titres en cours d'un emprunteur ni l'historique. |
| R-CDI34 | La photo de l'élève (si activée au socle 2) s'affiche au comptoir ; un prêt « libre-service » par carte seule est limité à 1 document à la fois. |
| R-CDI35 | Les séances d'enseignement du documentaliste sont signalées au 6 et au 10 avec la pondération réglementaire (×2, décret 2014-940 art. 2 III [V-copie]). |
| R-CDI36 | Aucun profil de besoin (DYS, handicap) n'est stocké au 45 ; les filtres d'accessibilité sont ouverts à tous. |
| R-CDI37 | Le surveillant d'internat habilité ne voit que les internes, pendant la plage de l'internat (41). |
| R-CDI38 | Une AESH ne voit que les prêts et réservations qu'elle a faits pour l'élève qu'elle accompagne. |
| R-CDI39 | Trois réservations en attente au plus par personne. |
| R-CDI40 | Aucun retour en libre-service par l'élève ; un retour est toujours scanné par un agent habilité (comptoir ou boîte). |
| R-CDI8 (étendue) | Aucune relance ni rappel le soir, le week-end et pendant les vacances (cadre de référence du 7-9-2026 [V-indirect]) ; une échéance tombe sur la **prochaine ouverture** du CDI. |
| R-CDI2 (précisée) | Les co-emprunts sont comptés **au moment du déliement** (paires de notices, sans emprunteur), puis le lien est supprimé ; le DPO peut porter le délai à 4 mois au plus (ancienne norme simplifiée n° 9). |

### 30.3 Actions ajoutées (CDI-271 à CDI-292)
| N° | Action | Acteur · droit | Règles | Conf. | Événement | Lot |
|---|---|---|---|---|---|---|
| CDI-271 | Rédiger et versionner la politique documentaire ; la proposer au CA (46) | DOC, DIR | délibération exécutoire lue (46) | N2 | `instances.point_propose` | BIB11 |
| CDI-272 | Importer des statistiques agrégées de N-1 (prêts par notice, année, niveau) | DOC | aucun emprunteur | N2 | — | BIB5 |
| CDI-273 | Rediriger les anciennes références de notices (liens e-sidoc) | SYS | table de correspondance | — | — | BIB5 |
| CDI-274 | Réserver des places d'accueil pendant une séance (jauge commune) | DOC | jauge = capacité − séance | N1 | — | BIB6 |
| CDI-275 | Proposer les ouvertures depuis les jours de présence du 6 | SYS → DOC | — | N1 | — | BIB6 |
| CDI-276 | Rappeler à l'AED prévu de se déclarer responsable présent | SYS | — | — | — | BIB6 |
| CDI-277 | Appeler un adulte (bouton « besoin d'aide ») depuis le CDI | DOC, AEDC | vers la vie scolaire | N1 | `cdi.aide_demandee` | BIB6 |
| CDI-278 | Réserver salle et série en un geste | PROF | 19 + 45 | N1 | — | BIB6 |
| CDI-279 | Transférer une série d'une classe à une autre | PROF, DOC | manquants rattachés avant | N1 | — | BIB4 |
| CDI-280 | Alerter d'un exemplaire de série chez un élève qui a quitté le cours | SYS | — | — | — | BIB4 |
| CDI-281 | Remonter présents et attendus à la vie scolaire à une fermeture subite | SYS | — | — | `cdi.ferme` | BIB6 |
| CDI-282 | Remettre un reçu de retour (ticket ou notification) | SYS | option de l'élève | — | — | BIB2 |
| CDI-283 | Campagne de renouvellement des manuels d'un niveau | DOC, DIR | — | N2 | — | BIB8 |
| CDI-284 | Produire l'avis de somme à payer et l'export des pertes (mode suivi) | DOC, CPT | S-4513 | N2 | `cdi.perte_a_facturer` | BIB7 |
| CDI-285 | Recueillir l'engagement de confidentialité d'un bénévole | DIR (7) | avant habilitation | N2 | — | BIB2 |
| CDI-286 | Proposer le refus d'un avis qui nomme un élève | SYS | détection serveur | — | — | BIB10a |
| CDI-287 | Alerter sur une consultation nominative massive | SYS → DIR | > 30 par heure hors prêt | — | `securite.alerte` | BIB2 |
| CDI-288 | Générer la fiche de registre du traitement « CDI » | DIR, DPO | — | N1 | — | BIB13 |
| CDI-289 | Publier les mentions d'information (familles, élèves, en mots simples) | DIR | — | N1 | — | BIB3 |
| CDI-290 | Reprendre les prêts de l'été des personnels | DOC | fenêtre de reprise | N2 | — | BIB5 |
| CDI-291 | Guide « première semaine » (documentaliste, AED) | SYS | — | — | — | BIB5 |
| CDI-292 | Garder l'ancien SIGB en lecture jusqu'à une date (retour arrière) | DIR | décision | N2 | — | BIB5 |

**Total après la passe : 193 actions** (CDI-100 à CDI-292).

### 30.4 Modèle de données (dans la plage 2900–2919)
`ent_exemplaire` + `dernier_pret_le DATE NULL`, `nb_prets INT DEFAULT 0` (2901) ; `ent_pret_perte.decision` + `facturee_hors_scolia`, colonne `reference_externe TEXT NULL` (2903) ; `ent_cdi_coemprunt (etablissement_id, notice_a_id, notice_b_id, annee INT, compte INT)` (2903) ; `ent_cdi_stat_reprise (etablissement_id, notice_id, annee INT, niveau, prets INT)` (2911) ; `ent_cdi_redirection (etablissement_id, ancienne_ref TEXT, notice_id)` (2911) ; `ent_cdi_politique_documentaire (id, etablissement_id, version INT, objet_id, deliberation_ref UUID NULL)` (2911) ; `ent_cdi_engagement_benevole (etablissement_id, personne_id, piece_id, signe_le)` (2902). Aucun JSON.

### 30.5 Cas limites ajoutés
CCD-49 à CCD-62 : `PERSONAS_45.md` §4 (repris tels quels).

### 30.6 Lots révisés
BIB10 → **BIB10a** (portail sans GAR, peut suivre BIB3) et **BIB10b** (médiacentre, après le 9 X11–X13). **Parcours d'usage réel du pilote (rentrée 2027)** : BIB0–BIB7 (BIB7 en mode « suivi ») + BIB10a ; collège : + BIB8.

---

## 31. Questions et décisions

### 31.1 Questions (Q-CDI) et recommandation appliquée par défaut (D-031)
| N° | Question | Recommandation | État |
|---|---|---|---|
| Q-CDI1 | Quel SIGB et quel format d'export dans le pilote ? | fournir un export anonymisé avant BIB5 | **à Florian** (fait que lui seul connaît) |
| Q-CDI2 | Manuels scolaires au 45 ou au 44 ? | au 45 ; le 44 réutilise le moteur d'exemplaires et de prêt | appliqué par défaut |
| Q-CDI3 | Livrer un thésaurus et une classification ? | non (licences Motbis et Dewey [NV]) ; import par l'établissement | appliqué par défaut |
| Q-CDI4 | Prêt sur mobile par l'élève ? | oui, en attente de validation, lot BIB9 | appliqué par défaut |
| Q-CDI5 | Un parent peut-il connaître les titres en cours ? | sous 15 ans oui (R-CDI3 bis) ; au-delà, décision du documentaliste sur demande, prêts en cours seulement | appliqué par défaut ; avis du DPO demandé |
| Q-CDI6 | Durée de déliement | 30 jours (≤ 4 mois) | appliqué par défaut ; **avis du DPO** |
| Q-CDI7 | Relecture juridique des [NV] (Légifrance bloqué au client automatisé) | relecture humaine avant BIB7, BIB8, BIB13 | à faire |
| Q-CDI8 | Manuels du collège privé sous contrat : qui les finance ? | paramètre « financeur » ; aucun calcul de dotation | appliqué par défaut ; fait à confirmer par Florian |
| Q-CDI9 | Catalogue public sans connexion | désactivé par défaut | appliqué par défaut |
| Q-CDI10 | Exception handicap (fichiers adaptés) | aucun fichier adapté protégé diffusé par le 45 | appliqué par défaut |
| Q-CDI11 | Mode des pertes pendant le doublon | « suivi » | appliqué par défaut |
| Q-CDI12 | CDI du pilote en usage réel à la rentrée 2027 ? | oui, après recette de BIB0–BIB7 | **à Florian** (données réelles, accord de l'établissement) |
| Q-CDI13 | Accès aux ressources payantes pendant la période sans GAR | garder l'accès actuel (ENT ou portail existant) | **à Florian** (contrats éditeurs) |
| Q-CDI14 | Bénévoles au comptoir | non par défaut ; sur décision et engagement | appliqué par défaut |

### 31.2 Ce qui engage Florian
Export réel du SIGB (anonymisé pour les essais, réel après l'accord écrit de l'établissement et l'avis du DPO) ; date d'usage réel ; contrats et abonnements des éditeurs ; avis du DPO (durées, âge de 15 ans) ; barème des pertes à faire voter (CA ou organisme) ; relecture juridique.

## 32. Faits, hypothèses, recommandations
- **Faits [F]** : aucun code ni module CDI dans ScolIA (Rust et Next.js) ; fonction `documentaliste` au 6 ; tuiles prévues au 29 ; GAR et responsables d'affectation au 9 ; réservation de lieux au 19 ; recherche tolérante disponible (290).
- **Hypothèses [H]** : SIGB du pilote (BCDI/e-sidoc ou PMB) ; pratiques de relance et de manuels ; bénévoles dans le privé ; CDI ouvert le soir pour l'internat ; année du CDI (§4).
- **Recommandations [R]** : toutes les règles R-CDI ; moteur commun avec le 44 ; usage réel du CDI dès BIB7 ; pas de lot 0 (rien à corriger).
