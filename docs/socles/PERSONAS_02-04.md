# Passe « personas » sur les socles 2, 3 et 4

**Date :** 2026-11-06 — **Statut :** proposition, **non décidée** (à relire par Florian). Aucun code écrit ni modifié ; aucun accès à la base ni aux secrets de production ; l'application Next.js n'a pas été lue pour ce document.
**Branche :** `ent-personas-socles-2-4`.
**Éprouve :** `docs/socles/02_ELEVE.md`, `03_INSCRIPTIONS.md`, `04_ADMINISTRATIF.md` (lus en entier), avec `05_STRUCTURE.md` et `06_PERSONNELS.md` en survol, `SOCLES.md`, `DECISIONS.md` (D-017, D-019 à D-022), `MIGRATIONS.md`.
**Ajouts :** faits **directement** dans les trois conceptions, marqués **[personas]** (actions `ELV-29` à `ELV-48`, `INS-53` à `INS-84`, `ADM-71` à `ADM-102`, règles `R-E*`, `R-I*`, `R-A*`, cas `C28` à `C47`, questions `Q-E18+`, `Q-I21+`, `Q-A21+`).
**Mise à jour du même jour (D-023, D-024)** : Florian vise d'abord une couverture **au moins égale à Charlemagne**, plus les actions réelles du terrain, et des **modèles de documents prêts** (3 ou 4 trames par type) plutôt qu'une mise en forme libre. La passe intègre ce regard : manques de couverture et de terrain (M-133 à M-138, dont une lecture ciblée de l'application Next.js), **réglages signalés** comme candidats à l'allègement sans rien supprimer (§5 bis.2), documents ramenés à des trames (§5 bis.3). Ce document dit **d'où vient** chaque ajout.

---

## 0. À lire en premier : ce qui change pour les lots en cours de code

Le lot **E2** (socle 2, identité versionnée) et le lot **A0** (socle 4, correctifs et ports pour le socle 3) sont en cours. Cinq ajouts les touchent. Les autres ajouts sont **additifs** et peuvent attendre leur lot.

### Lot E2 (socle 2) — trois changements, deux recommandés

| # | Changement | Pourquoi | Ce que E2 doit faire | Gravité |
|---|---|---|---|---|
| E2-a | **Date de naissance incomplète ou présumée** : deux colonnes dans `ent_eleve_identite` (et la projection sur `ent_eleve`) : `date_naissance_precision` (`jour`, `mois`, `annee`, `inconnue`) et `date_naissance_statut` (`declaree`, `justifiee`, `presumee`, `contestee`) | Enfant arrivé d'un pays sans état civil fiable, mineur non accompagné dont la minorité est contestée : aujourd'hui on inventerait un « 1ᵉʳ janvier » indiscernable d'une vraie date (persona P11) | **Ajouter les deux colonnes dans la migration de E2** (reprise : `jour` / `declaree` pour tous). Les ajouter plus tard obligerait à reprendre toutes les versions | bloquant |
| E2-b | **Motif codé** : `motif_code` (liste du logiciel) en plus du `motif` libre ; la contrainte « motif obligatoire » porte sur le code | Sans code, une correction en lot (casse, accents après un import) exige 600 motifs tapés ; les statistiques d'erreurs sont impossibles (P01) | **Changer la contrainte dans E2** : `motif_code NOT NULL` pour `correction` et `changement_etat_civil` ; texte libre obligatoire seulement pour le code `autre` | important |
| E2-c | **Sensibilité forcée** : les motifs `adoption`, `changement_prenom`, `changement_mention_sexe`, `protection` posent `sensible = true`, non modifiable par l'opérateur | Une adoption plénière ou un changement de prénom ne doivent jamais dépendre de la vigilance de la personne qui saisit (P15) | Règle de service dans E2 (ELV-04/05) + test | important |
| E2-d (recommandé) | Colonnes facultatives `nom_ecriture_origine`, `prenoms_ecriture_origine` dans la version | Graphie d'origine (arabe, cyrillique, dari) utile pour rapprocher des pièces (P11) | Ajouter maintenant (nullable, aucun coût) ou décider explicitement de différer | important |
| E2-e (aucun changement) | Identifiants de connexion qui suivent le prénom d'usage (ELV-47) | — | Rien dans E2 : l'événement `eleve.identity_changed` porte déjà les noms des champs changés ; le socle 1 réagira | — |

### Lot A0 (socle 4) — quatre changements

| # | Changement | Pourquoi | Ce que A0 doit faire | Gravité |
|---|---|---|---|---|
| A0-a | **Fuseau de l'établissement**, pas « Europe/Paris » codé en dur | La Réunion, la Guyane, Mayotte, les établissements français à l'étranger : un lien qui finit le 31 août resterait actif des heures de trop, ou s'ouvrirait trop tôt (P02) | La vue `ent_eleve_lien_actif` (migration 260) lit le fuseau du paramètre d'établissement (socle 5, STR-03), **Europe/Paris par défaut** ; test de bord de journée avec deux fuseaux | important |
| A0-b | **Contrat `DeclarationResponsables` versionné avec des champs réservés** : `adresse_a_proteger`, `ne_pas_solliciter` (+ référence scellée), `langue`, `besoin_interprete`, `organisme` (ASE comme titulaire), `nature_code` (vocabulaire du socle 4) | Une mère protégée par une ordonnance déclare sa situation dès la préinscription (P14) ; une famille ASE inscrit un enfant placé (P12) | Le type porte ces champs **dès A0** (facultatifs). Sur le schéma actuel, ce qui ne peut pas être stocké de façon sûre **n'est jamais écrit en clair** : une adresse à protéger n'entre pas dans `ent_personne.adresse` ; elle est **parquée** pour la direction (alerte bloquante d'information) jusqu'au lot A3 | bloquant |
| A0-c | **« Ne pas solliciter »** | Envoyer une demande d'accord ou une invitation de compte au parent violent lui révèle l'établissement de l'enfant (P14) | `integrer_declaration` ne déclenche **aucune** invitation vers une personne marquée `ne_pas_solliciter` ; même règle dans le lot **I0** du socle 3 pour la demande d'accord (à signaler aussi à l'équipe I0) | bloquant |
| A0-d | **Confirmation du lien au guichet, ou en un geste depuis un compte existant** | A0 rend le lien vers une personne qui a déjà un compte **inactif** jusqu'à sa confirmation. Sans autre voie que l'invitation par e-mail, des parents légitimes resteraient bloqués à la rentrée (e-mail perdu, parent qui ne lit pas ses courriels, professeur-parent) (P01, P19) | Deux voies de confirmation en plus de l'invitation : (1) le parent connecté voit « Vous êtes déclaré responsable de X : confirmer » ; (2) le secrétariat confirme au guichet après vérification d'une pièce d'identité (type de pièce et auteur tracés, jamais la copie) | bloquant |

**Lot I0 (socle 3)**, non demandé mais touché : la règle A0-c vaut pour la **demande d'accord du second parent** et pour les relances ; à transmettre à l'équipe I0 avant la fusion de S2-3.

---

## 1. Méthode

**Démarche.** 23 personas réalistes et variées, choisies pour couvrir : public, privé sous contrat, hors contrat ; collège, lycée, BTS ; métropole et outre-mer ; personnels de bureau, de terrain et d'encadrement ; familles ordinaires, séparées, recomposées, non francophones, placées, en danger ; l'élève lui-même, mineur et majeur. Pour chacune : sa journée, sa semaine, son année mois par mois (rentrée, conseils, réinscriptions, examens, bascule), ses incidents (erreur, contestation, départ, contrôle, panne). Chaque fois qu'elle voulait faire quelque chose que les conceptions ne prévoient pas, ou que c'était trop long, ambigu, risqué ou frustrant, le manque est noté.

**Gravité.**
- **bloquant** : sans lui, l'établissement contourne de façon dangereuse (sécurité d'un enfant, fuite d'une donnée protégée, pratique probablement illégale) ou ne peut pas saisir un cas réel.
- **important** : cas fréquent sans solution propre, perte de temps forte ou risque d'erreur.
- **confort** : améliore nettement le quotidien ; un contournement existe.
- **différenciant** : peu de logiciels du marché le font [H, comparaison de mémoire] ; argument de vente.

**Conventions.** [H] hypothèse ; [NV, à vérifier] règle réglementaire citée de mémoire, à vérifier sur un texte officiel avant de coder ; [R] recommandation. Les numéros de socles renvoient à `SOCLES.md`.

**Limites.** Les personas sont construites, pas interviewées : elles complètent, sans la remplacer, la relecture par des utilisateurs réels (voir §6). Les manques des socles ultérieurs sont notés sans être conçus.

---

## 2. Les personas en un tableau

| # | Persona | Établissement | Ce qu'elle éprouve surtout |
|---|---|---|---|
| P01 | **Sylvie**, secrétaire de scolarité, seule avec une mi-temps | Collège privé sous contrat, 650 élèves | Saison d'inscription, réinscriptions, rentrée, erreurs, août |
| P02 | **Marc**, proviseur | Lycée public, 1 400 élèves, BTS ; ancien poste à La Réunion | Affelnet, examens, conseil d'administration, fuseau |
| P03 | **Nadia**, CPE | Collège public REP, 500 élèves | Appels aux familles, fugue, situations qui changent sans prévenir |
| P04 | **Thomas**, professeur principal de 3ᵉ | Collège privé | Connaître sa classe, rentrée, réinscriptions |
| P05 | **Julie**, contractuelle de lettres arrivée le 18 novembre | Lycée public | Premier jour, 140 élèves inconnus |
| P06 | **Josiane**, agent d'accueil (collectivité) | Collège public | 16 h 30, la loge, les remises d'enfants |
| P07 | **Pierre**, comptable de l'OGEC | Ensemble scolaire privé (école, collège, lycée) | Payeurs, fratries, réductions |
| P08 | **Claire**, infirmière | Cité scolaire | Joindre vite, remettre un enfant malade |
| P09 | **Karim et Émilie**, parents séparés, garde alternée semaine paire / impaire | Deux enfants en 6ᵉ et 4ᵉ | Fiche de rentrée, absences, factures, désaccords |
| P10 | **Monique**, 74 ans, grand-mère, sans smartphone | Vient chercher son petit-fils le mercredi | Être reconnue, rester à jour |
| P11 | **Famille Ahmadi**, arrivée d'Afghanistan en janvier, parle dari | Collège public, UPE2A | Identité incertaine, langue, hébergement d'urgence |
| P12 | **Mme Leroy**, référente de l'aide sociale à l'enfance ; **Sandrine**, assistante familiale | Enfant placé en 5ᵉ | Qui décide, qui signe, placement en urgence |
| P13 | **Léa**, terminale, 18 ans le 14 février | Lycée public | Majorité au milieu de l'année |
| P14 | **Inès**, mère protégée par une ordonnance de protection | Inscription en 4ᵉ dans le privé après un déménagement | Ne pas être retrouvée |
| P15 | **Sam**, 15 ans, demande un prénom d'usage | Lycée public | Dignité, discrétion, examens |
| P16 | **Hélène**, directrice d'un établissement hors contrat bilingue, 120 élèves | Hors contrat | Inspection, admissions toute l'année, familles expatriées |
| P17 | **Famille Morel**, militaire, mutée deux fois en trois ans | Collège public puis privé | Départ en trois jours, arrivée en juillet, année à l'étranger |
| P18 | **Yanis**, 20 ans, étudiant de BTS en apprentissage | Lycée privé avec BTS | Aucun responsable, il paie lui-même |
| P19 | **Bruno**, professeur et parent d'élève ; **Carole**, secrétaire, mère séparée d'un élève | Même établissement que leur enfant | Conflit d'intérêts |
| P20 | **Kevin**, assistant d'éducation (grille, internat) | Lycée avec internat | Badges, photos, erreurs vues sur le terrain |
| P21 | **M. Duval**, agent comptable | Lycée public | Régimes et forfaits |
| P22 | **La direction face à un deuil** (décès d'un élève en avril) | Collège | Que rien d'automatique ne parte |
| P23 | **Le délégué à la protection des données** du réseau ; **un père** qui exerce son droit d'accès | Privé sous contrat | Demandes d'accès, de rectification, d'opposition |

---

## 3. Parcours et manques, persona par persona

Chaque tableau donne : le manque, pourquoi il compte, sa gravité, le socle concerné, et l'ajout fait (identifiant dans la conception) ou le socle ultérieur où il est renvoyé.

### P01 — Sylvie, secrétaire de scolarité (collège privé sous contrat)

**Journée type (février).** 7 h 50 : 14 courriels de familles (« avez-vous reçu mon dossier ? »), 6 pièces déposées à contrôler, un père au téléphone qui a changé d'adresse e-mail et ne reçoit plus les codes. 10 h : deux parents séparés ont déposé chacun un dossier pour le même enfant. 14 h : la direction veut savoir combien de places restent en 6ᵉ bilangue. 16 h : une famille apporte un dossier papier de 23 pages.
**Année.** Septembre : 650 fiches de rentrée, dont 80 sur papier ; corrections d'état civil venues de l'import. Octobre : remontée vers SIECLE et ses rejets. Décembre–mars : portes ouvertes, candidatures, commission, décisions. Février–avril : réinscriptions. Juin : départs de 3ᵉ, certificats de radiation, dossiers scolaires à transmettre. Juillet : désistements, liste d'attente, changements d'option. Août : secrétariat fermé trois semaines, mais les délais courent.
**Ce qu'elle vit avec son logiciel actuel (D-023).** Des courriers de décision et des certificats bricolés dans un traitement de texte, refaits chaque année ; des dizaines d'écrans de paramétrage qu'elle n'ouvre jamais ; un tableur partagé pour ce que le logiciel ne prévoit pas (les appels de parents reçus à l'accueil). Ce qu'elle veut : que les documents sortent tout seuls, propres, des données ; qu'on ne lui demande de régler que ce qui est propre à son établissement.
**Incidents.** Elle valide par erreur l'inscription d'Emma pour la mauvaise année ; la plateforme est lente le dernier jour des réinscriptions ; des jumeaux apparaissent chaque nuit comme « doublons probables » ; l'établissement passe à ScolIA en janvier, en pleine saison.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-001 | Modifier une candidature déjà déposée (vœux, informations, pièce oubliée) avant la décision | Aujourd'hui seul le retrait existe : la famille retire et redépose, perd son numéro et sa date de dépôt | important | 3 | INS-55 |
| M-002 | Changer l'e-mail du déposant, ou transférer la démarche à l'autre parent | Un parent qui perd l'accès à sa boîte aux lettres en pleine campagne est bloqué ; le secrétariat n'a aucun geste prévu | important | 3 | INS-56 |
| M-003 | Numériser un dossier papier reçu par courrier et le découper en pièces typées | Les familles âgées ou peu connectées envoient encore du papier ; sans outil, chaque page est déposée à la main | confort | 3 | INS-59 |
| M-004 | Publier toutes les décisions d'une commission à la même heure | Des décisions qui tombent au fil de l'eau créent des appels en cascade (« mon voisin a eu sa réponse, pas moi ») | différenciant | 3 | INS-62 |
| M-005 | Suspendre les délais et les relances pendant la fermeture du secrétariat | Un délai de réponse de 8 jours qui expire le 10 août fait perdre une place sans que personne ne puisse répondre | important | 3 (+5 calendrier) | INS-80, R-I6 |
| M-006 | Prolonger une échéance pour toute une campagne en un geste (panne, grève postale, intempérie) | Le jour de la date limite, une panne de trois heures ; aujourd'hui il faudrait rouvrir chaque dossier | important | 3 | INS-79 |
| M-007 | Annuler une validation faite par erreur (mauvais enfant, mauvaise année, doublon) | La validation crée un élève, des invitations, une facture ; le désistement n'est pas fait pour une erreur | important | 3 | INS-68, R-I11 |
| M-008 | Délivrer une attestation d'inscription pour l'année à venir | Employeur, bailleur, consulat, caisse d'allocations la demandent dès mai ; c'est le document le plus demandé au guichet en été [H] | important | 3 (+7) | INS-69 |
| M-009 | Émettre les certificats de radiation en lot à la fin de l'année | 120 élèves de 3ᵉ partent ; un par un, c'est une demi-journée | confort | 3 | INS-73 |
| M-010 | Transmettre le dossier scolaire à l'établissement d'accueil, et répondre à une demande reçue | Le flux existe sur le papier (`03` §10.1) mais aucune action ne le porte ; on ne sait pas ce qui a été envoyé, à qui | important | 3 (+7) | INS-74, INS-75 |
| M-011 | Déclarer deux fiches comme distinctes (jumeaux, homonymes) une fois pour toutes | La détection nocturne remonte les mêmes jumeaux chaque nuit : bruit, puis plus personne ne regarde | important | 2 | ELV-33, R-E20 |
| M-012 | Corriger en lot la typographie (majuscules, accents, espaces) après un import | Un import en majuscules sans accents : 650 corrections une à une, chacune avec motif | confort | 2 | ELV-31, E2-b |
| M-013 | Fiches de rentrée papier pré-remplies, et saisie des retours par différences | 10 à 15 % des familles ne rempliront rien en ligne [H] ; sans cela, le secrétariat ressaisit tout | important | 4 | ADM-95 |
| M-014 | Relancer de façon ciblée (pas ouvert, commencé sans finir, un seul parent a répondu) | Une relance uniforme agace ceux qui ont presque fini et ne touche pas ceux qui n'ont rien vu | confort | 3 | INS-72 |
| M-015 | Vérifier une priorité déclarée (frère déjà scolarisé, enfant du personnel) | La famille coche « fratrie » ; le secrétariat vérifie à la main dans un autre écran | confort | 3 | INS-82 |
| M-016 | Lier les décisions de jumeaux (« les deux ou aucun ») | Admettre un jumeau et refuser l'autre est presque toujours une erreur, et un drame familial | important | 3 | INS-64, R-I14 |
| M-017 | Changer d'option entre la validation et la rentrée | En juillet, la famille renonce au latin ; rien ne le prévoit entre le socle 3 (vœux) et le socle 5 (enseignements suivis) | important | 3 (+5) | INS-70 |
| M-018 | Reprendre des dossiers d'inscription en cours depuis l'ancien logiciel, dans leur état réel | Un établissement qui bascule en janvier a 200 dossiers ouverts ; les recréer à zéro renverrait des accusés de réception | important | 3 (+28) | INS-81, R-I26 |

### P02 — Marc, proviseur (lycée public)

**Semaine type.** Lundi : réunion de direction, tableau des effectifs. Mardi : conseil pédagogique. Jeudi : rendez-vous avec une famille qui conteste un refus de changement de régime. Vendredi : remontée d'enquête au rectorat.
**Année.** Juin–juillet : résultats Affelnet, inscriptions des entrants en quelques jours, puis non-inscrits à signaler. Septembre : constat de rentrée. Novembre : conseil d'administration (indicateurs sociaux). Janvier–mars : inscriptions aux examens, vérification des noms qui figureront sur les diplômes. Toute l'année : régime de demi-pension au forfait.
**Ce qu'il vit (D-023).** Il ne veut pas « concevoir » un courrier de refus ni une attestation : il veut choisir une trame soignée une fois, y mettre son logo et sa signature, et ne plus y penser. Il veut aussi retrouver tout ce que faisait l'ancien logiciel, sans ses réglages accumulés.
**Incidents.** Une liste de classe avec adresses circule sur un réseau social ; un ancien élève demande une attestation de scolarité de 2019 ; dans son poste précédent à La Réunion, les liens et les majorités basculaient à la mauvaise heure dans l'ancien logiciel.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-019 | Signaler à l'administration les affectés non inscrits et les places vacantes | Dans le public, les places non prises doivent être réattribuées vite [NV, à vérifier] ; aujourd'hui, liste faite à la main | important | 3 | INS-77 |
| M-020 | Faire vérifier l'état civil par les familles avant les inscriptions aux examens | Une faute dans un prénom se retrouve sur le diplôme ; la corriger après coup est long [H] | différenciant | 2 (+38) | ELV-32 |
| M-021 | Changements de régime limités à des dates autorisées, avec dérogation motivée | Dans le public, la demi-pension est souvent au forfait trimestriel [H] ; un changement au 12 novembre casse la facturation | important | 2 (+24) | R-E5 |
| M-022 | Filigrane nominatif sur toute liste exportée ou imprimée | Une liste qui fuit doit pouvoir être rattachée à son auteur ; et dissuader de la diffuser | différenciant | 2, 4 (+7) | R-E7 |
| M-023 | Indicateurs sociaux (catégories socioprofessionnelles) pour le conseil d'administration, sans exposer les familles | La collecte est prévue (`04` §8.1), son usage agrégé ne l'est pas ; un petit effectif rend une case identifiante | confort | 4 (+47) | R-A37 |
| M-024 | Arrivées d'été pour l'année suivante (après la clôture des admissions) | Les mutations de juillet arrivent quand aucune campagne n'est ouverte pour N+1 | important | 3 | R-I5 |
| M-025 | Départ vers l'instruction en famille avec sa référence | L'instruction en famille est soumise à autorisation préalable depuis 2022 [NV, à vérifier] ; un départ sans référence doit alerter | important | 3 (+12) | INS-76 |
| M-026 | Enregistrer et instruire un recours contre une décision | Une famille conteste un refus ; aujourd'hui seule « revoir une décision » existe, sans trace de la demande ni échéance de réponse | important | 3 | INS-66 |
| M-027 | Fuseau horaire de l'établissement pour toutes les dates métier (liens, majorité, échéances) | « Europe/Paris » est écrit en dur dans `04` et `02` ; faux pour l'outre-mer et l'étranger | important | 2, 3, 4 (+5) | R-A38, R-E15, A0-a |
| M-028 | Attestation de scolarité pour une période passée (ancien élève) | Demande fréquente (concours, dossiers administratifs) ; elle exige de lire des scolarités archivées | important | 2 (+7) | ELV-38 |

### P03 — Nadia, CPE (collège public REP)

**Journée type.** 8 h 15 : 22 absences à traiter, 9 appels à passer. Le numéro de la mère de Rayan « n'est pas attribué ». La mère de Dilan ne parle que le turc ; c'est Dilan, 12 ans, qui traduit d'habitude. 11 h : Inès (4ᵉ) dit qu'elle vit chez son père depuis un mois. 15 h : Kenzo n'est pas rentré de la cantine ; sa mère dit qu'il a fugué.
**Incidents.** Fugue : il faut toutes les adresses connues et tous les contacts, maintenant ; or la CPE n'a pas le droit de voir les adresses (`04` §13 : F2 non accordé au CPE). Deux élèves dont l'un est sous contrôle judiciaire avec interdiction d'entrer en contact avec l'autre.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-029 | Signaler en un clic une coordonnée erronée depuis l'appel | Le CPE est le premier à savoir qu'un numéro est faux ; aujourd'hui seul le secrétariat ou le fournisseur peut le dire | important | 4 | ADM-75 |
| M-030 | Relais de communication (interprète familial, médiateur) distinct d'un responsable, et alerte « pas d'enfant interprète » | Faire traduire une absence ou une sanction par l'élève lui-même est courant et nuisible [H] | important | 4 (+8) | ADM-85, R-A6 |
| M-031 | Accès exceptionnel motivé (« bris de glace ») aux adresses et à la situation d'un élève en danger | Fugue, disparition, malaise grave : il faut lire ce que le rôle n'ouvre pas, avec trace et contrôle après coup | important | 2, 4 (+1, 36) | ELV-44, ADM-98, R-E11 |
| M-032 | Signaler un changement de situation constaté par un personnel | « Il vit chez son père depuis un mois » : la CPE le sait, le dossier non ; aucune source « personnel » dans les propositions | important | 4 | ADM-97 |
| M-033 | Résidence temporaire (hébergé chez sa grand-mère pendant une hospitalisation) | Les modes actuels (principale, alternée, secondaire) ne disent pas « pour trois semaines » | confort | 4 | R-A4 |
| M-034 | Famille injoignable durablement (aucun numéro valide depuis trois semaines) | Signal faible de décrochage ou de danger ; devrait remonter au CPE et à l'assistante sociale | important | 12, 36 | renvoi |
| M-035 | Interdiction de contact entre deux élèves (contrôle judiciaire, mesure éducative) | Doit peser sur la constitution des classes, les groupes, les sorties | important | 13, 5 | renvoi |

### P04 — Thomas, professeur principal de 3ᵉ (collège privé)

**Rentrée.** Il découvre 27 noms, dont 5 nouveaux et 2 redoublants qu'il ne connaît pas ; deux familles séparées à convoquer séparément.
**Année.** Réinscriptions en février (lecture de l'état de sa classe, Q-I14) ; orientation ; sortie de fin d'année avec la liste des contacts d'urgence.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-036 | Voir les arrivées, départs et changements de classe récents de mes classes et groupes | Le professeur apprend qu'un élève est parti quand il fait l'appel | confort | 2 | ELV-45 |
| M-037 | Pastilles « nouvel arrivant », « redoublant », « arrivé en cours d'année » | Information pédagogique de base, aujourd'hui dispersée | confort | 2 | R-E9 |
| M-038 | Suivre la complétude de rentrée de sa classe (fiche de rentrée, assurance, photo, autorisations, comptes activés), sans voir le contenu | Le PP relance mieux que le secrétariat, s'il sait qui n'a rien fait | confort | 3, 4 | INS-78 |
| M-039 | Liste de sortie hors ligne avec contacts d'urgence et autorisés | Car de 50 élèves, zone sans réseau | important | 42 (+4) | renvoi (ADM-91 en fournit la base) |

### P05 — Julie, contractuelle arrivée le 18 novembre (lycée public)

**Premier jour.** 140 élèves sur 5 classes ; elle ne sait ni les prononcer ni les reconnaître ; deux « Lucas M. » en 1ʳᵉ B ; elle ne sait pas qui a un aménagement.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-040 | Prononciation du prénom et du nom, indiquée par l'élève ou la famille | Écorcher un prénom à chaque appel blesse ; un remplaçant n'a personne à qui demander | différenciant | 2 | ELV-43 |
| M-041 | Désambiguïser deux élèves de même désignation dans une liste | Deux « Lucas M. » : erreurs d'appel et de notes | confort | 2 | R-E8 |
| M-042 | Points d'attention pédagogiques (PAP, aménagements) visibles dès le premier cours | Le socle 2 affiche les pastilles ; leur contenu relève du socle 35 | important | 35 | renvoi |
| M-043 | Droits ouverts dès la prise de poste, avant le premier cours | Sinon elle fait l'appel sur papier les deux premiers jours | important | 6 | renvoi |

### P06 — Josiane, agent d'accueil (collège public)

**16 h 30, un mercredi.** Une dame âgée vient chercher Léo ; elle n'a pas sa carte d'identité. Un homme dit être le père de Lina ; aucun père n'est déclaré. À 14 h, une mère a téléphoné : « mon frère viendra chercher Noah à 15 h ». À 15 h, la direction a posé une interdiction de remise sur un parent. À 16 h 35, le réseau tombe.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-044 | Demander à distance l'accord d'un titulaire pour une remise non prévue | Remplace dix minutes d'appels sans réponse par une notification « oui / non » tracée | différenciant | 4 (+14, 11) | ADM-87, R-A12 |
| M-045 | Consigne téléphonique de remise, confirmée par rappel sur un numéro du dossier | Une consigne téléphonique est la porte ouverte à l'usurpation | important | 4 (+14) | ADM-88, R-A13 |
| M-046 | Procédure pour une personne inconnue qui se dit parent | Ne pas remettre, tracer, et ouvrir une vérification (un vrai père non déclaré existe) | important | 4 (+14) | ADM-89, R-A14 |
| M-047 | Fiche de secours imprimable ou disponible hors ligne (remises et interdictions du jour) | Une panne à 16 h 30 ne doit pas faire remettre un enfant à une personne interdite | important | 4 (+11, 14) | ADM-91, R-A11 |
| M-048 | Une interdiction posée doit apparaître sur tous les postes et téléphones en moins d'une minute | Une interdiction posée à 15 h et invisible à 16 h 30 sur un téléphone en cache est la pire défaillance du domaine | bloquant | 4 (+11) | R-A10 |
| M-049 | Carte de retrait (QR) pour les personnes autorisées | Reconnaître en deux secondes une personne autorisée, sans lire de liste | différenciant | 4 (+14) | ADM-92, R-A15 |

### P07 — Pierre, comptable de l'OGEC (ensemble scolaire)

**Rentrée.** Réduction « fratrie » : trois enfants dont un demi-frère qui vit chez son père ; deux enfants du personnel ; une cantine que les parents séparés veulent payer « chacun ses semaines ». **Février.** Un père écrit : « à partir de mars je ne paie plus la cantine ».

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-050 | Une définition explicite de la fratrie, selon l'usage (tarif, priorité, envois) | Le socle 4 n'a pas de « fratrie » : chaque socle la recalculerait autrement ; la réduction serait contestable | important | 4 (+24) | ADM-94, R-A22 |
| M-051 | Savoir qu'un élève est enfant du personnel (priorité, tarif) | Le lien existe entre socles 4 et 6, aucun port ne l'expose | confort | 4, 6 (+24) | R-A23 |
| M-052 | Répartition des frais de cantine « au foyer du jour » | En garde alternée, chaque parent veut payer les repas de ses semaines ; un pourcentage fixe est faux une semaine sur deux | différenciant | 4 (+24) | ADM-93, R-A21 |
| M-053 | Un payeur seul ne peut pas changer une répartition qui engage l'autre | « Je ne paie plus » : sans règle, la comptabilité bascule tout sur l'autre parent | important | 4 | R-A20 |

### P08 — Claire, infirmière (cité scolaire)

**Situation.** Une élève fait un malaise ; il faut joindre un parent, puis la remettre à une personne autorisée ; la mère est au travail jusqu'à 18 h.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-054 | Prévenir tous les joignables en même temps, sans détail médical (« merci de rappeler l'infirmerie ») | Un appel après l'autre fait perdre de longues minutes | confort | 8 (+4) | renvoi |
| M-055 | Contacts médicaux (médecin traitant) | Hors du socle 4 | confort | 34 | renvoi |

(La remise d'un enfant malade est couverte par ADM-44 et ADM-87.)

### P09 — Karim et Émilie, parents séparés en garde alternée

**Rentrée.** Chacun remplit sa fiche : Karim déclare « résidence alternée », Émilie « résidence principale chez moi » ; Karim met sa compagne en personne autorisée, Émilie la retire. **Octobre.** « Cette semaine c'est moi, on a échangé » : le SMS d'absence part chez Émilie. **Décembre.** Karim veut payer les repas de ses semaines. **Mars.** Karim ne comprend pas pourquoi il ne voit plus les notes d'un des enfants (décision prise par la direction au vu d'un jugement).

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-056 | Détecter les déclarations contradictoires des deux titulaires et n'en appliquer aucune automatiquement | Sinon le dernier qui écrit gagne : c'est la source de conflit la plus fréquente [H] | important | 4 | ADM-96, R-A1 |
| M-057 | Échange ponctuel de garde déclaré par un titulaire | Le « foyer du jour » se trompe dès que les parents échangent une semaine | important | 4 | ADM-78, R-A2 |
| M-058 | Partage des vacances dans le rythme | Sorties, internat ou stages pendant les congés : à qui écrire ? | confort | 4 | ADM-79, R-A3 |
| M-059 | Voir l'historique de ce que j'ai moi-même reçu | Preuve en cas de conflit (« je n'ai jamais été prévenu ») | confort | 8 | renvoi |
| M-060 | Expliquer à un parent ses droits, enfant par enfant, et pourquoi | « Pourquoi je ne vois plus les notes ? » : sans réponse dans l'outil, l'appel arrive chez la direction | important | 4 | ADM-86, R-A24 |
| M-061 | Adresse e-mail commune du couple devenu séparé | Une adresse partagée continue d'informer l'autre parent après la séparation | important | 4 | R-A19 |

### P10 — Monique, grand-mère sans smartphone

**Mercredi midi.** Elle vient chercher Léo, autorisée en permanence le mercredi. Elle a changé de téléphone fixe ; elle ne peut rien mettre à jour seule.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-062 | Un titulaire peut proposer la mise à jour d'un contact ou d'une personne autorisée qu'il a déclaré | Aujourd'hui seule la personne elle-même ou le secrétariat peuvent le faire ; la grand-mère n'a pas de compte | important | 4 | ADM-73, R-A33 |
| M-063 | Téléphone fixe du domicile, partagé par le foyer | Modèle par personne seulement : le même fixe saisi trois fois, et oublié au déménagement | confort | 4 | ADM-74, R-A34 |
| M-064 | Être prévenue des absences les jours où elle a l'enfant, si un titulaire le demande | Le mercredi, c'est elle qui l'attend à la sortie | confort | 4 (+8) | R-A35 |

### P11 — Famille Ahmadi, arrivée d'Afghanistan en janvier

**Arrivée.** Évaluation par le CASNAV, affectation en UPE2A [NV, à vérifier]. Le fils, Omid, a une date de naissance connue à l'année seulement ; son nom s'écrit Ahmadi, Ahmady ou Ahmadhi selon les papiers ; il n'a ni INE, ni certificat de radiation, ni acte de naissance traduit. La famille vit dans un hôtel d'hébergement d'urgence, domiciliée par une association. Le père lit le dari et un peu l'anglais.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-065 | Date de naissance incomplète ou présumée, distinguée d'une vraie date | Sans cela, on invente un 1ᵉʳ janvier indiscernable ; majorité, examens, exports se fondent sur une date fausse | bloquant | 2 | ELV-29, R-E1, **E2-a** |
| M-066 | Autres graphies et écriture d'origine du nom | Rapprocher les pièces et retrouver l'élève dans la recherche | important | 2 | ELV-30, R-E2 |
| M-067 | Pièces en langue étrangère, traduction facultative | Exiger une traduction assermentée retarde la scolarisation [H] | confort | 3 (+7) | R-I3 |
| M-068 | Parcours famille (préinscription, Mes démarches, accord, réinscription) dans la langue de la famille | Un formulaire en français seulement exclut une partie des familles ; peu d'outils le font [H] | différenciant | 3 (+29) | R-I4 |
| M-069 | Interprète au rendez-vous et pour les courriers importants | Une décision ou un entretien mal compris est une décision contestable | important | 3 | INS-61 |
| M-070 | Adresse de domiciliation distincte du lieu de vie ; situation sans domicile stable | Le modèle suppose un foyer à une adresse ; l'hébergement d'urgence change toutes les semaines | important | 4 | ADM-77, R-A5 |
| M-071 | Ne jamais demander de titre de séjour ni la situation administrative | Le droit à l'instruction ne dépend pas du séjour [NV, à vérifier] ; une question personnalisée suffit à créer une pratique illégale | bloquant | 3, 4 | R-I1 |
| M-072 | Pièce « sans objet » (pas de certificat de radiation quand on arrive de l'étranger), distincte d'une dispense | Sinon le dossier reste « incomplet » à vie ou on invente une dispense | confort | 3 | R-I2 |
| M-073 | Modalité de scolarisation (UPE2A, temps partagé, double inscription) | Elle change l'emploi du temps, l'appel, le bulletin ; aucune donnée ne la porte dans le socle 2 | important | 2 (+35) | ELV-36, R-E14 |

### P12 — Mme Leroy (ASE) et Sandrine (assistante familiale)

**Situation.** Enzo, 5ᵉ, est confié au service de l'aide sociale à l'enfance ; il vit chez Sandrine ; ses parents gardent l'autorité parentale, avec des visites médiatisées. **Mardi 11 h :** ordonnance de placement provisoire pour sa sœur, scolarisée dans le même collège ; la mère pourrait venir à 16 h 30. **Mars :** changement de famille d'accueil. **Juin :** Mme Leroy change de poste ; son successeur doit tout récupérer.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-074 | Changer de lieu d'accueil en un assistant (résidence, remise, droits, transport, adresse protégée) | Sinon cinq écrans, et un oubli (l'ancienne assistante familiale reste autorisée) | important | 4 | ADM-80 |
| M-075 | Droits portés par l'organisme, et non par la référente du moment | Un changement de référente ne doit pas faire perdre l'accès ni exiger un nouveau lien | important | 4 | R-A7 |
| M-076 | Distinguer « signer les actes du quotidien » et « signer les actes importants » | L'assistante familiale signe une sortie de proximité, pas une inscription [NV, à vérifier] ; le catalogue des droits n'a qu'un `signer_documents` | important | 4 (+18) | R-A8 |
| M-077 | Interdiction provisoire d'urgence, avant la pièce | La décision tombe à 11 h, la copie arrive le lendemain ; il faut protéger l'enfant à 16 h 30 | bloquant | 4 | ADM-90, R-A9 |
| M-078 | Fin de placement (retour en famille, mainlevée) en cours d'année | Les droits des parents et de l'organisme doivent basculer à la date | confort | 4 | ADM-80, R-A7 |
| M-079 | Bilans d'assiduité adressés au service gardien | Le service les demande ; ils relèvent du socle 12 | confort | 12 | renvoi |

### P13 — Léa, 18 ans le 14 février (terminale)

**Avant.** Elle ne sait pas que ses droits changent ; ses parents non plus. **Le 14 février.** Le bulletin du 2ᵉ trimestre est en préparation ; les vœux d'orientation sont en cours. **Avril.** Elle quitte le domicile familial après un conflit ; elle ne veut pas que ses parents voient sa nouvelle adresse ; ils restent payeurs.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-080 | Préparer la majorité un mois avant (élève et parents informés, choix proposés, coordonnées propres vérifiées) | Une bascule brutale à minuit, au milieu de l'orientation, crée des coupures et des conflits | important | 2, 4 | ELV-39, R-A25 |
| M-081 | Parents informés de ce qu'ils ne reçoivent plus, sans le détail du choix | Sans explication, ils appellent l'établissement | important | 4 | ADM-86 |
| M-082 | Élève majeure avec sa propre adresse, invisible de ses parents | Elle n'est pas une « personne » du socle 4 sauf comme payeuse ; sa résidence propre n'est pas prévue | important | 4 | R-A26 |
| M-083 | Documents à cheval sur la majorité : droits évalués à la date d'envoi | Le bulletin du 2ᵉ trimestre (période où elle était mineure) part après sa majorité : à qui ? | confort | 4 (+23) | R-A27 |

### P14 — Inès, mère protégée par une ordonnance de protection

**Situation.** Elle a fui avec ses enfants ; le juge a ordonné la dissimulation de son adresse [NV, à vérifier] ; le père garde l'autorité parentale. Elle inscrit sa fille en 4ᵉ dans un collège privé, en ligne, un dimanche soir.
**Ce que font les conceptions aujourd'hui.** `03` §4.4 envoie une **demande d'accord** au second parent dès le dépôt ou l'admission (privé : « tous les titulaires ») ; D-014 l'invite à créer un compte à la validation ; l'adresse déclarée entre en clair dans le socle 4 ; la protection n'est posée qu'ensuite, par la direction, si quelqu'un y pense.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-084 | Marquer un responsable « à ne pas solliciter » (ni demande d'accord, ni invitation, ni relance, ni copie) | La demande d'accord révèle au père l'établissement où se trouve l'enfant | bloquant | 3, 4 | INS-57, R-I8, **A0-c** |
| M-085 | Déclarer dès la préinscription une situation qui exige la confidentialité | La famille sait avant l'établissement ; si le formulaire ne le demande pas, l'adresse est déjà partie | bloquant | 3 | INS-58, R-I9 |
| M-086 | Une adresse à protéger n'est jamais écrite en clair, même avant la décision de la direction | Le socle 4 actuel stocke l'adresse en texte libre ; la protection posée après coup laisse des copies (journal, export, cache) | bloquant | 4 | R-I9, **A0-b** |
| M-087 | Le père demande « toutes les données me concernant et concernant ma fille » | L'extrait doit exclure l'adresse et les coordonnées de la mère, et les motifs scellés | bloquant | 2, 4 (+27) | ADM-99, R-E19, R-A31 |

### P15 — Sam, 15 ans, demande un prénom d'usage

**Situation.** Ses parents sont d'accord ; l'établissement applique la procédure [NV, à vérifier : circulaire de 2021]. Une fois le prénom d'usage posé, Sam voit encore son ancien prénom dans son identifiant de connexion et dans l'adresse de messagerie interne ; un remplaçant imprime une vieille liste.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-088 | Identifiants de connexion et adresse de messagerie qui suivent le prénom d'usage | L'ancien prénom affiché à chaque connexion, et visible des autres dans la messagerie | important | 2 (+1) | ELV-47, R-E16 |
| M-089 | Sensibilité forcée de certaines versions (adoption, changement de prénom, de mention du sexe) | Ne pas dépendre d'une case cochée par la personne qui saisit | important | 2 | R-E3, **E2-c** |
| M-090 | Nom transmis au GAR et aux ressources numériques (état civil ou usage ?) | `02` dit « exports nationaux = état civil » ; le GAR est un annuaire d'usage quotidien [NV, à vérifier] | important | 2 (+9) | Q-E22 |
| M-091 | Demande de prénom d'usage non aboutie : jamais visible d'une vue famille | Si les parents refusent, la demande elle-même est une information intime | important | 2 | R-E3 |

### P16 — Hélène, directrice hors contrat bilingue

**Année.** Admissions toute l'année ; familles expatriées qui demandent des certificats en anglais ; un élève suit une partie de sa scolarité au CNED ; inspection annoncée : il faut présenter le registre des élèves et la liste transmise au maire [NV, à vérifier].

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-092 | Registre chronologique des entrées et sorties, à une date | Pièce demandée en cas de contrôle [NV, à vérifier] ; aujourd'hui reconstruit à la main | important | 2 | ELV-37, R-E18 |
| M-093 | Certificats et attestations bilingues | Visa, employeur étranger ; argument commercial à l'international | différenciant | 2, 3 (+7) | R-E17, R-I19 |
| M-094 | Scolarité à temps partiel, double inscription | Cas réel du hors contrat et du sport de haut niveau | confort | 2 | ELV-36 |
| M-095 | Admissions au fil de l'eau sans « campagne » datée | Le modèle de campagne annuelle ne colle pas au hors contrat | confort | 3 | R-I5 |

### P17 — Famille Morel, militaire

**Mai :** mutation annoncée ; départ dans trois jours. **Juillet :** arrivée dans un collège privé, admissions closes. **Deux ans plus tard :** expatriation d'un an, retour prévu dans le même établissement.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-096 | Départ express : certificat, documents, comptes, soldes en 48 h | La démarche de départ est complète mais pensée pour un départ planifié | important | 3 | R-I16 |
| M-097 | Mobilité avec retour garanti (année à l'étranger), sans sortie ni certificat | Aujourd'hui : sortie puis réintégration, place perdue en réinscription | confort | 2, 3 | ELV-35, INS-71, R-E13 |

### P18 — Yanis, 20 ans, étudiant de BTS en apprentissage

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-098 | Aucun responsable exigé pour un élève majeur ; il est son propre payeur ou désigne un organisme | Les écrans et les contrôles de qualité supposent des responsables | important | 3, 4 | R-I17 |
| M-099 | Employeur et organisme financeur liés à l'élève | Apprentissage | confort | 51 | renvoi |

### P19 — Bruno (professeur et parent) et Carole (secrétaire, mère séparée)

**Situation.** Carole a un fils dans l'établissement ; elle est séparée du père, qui a déménagé et dont l'adresse est au dossier. Par son rôle de secrétaire, elle peut ouvrir la fiche de son fils et lire l'adresse du père. Bruno est membre de la commission d'admission l'année où sa nièce candidate.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-100 | Un personnel responsable d'un élève ne lit pas, par sa casquette professionnelle, les données des autres responsables de cet élève | Usage détourné le plus probable du dossier administratif ; risque pénal et humain | bloquant | 4, 2 (+6, 1) | R-A18, R-E10, ELV-46 |
| M-101 | Déport d'un membre de commission en conflit d'intérêts | Équité, et défense de l'établissement en cas de recours | important | 3 | INS-63, R-I13 |
| M-102 | Confirmation du lien d'un professeur-parent sans nouvelle invitation | Il a déjà un compte (personnel) ; l'invitation par e-mail est inutile et bloquante | bloquant | 4 | ADM-82, **A0-d** |

### P20 — Kevin, assistant d'éducation

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-103 | Déclarer un badge perdu et en attribuer un nouveau, sans passer par le secrétariat | Matin à la grille, midi à la cantine : l'élève sans badge bloque la file | confort | 2 | ELV-41 |
| M-104 | Enregistrer le refus de la photo administrative | Sinon l'élève apparaît dans « photos manquantes » chaque semaine | confort | 2 | ELV-42 |
| M-105 | Signaler une erreur d'identité vue sur le terrain (« il s'appelle Yanis, pas Yannis ») | Le terrain voit les erreurs avant le secrétariat | confort | 2 | ELV-40 |

### P21 — M. Duval, agent comptable (lycée public)

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-106 | Modalités du régime (forfait 4 ou 5 jours) portées avec le régime | La facturation au forfait en dépend [H] | confort | 2 (+40) | R-E6 |

(Les dates de changement de régime sont traitées en M-021.)

### P22 — La direction face au décès d'un élève

**Avril.** Un élève de 4ᵉ décède un samedi. Lundi, sans règle, partiraient : la relance de réinscription, la fiche de rentrée de l'an prochain, la facture de cantine, l'alerte d'absence non justifiée, la notification d'une note.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-107 | Enregistrer le décès d'un élève, avec arrêt immédiat de **tous** les automatismes le concernant | Un message automatique à une famille endeuillée est la pire faute qu'un ENT puisse commettre ; le motif de sortie `deces` n'existe même pas dans `02` §7.4 | bloquant | 2, 3 (+8, 12, 24) | ELV-34, R-E12 |
| M-108 | Décès d'un responsable signalé par la famille depuis le portail | ADM-04 est réservé au secrétariat ; la famille est souvent la première à pouvoir le dire | important | 4 | ADM-76 |

### P23 — Le délégué à la protection des données, et un père qui exerce son droit d'accès

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-109 | Registre des demandes d'exercice des droits, avec échéance et preuve de réponse | Délai d'un mois [NV, à vérifier] ; sans suivi, il est dépassé | important | 4 (+27) | ADM-99 |
| M-110 | Dossier de justification d'une décision d'admission (critères, avis, chronologie) | Recours, Défenseur des droits, contrôle | important | 3 | INS-67 |

### Manques transverses (plusieurs personas)

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-111 | Vue « constellation familiale » : tous les foyers, personnes, demi-frères et sœurs, rôles | Une famille recomposée sur trois foyers est illisible en liste (P01, P07, P09) | différenciant | 4 | ADM-81 |
| M-112 | Signaler une suspicion de fraude (justificatif de domicile, faux bulletin) sans conséquence automatique | Sectorisation dans le public, sélection dans le privé (P01, P02) | confort | 3 | INS-83 |
| M-113 | Session de tests d'admission (convocations, salles, résultats) | Fréquent dans le privé sélectif et les sections (P01, P16) | confort | 3 | INS-53 |
| M-114 | Avis externe par lien à usage unique (club sportif, conservatoire) | Sections sportives, musicales, internationales | différenciant | 3 | INS-54 |
| M-115 | L'élève prépare sa démarche, le responsable la vérifie et la dépose | Le lycéen sait mieux que ses parents ce qu'il veut ; les parents gardent la décision | différenciant | 3 | INS-60 |
| M-116 | Proposer une offre alternative (« pas de place en bilangue, une place en classe standard ») | Aujourd'hui : refus puis nouvelle candidature | confort | 3 | INS-65 |
| M-117 | Sous-capacités d'internat par unité d'hébergement (dortoirs, chambre adaptée) | Une capacité d'internat globale ne dit pas s'il reste un lit compatible | important | 3 | R-I15 |
| M-118 | Procuration temporaire pour les actes du quotidien (hospitalisation d'un parent) | Ni délégation d'autorité, ni personne autorisée : un mandat daté | important | 4 | ADM-83, R-A16 |
| M-119 | Correspondant d'un interne dont la famille vit à l'étranger | Internats, lycées français à l'étranger | important | 4 (+41) | ADM-84, R-A17 |
| M-120 | Déclarer deux personnes distinctes (père et fils homonymes) | Fusion proposée à tort, et répétée | confort | 4 | ADM-71 |
| M-121 | Vérifier l'identité d'une personne au guichet pour rétablir son accès (e-mail perdu) | D-018 suppose l'accès à l'e-mail ; un parent qui l'a perdu n'a aucune voie | important | 4 (+1) | ADM-72 |
| M-122 | Responsable incarcéré | Courrier seulement, aucune remise, autorité inchangée sauf décision [NV, à vérifier] | confort | 4 | C41, R-A29 |
| M-123 | Responsable lui-même majeur protégé | Son tuteur ou curateur doit pouvoir être joint selon la mesure [NV, à vérifier] | confort | 4 | C42, R-A30 |
| M-124 | Versions sensibles pour les personnes (changement de prénom, de civilité d'un parent) | `04` scelle les versions de l'élève, pas celles des personnes | important | 4 | R-A28 |
| M-125 | Liste noire étendue aux revenus et à la nationalité des responsables avant l'admission | Les mêmes questions personnalisées, autres données | important | 3 | R-I1 |
| M-126 | Familles sans domicile : justificatif non bloquant, attestation de domiciliation acceptée | Droit à la scolarisation [NV, à vérifier] | important | 3, 4 | R-I25, R-A5 |
| M-127 | Recours contre une non-réinscription décidée par l'établissement | Même besoin que M-026, côté réinscription | important | 3 | INS-66 |
| M-128 | Appel hors ligne et listes de secours de la journée | Salle sans réseau, panne | important | 12, 11 | renvoi |
| M-129 | Traduction des gabarits d'envoi (absence, convocation, facture) | Prolonge M-068 hors du socle 3 | différenciant | 8 | renvoi |
| M-130 | Autorisations signées par la personne qui gère le quotidien (assistante familiale, correspondant, procuration) | Prolonge M-076 dans le moteur de formulaires | important | 18 | renvoi |
| M-131 | Accessibilité des parcours famille (parent malvoyant, illettrisme) | Les familles les plus fragiles sont aussi celles qui appellent le plus | important | 29 | renvoi |
| M-132 | Page d'état du service et mode dégradé annoncé aux établissements | Panne un jour de rentrée ou de date limite | important | 28 | renvoi |

### Couverture Charlemagne et actions de terrain (D-023, D-024)

Sources : le modèle Charlemagne repris dans le dépôt (`docs/ent-metier-checklist.md`, `db/schema-charlemagne-p*.ts`) et une lecture ciblée, en lecture seule, de l'application Next.js en production (accueil, dossier élève, guichet de demandes, modèles de documents). Les actions de terrain de Next.js qui relèvent d'autres socles (déclaration d'absence par l'accueil : socle 12, déjà conçue en S2-4 ; passages au portail : socle 14) ne sont pas recopiées ici ; seules leurs dépendances aux socles 2 à 4 le sont.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-133 | Rechercher l'établissement d'origine ou de destination dans l'**annuaire national des établissements** (UAI), au lieu d'un texte libre | Charlemagne et Next.js ont ce référentiel ; sans lui, origine et destination sont inexploitables (exports, certificats, statistiques) | important | 2, 3 (+9) | ELV-48, R-E23 |
| M-134 | **Identifier un appelant** à l'accueil (« je suis la mère de Léo ») : le numéro appelant ou le nom donné correspond-il à un responsable du dossier ? | C'est l'action de terrain qui précède la déclaration d'absence par téléphone (Next.js la fait, Charlemagne non [F Next, H]) ; sans vérification, n'importe qui peut excuser un élève | important | 4 (+12, 14) | ADM-100, R-A42 |
| M-135 | Une personne **sans compte** demande un changement (adresse, téléphone) par un formulaire public vérifié par code | Le guichet de demandes de Next.js accepte les dépôts sans compte ; les familles sans compte (grand-mère, parent sans e-mail d'invitation) restent sinon hors circuit | confort | 4 | ADM-101 |
| M-136 | **Sortie seul** et présence aux temps périscolaires **par jour** (midi, soir, étude, garderie), comme le dossier Next.js | Autorisation parentale quotidienne, distincte d'une personne autorisée ; lue par l'accueil et la vie scolaire | important | 4 (+14, 40, 43) | ADM-102, R-A43 |
| M-137 | **Fiche d'inscription imprimable** (dossier complet d'un candidat ou d'un élève, une page par section), modèle prêt | Présente dans Next.js et dans tous les logiciels du marché ; utile au guichet et en commission | confort | 3 | INS-84 |
| M-138 | Les conceptions prévoient des **courriers et certificats « paramétrables »** (gabarits, modèles de courriers) : contraire à D-023 | Sans trames prêtes, chaque établissement reconstruit ses courriers (le défaut vécu avec Charlemagne) | important | 2, 3, 4 (+7) | R-E24, R-I27, R-A41, §5 bis.3 |

---

## 4. Manques qui relèvent de socles ultérieurs

Notés pour que le porteur de chaque socle les retrouve ; non conçus ici.

| Socle | Manques | Ce qu'il faudra prévoir |
|---|---|---|
| 1 Sécurité | M-088, M-121 | Renommer identifiant de connexion et adresse interne après un changement de prénom ou de nom (alias caché, délai) ; récupération d'accès après vérification au guichet, avec délai de sécurité D-018 |
| 5 Structure | M-005, M-027, M-035 | Calendrier des fermetures du secrétariat ; fuseau de l'établissement lu par tous les socles ; contrainte « séparer » alimentée par une mesure du socle 13 |
| 6 Personnels | M-043, M-100, M-101 | Droits ouverts à la date de prise de poste ; déclaration et détection des liens personnels d'un agent avec un élève |
| 7 Documents | M-008, M-010, M-028, M-093 | Attestations vérifiables (code ou QR) ; gabarits bilingues ; dossier scolaire transmis avec bordereau ; filigrane |
| 8 Notifications | M-054, M-059, M-129 | « Rappelez l'établissement » à tous les joignables ; historique de ce que j'ai reçu ; gabarits traduits |
| 11 Mobile | M-047, M-048 | Cache chiffré de la remise du jour, invalidé par push en moins d'une minute |
| 12 Appel et assiduité | M-034, M-079, M-128 | Famille injoignable ; bilans au service gardien ; appel hors ligne |
| 13 Discipline | M-035 | Interdiction de contact entre élèves |
| 14 Accueil, sorties | M-044 à M-049 | Registre des remises, carte de retrait, consignes téléphoniques (le socle 4 fournit la vérification) |
| 18 Formulaires | M-076, M-130 | Chaque formulaire déclare le droit requis : `signer_quotidien` ou `signer_documents` |
| 24 Facturation | M-021, M-050, M-052 | Forfaits et dates de changement ; réduction fratrie selon le port `fratrie` ; répartition au foyer du jour |
| 27 RGPD | M-087, M-109 | Registre des demandes d'exercice des droits, échéances |
| 28 Mise en service | M-018, M-132 | Reprise de dossiers en cours ; mode dégradé |
| 29 Portails | M-068, M-131 | Interface multilingue ; accessibilité |
| 34 Santé | M-055 | Contacts médicaux |
| 35 Besoins particuliers | M-042, M-073 | Points d'attention dès le premier cours ; contenu des modalités UPE2A, ULIS |
| 36 Suivi social | M-031, M-034 | Protocole de disparition ; famille injoignable |
| 38 Examens | M-020 | Utiliser la campagne de vérification de l'état civil avant les inscriptions |
| 41 Internat | M-117, M-119 | Unités d'hébergement ; correspondants |
| 42 Sorties | M-039 | Liste hors ligne avec urgences et autorisés |
| 47 Pilotage | M-023 | Indicateurs sociaux agrégés avec seuil de secret |
| 51 Post-bac | M-099 | Employeur, organisme financeur |

---

## 5. Synthèse

### 5.1 Manques par gravité

| Gravité | Nombre |
|---|---|
| bloquant | 11 |
| important | 75 |
| confort | 39 |
| différenciant | 13 |
| **Total** | **138** |

Dont **122** traités par un ajout dans les socles 2 à 4, et **16** renvoyés à un socle ultérieur (15) ou à une question ouverte (M-090 → Q-E22). Les onze bloquants : M-048, M-065, M-071, M-077, M-084, M-085, M-086, M-087, M-100, M-102, M-107.

### 5.2 Ajouts par socle

| Socle | Actions métier ajoutées | Règles, paramétrages, cas limites ajoutés | Questions ajoutées |
|---|---|---|---|
| 2 L'élève (`02_ELEVE.md`) | **20** (ELV-29 à ELV-48) | 24 règles (R-E1 à R-E24), 10 cas limites | 6 (Q-E18 à Q-E23) |
| 3 Inscriptions (`03_INSCRIPTIONS.md`) | **32** (INS-53 à INS-84) | 27 règles (R-I1 à R-I27), 16 cas limites | 6 (Q-I21 à Q-I26) |
| 4 Dossier administratif (`04_ADMINISTRATIF.md`) | **32** (ADM-71 à ADM-102) | 43 règles (R-A1 à R-A43), 20 cas limites (C28 à C47) | 7 (Q-A21 à Q-A27) |
| **Total** | **84** | 94 règles, 46 cas limites | 19 |

### 5.3 Les dix ajouts les plus importants

1. **Ne pas solliciter un responsable dangereux, et protéger une adresse dès la préinscription** (INS-57, INS-58, R-I8, R-I9 ; A0-b, A0-c). Sans cela, la demande d'accord du second parent révèle l'établissement de l'enfant à la personne dont la mère se protège.
2. **Décès d'un élève : arrêt de tous les automatismes** (ELV-34, R-E12). Aucune relance, facture, fiche de rentrée ni alerte ne doit partir.
3. **Conflit d'intérêts d'un personnel qui est aussi responsable** (R-A18, R-E10, ELV-46). Une secrétaire ne lit pas l'adresse de son ex-conjoint par sa casquette professionnelle.
4. **Interdiction de remise provisoire d'urgence, visible partout en moins d'une minute, et fiche de secours hors ligne** (ADM-90, R-A10, ADM-91).
5. **Date de naissance incomplète ou présumée** (ELV-29, R-E1 ; **E2-a**). Sans elle, on invente des dates.
6. **Confirmation du lien au guichet ou en un geste** (ADM-82 ; **A0-d**). Sans elle, la sécurité ajoutée par A0 bloque des parents légitimes à la rentrée.
7. **Jamais de titre de séjour ni de situation administrative demandés ; pièce « sans objet »** (R-I1, R-I2).
8. **Droit d'accès d'un parent sans les données de l'autre, et registre des demandes avec échéance** (R-E19, R-A31, ADM-99).
9. **Déclarations contradictoires des deux parents et échange ponctuel de garde** (ADM-96, ADM-78, R-A1, R-A2). La garde alternée « native » ne tient que si elle survit aux désaccords et aux échanges.
10. **Remise non prévue : accord à distance du titulaire, consigne téléphonique confirmée par rappel, personne inconnue qui se dit parent** (ADM-87, ADM-88, ADM-89).

Suivent de près : annuler une validation erronée, prolonger une échéance en masse, suspendre les délais pendant les fermetures (INS-68, INS-79, INS-80) ; la campagne de vérification de l'état civil avant les examens (ELV-32) ; la répartition des frais « au foyer du jour » (ADM-93).

### 5.4 Ce que la passe a montré sur les conceptions

- **Les conceptions sont solides sur le modèle** (historique, propositions, cloisonnement, parents séparés). Les manques sont surtout des **gestes de rattrapage** (annuler, prolonger, corriger en lot, déclarer distincts), des **situations de crise** (décès, danger, panne, urgence) et des **voix absentes** (le terrain qui constate, la famille qui ne parle pas français, la grand-mère sans compte).
- **Trois angles morts reviennent dans les trois socles** : le **fuseau** codé en dur, la **sécurité qui bloque des cas légitimes** (A0 sans confirmation au guichet), et l'**absence de « conflit d'intérêts »** entre les casquettes d'une même personne.
- **Le socle 3 est le plus exposé au danger** : il contacte des personnes **avant** que le socle 4 ait pu poser une protection. L'ordre 3 → 4 fixé par D-021 rend nécessaires les champs réservés du contrat (A0-b).

---

## 5 bis. Retour de Florian (D-023, D-024) : réglages signalés et documents prêts

### 5 bis.1 Ce que ce retour change dans la lecture

- **Rien n'est retiré.** Aucune action, aucun réglage des conceptions n'est supprimé (D-023) ; les manques de couverture Charlemagne et de terrain sont ajoutés (M-133 à M-138).
- **Les réglages lourds sont signalés** ci-dessous, avec une version automatique proposée, pour une décision commune lors des relectures. Les ajouts de cette passe sont passés au même crible : plusieurs de mes propres paramètres y figurent.
- **La liberté reste totale pour les données de l'établissement** (D-022) : motifs locaux, régimes locaux, natures de lien proposées, informations complémentaires, offres, capacités, critères affichés ne sont **pas** signalés.

### 5 bis.2 Réglages candidats à l'allègement (signalés, non supprimés)

Règle de lecture : « version automatique » = ce que fait le logiciel quand l'établissement ne touche à rien ; le réglage reste disponible tant que la décision n'est pas prise.

| # | Réglage | Où | Pourquoi il pèse | Version automatique proposée |
|---|---|---|---|---|
| S-01 | Cinq réglages **par champ** de l'élève (visible, obligatoire et à quel moment, saisie famille, source qui fait foi, modification locale) | `02` §7.8, `ent_param_eleve_champ` | Des dizaines de cases que personne n'ouvrira ; exactement l'accumulation reprochée à Charlemagne | Entièrement déduits du **profil d'établissement** (public, privé sous contrat, hors contrat) ; seules les **exceptions** sont modifiables, dans une liste courte « champs que vous voulez demander en plus » |
| S-02 | Règles d'application automatique des propositions, par champ et par source | `02` §7.8, §9.2 | Redondant avec la source qui fait foi | Déduites de la source qui fait foi ; aucun réglage visible |
| S-03 | Format du numéro d'élève (motif, préfixe de site, remise à zéro annuelle) | `02` §9.2 | Réglé une fois, jamais revu ; source d'erreurs à la reprise | Format unique du logiciel ; reprise des numéros de l'ancien logiciel conservés |
| S-04 | Durée de conservation des anciennes photos « dans les bornes » | `02` §9.2 | Décision DPO, pas de l'établissement | Valeur fixe validée par le DPO |
| S-05 | Âge ou niveau d'ouverture des comptes élèves | `02` §7.9, §9.2 | Peu d'établissements dévient du profil | Profil (compte à partir de la 6ᵉ dans le second degré) ; exception possible |
| S-06 | Procédure du prénom d'usage « paramétrée » | `02` §7.2, §9.2 | Une procédure se décrit, elle ne se paramètre pas | Procédure du logiciel conforme au cadre vérifié [NV] ; seule l'activation reste |
| S-07 | Étapes d'instruction paramétrées par campagne (liste, ordre, libellés, caractère obligatoire) | `03` §8.7, §9 | Chaque campagne à reconstruire | Trois parcours types (dossier simple, avec entretien, avec commission) ; copie de l'an dernier (INS-19) ; personnalisation rare |
| S-08 | Règles de pièces : moment, caractère bloquant, **conditions en JSON** | `03` §8.6 | Illisible pour un secrétariat | Liste de pièces par profil, avec moment et caractère bloquant déjà choisis ; ajout d'une pièce en un clic ; conditions choisies dans une liste fermée |
| S-09 | Motif communiqué : jamais / facultatif / toujours | `03` §8.1 | Choix de principe fait une fois pour toutes | « facultatif » fixé ; le motif communiqué est proposé par la trame |
| S-10 | Règle de relance (intervalle, nombre, canal) par campagne | `03` §8.1, §9 | Réglage technique sans enjeu métier | Rythme du logiciel (J+3, J+7, puis veille d'échéance), plafonné |
| S-11 | Délais bornés (réponse à une proposition de place, non-présentés J+N, fermeture des comptes) | `03` §9 | Trois nombres à fixer chaque année | Valeurs du profil (8 jours, 3 jours de classe, 30 jours) ; contractuels seulement dans le privé |
| S-12 | Reconfirmation des rendez-vous (oui / non, J-N) | `03` §9 | — | Toujours J-7 (héritage Next.js), désactivable |
| S-13 | Format du numéro de dossier | `03` §9 | Comme S-03 | Format unique |
| S-14 | **Modèles de courriers paramétrables**, gabarit du certificat de radiation | `03` §9, §11 | Contraire à D-023 | Trames prêtes (§5 bis.3) |
| S-15 | Saisie famille **par champ** (non / proposer / modifier) | `04` §8.10, §8.13 | Même défaut que S-01 | Profil ; trois catégories fixes (coordonnées personnelles : modifier après vérification ; adresse : proposer ; situation juridique : non) |
| S-16 | Pièces par fait (exigée / attendue / non) | `04` §8.13 | Décision de prudence juridique, rarement locale | Valeurs du logiciel, alignées sur la vérification juridique (Q-A16) |
| S-17 | **Matrice nature × droit** des droits par défaut d'un lien | `04` §8.6, §8.13 | Une grille de 16 × 16 cases | Droits par défaut du logiciel par nature et qualité ; ajustement **au cas par cas** (ADM-33), pas dans une matrice |
| S-18 | Règles d'envoi « dans les bornes » | `04` §8.12 | La règle par défaut est la bonne dans presque tous les cas | Règles du logiciel ; un seul choix exposé : « absence en garde alternée : prévenir les deux / le foyer du jour » (Q-A8) |
| S-19 | Paramètres de remise (pièce d'identité exigée, autorisations ponctuelles sans validation, nombre maximal) et d'urgence (nombre minimal et maximal de contacts) | `04` §8.13 | Petits réglages qui s'accumulent | Valeurs du profil ; seuls « photos des personnes autorisées » et « carte de retrait » restent des choix explicites |
| S-20 | Sections et caractère obligatoire de la fiche de rentrée | `04` §8.10, §8.13 | Reconstruite chaque année | Fiche type du logiciel ; sections ajoutées seulement par les modules activés |
| S-21 | Libellés des natures de lien | `04` §8.13 | Utile pour un établissement sur cent | Libellés du logiciel ; personnalisation rare |
| S-22 | **Mes ajouts** : durée d'une exception de garde, délai de l'accord à distance, durée maximale d'une procuration, âge du co-remplissage, durée du bris de glace, délai d'annulation d'une validation, publication programmée, jours ouvrés pour les délais | `02` R-E11, `03` R-I6, R-I7, R-I11, R-I22, `04` R-A2, R-A12, R-A16 | Paramètres ajoutés par cette passe : même risque d'accumulation | **Valeurs fixes** du logiciel au départ ; un réglage n'apparaît que si un établissement pilote en exprime le besoin |
| S-23 | Dates de changement de régime autorisées (R-E5) | `02` R-E5 | Nouveau réglage | Déduit du profil (fin de trimestre dans le public, libre dans le privé) |

### 5 bis.3 Documents : 3 ou 4 trames prêtes par type (D-023)

Règle pour les socles 2 à 4 : chaque document ci-dessous existe en **3 ou 4 trames** soignées ; l'établissement en choisit une et n'ajuste que son **identité** (logo, couleurs, coordonnées, signature, et un ou deux textes libres encadrés). Le contenu se remplit **seul** à partir des données. Pas d'éditeur de mise en page, pas de modèle à construire dans un traitement de texte. Le rendu passe par le moteur de documents du socle 7 et la conversion maison (D-005). Next.js a déjà quatre modèles codés (certificat de scolarité, fiche d'inscription, autorisation de sortie, courrier aux familles) et aucun éditeur [F Next] : c'est la bonne direction.

| Socle | Documents |
|---|---|
| 2 | Certificat de scolarité ; attestation de scolarité passée (ELV-38) ; registre des élèves (ELV-37) ; liste nominative et trombinoscope imprimés (filigrane) ; extrait des données de l'élève (droit d'accès) ; versions bilingues (R-E17) |
| 3 | Accusé de réception ; demande de pièces ; convocation (entretien, tests) ; courriers de décision (admis, admis sous réserve, liste d'attente, refus, offre alternative) ; proposition de place ; non-réinscription ; réponse à un recours ; attestation d'inscription N+1 ; certificat de radiation ; bordereau du dossier scolaire transmis ; fiche d'inscription imprimable (INS-84) ; listes mairie ; dossier de justification d'une décision |
| 4 | Fiche de rentrée papier pré-remplie ; étiquettes et publipostage ; fiche de secours du jour ; carte de retrait ; extrait des données d'une personne ; information d'une décision sur l'autorité (texte rédigé par la direction dans un cadre fixe) |

Règles [R] : une trame ne contient **aucune** donnée hors de ce que le destinataire a le droit de recevoir (adresse protégée, autres responsables : `04` §8.12) ; les textes libres encadrés sont courts et relus ; les trames sont versionnées par ScolIA (un document émis garde la version qui l'a produit) ; le changement de trame vaut pour les documents **futurs**.

---

## 6. Méthode proposée pour refaire cette passe sur chaque socle suivant

1. **Quand.** Après la conception sur Opus et **avant** le premier lot de code ; une seconde passe courte après le premier lot livré (ce qui manque au vrai écran). Budget indicatif : un tiers du temps de conception.
2. **Choisir 15 à 25 personas** à partir du tableau « Acteurs » du socle, en ajoutant toujours : un acteur public, un privé sous contrat, un hors contrat ; un établissement d'outre-mer ; un nouveau venu (remplaçant, famille arrivée en cours d'année) ; une personne vulnérable (non francophone, sans compte, en danger) ; une personne qui a deux casquettes ; un contrôleur (inspection, DPO).
3. **Pour chaque persona, dérouler** sa journée, sa semaine et son année mois par mois, puis trois incidents : une **erreur** (et comment l'annuler), une **contestation** (et qui tranche), une **panne** un jour critique.
4. **Poser la grille transversale** à chaque socle (c'est elle qui a produit la moitié des manques ici) : décès ; majorité ; danger et protection ; panne et hors ligne ; fuseau ; langue et accessibilité ; conflit d'intérêts ; fermeture (vacances, grève) ; erreur et annulation ; contestation et recours ; fraude ; plusieurs établissements (D-013) ; reprise en pleine saison ; « qui le voit, et que voit l'autre parent ? ».
4 bis. **Comparer à Charlemagne et au terrain** (D-023, D-024) : la matrice de couverture Charlemagne du socle, et les actions que l'équipe faisait « à côté » (tableur partagé, traitement de texte, papier) ; **signaler** dans une liste à part, sans rien supprimer, les réglages qui pèsent, avec leur version automatique ; ramener chaque document produit à 3 ou 4 trames prêtes.
5. **Noter chaque manque** dans le même tableau (manque, pourquoi, gravité bloquant / important / confort / différenciant, socle, ajout), en renvoyant ce qui n'appartient pas au socle.
6. **Écrire les ajouts dans la conception elle-même**, marqués **[personas]**, avec des identifiants qui prolongent la numérotation (jamais de renumérotation) ; chaque action ajoutée a son acteur, son droit, ses règles et son événement ; chaque manque **bloquant** reçoit un scénario de test nommé.
7. **Signaler en tête** ce qui change pour les lots déjà en cours de code.
8. **Confronter à deux personnes réelles** par socle (par exemple une secrétaire et une CPE d'un établissement pilote), en leur faisant raconter leur dernière semaine plutôt que commenter la liste : la passe imaginée ne remplace pas ce regard.
9. **Outils.** Opus pour la passe ; un sous-agent Haiku seulement pour retrouver un fait dans le code ou la doc ; aucune lecture de la base de production.

---

## 7. Faits, hypothèses, contradictions

**Faits [F]** vérifiés dans les conceptions : « Europe/Paris » codé dans `02_ELEVE.md` §7.5 et `04_ADMINISTRATIF.md` §2.2 n° 4, §8 ; motif de sortie sans `deces` (`02` §7.4) ; demande d'accord au dépôt ou à l'admission et invitation à la validation (`03` §4.4) ; lien vers une personne avec compte inactif jusqu'à confirmation (`04` §5.3, A0) ; un seul droit `signer_documents` (`04` §8.6) ; sources de propositions sans « personnel » (`02` §7.8, `04` §8.10) ; adresse en texte libre sur la personne (`04` §2.2 n° 3) ; CPE sans accès aux adresses par défaut (`04` §13.2).

**Hypothèses [H]** : la vie des personas, les fréquences, les pratiques de marché.

**À vérifier [NV]** : droit à la scolarisation indépendant du titre de séjour ; autorisation préalable de l'instruction en famille ; délai de réponse aux demandes d'exercice des droits ; dissimulation de l'adresse dans une ordonnance de protection ; actes usuels de l'assistant familial ; procédure du prénom d'usage ; nom transmis au GAR ; registre des élèves exigé en contrôle ; forfait trimestriel de demi-pension ; affectation UPE2A ; droits d'un parent incarcéré ou sous mesure de protection.

**Contradiction signalée** : D-021 fixe l'ordre 3 → 4 ; le socle 3 contacte des personnes avant que le socle 4 puisse les protéger. Ce document ne change pas l'ordre ; il demande que le contrat `DeclarationResponsables` porte dès A0 les champs de protection (A0-b) et que I0 respecte « ne pas solliciter » (A0-c).
