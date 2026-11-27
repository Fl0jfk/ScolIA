# Passe personas — Socle 26 Bourses et aides financières

**Date :** 2026-11-11 — **Statut :** passe personas sur la conception `docs/socles/26_BOURSES.md`, **non décidée** ; bloquants et ajouts **appliqués par défaut** (D-031) dans la conception, §30. Aucun code ; aucune donnée réelle (personas fictifs ; les noms Haddad, Lambert, Caron, Garnier, Sorel reprennent ceux de `PERSONAS_24_25.md` pour la cohérence des scénarios).
**Branche :** `ent-socle-26-conception`.
**Méthode.** Chaque persona vit une année réelle (2026-2027) à travers la conception, avec la grille : **qui demande, qui décide, qui reçoit l'argent, qui voit** ; ce qui se révèle par un libellé, un compteur, une absence de ligne, une notification, une tâche ; ce qui change à une date (séparation, majorité, départ, retrait) ; ce que la personne peut faire **elle-même** ; ce que l'IA ajoute **sans décider** ; ce que fait Charlemagne (parité, D-024). Chaque trou devient un **manque** (M-BA01…), classé **bloquant** (à trancher avant le code du lot), **important** ou **confort**, avec la réponse ajoutée à la conception (action BRS-3xx, règle R-BA41 et suivantes, cas CBA-61 et suivants).

## 0. En bref

- **17 personas**, **58 manques** (M-BA01 à M-BA58), dont **7 bloquants** (§2), **38 actions** ajoutées (BRS-300 à BRS-337), **6 règles** (R-BA41 à R-BA46), **14 cas limites** (CBA-61 à CBA-74), **3 contradictions** relevées (§4).
- **Les sept bloquants** (appliqués par défaut) :
  1. **Le guichet ne confirme rien à un tiers** : le registre des dépôts ne se consulte pas par élève au guichet (M-BA09, R-BA41).
  2. **Tâches et compteurs du portail adressés au seul bénéficiaire**, jamais à l'élève ni au foyer : sinon l'autre parent voit « 1 document à signer » (procuration) sur la vue de l'enfant (M-BA36, R-BA42).
  3. **L'état civil du concubin** recueilli pour l'étude automatique est la donnée d'un tiers : effacé après transmission (M-BA47, R-BA43).
  4. **Délégué élève en commission** : les dossiers d'élèves de **sa classe** (et de sa fratrie) lui sont masqués (M-BA30, R-BA44).
  5. **« Situation connue du service social »** révèle un suivi social : retiré de la fiche de commission, visible du chef seul (M-BA14, R-BA45).
  6. **Contrôle de l'autorité** : les documents exigés par les services académiques peuvent être nominatifs, produits par la direction, journalisés, transmis par voie sûre (M-BA53, R-BA46).
  7. **Changement d'établissement public → privé en cours d'année** : la bourse change de mode (déduite par l'agent comptable → versée par l'État ou avec procuration) ; sans traitement, on déduit sans procuration (M-BA55, BRS-334).

## 1. Les personas

| # | Persona | Établissement | Ce qui compte pour elle ou lui |
|---|---|---|---|
| P01 | **Mme Fabre**, cheffe d'établissement | Lycée privé pilote (900 élèves) | Ne laisser aucun élève sans repas, décider vite, signer peu, ne rien voir fuiter |
| P02 | **Mme Lambert**, comptable de l'OGEC | Lycée pilote | Ventiler les virements de l'État, reverser au bon parent, justifier le fonds cantines |
| P03 | **M. Ndiaye**, secrétaire de scolarité | Lycée pilote | 200 dossiers papier en septembre, accusés, pièces, téléphone |
| P04 | **Mme Roussel**, assistante de service social | Partagée entre le lycée pilote et deux collèges publics | Secret professionnel, avis technique, peu de temps de présence |
| P05 | **M. Perrin**, CPE | Collège public voisin | Repérer, signaler, justifier les absences sans peser sur la bourse |
| P06 | **Famille Haddad** (Mme Haddad, mère, demandeuse) | Lycée pilote et collège public | Bourse déduite, excédent reversé, comprendre en arabe et en français, discrétion |
| P07 | **Mme et M. Garnier**, séparés, résidence alternée | Lycée pilote | Qui demande, qui reçoit, qui paie la demi-pension |
| P08 | **Théo**, 19 ans, terminale bac pro, non rattaché au foyer fiscal | Lycée pilote | Bénéficiaire lui-même, parents tenus à l'écart |
| P09 | **M. Caron**, professeur organisateur du voyage à Rome, et professeur principal de 1re | Lycée pilote | Informer toutes les familles, ne savoir qui est aidé, partir sans déficit |
| P10 | **Mme Vidal**, adjointe gestionnaire, et **M. Brun**, agent comptable | Collège public (EPLE, 600 élèves) | Fonds sociaux, commission, mandats 6576, bourses déduites, Orquestra |
| P11 | **Mme Leroy**, parent élue, membre de la commission | Collège public | Lire des dossiers sans reconnaître ses voisins |
| P12 | **Inès**, 15 ans, déléguée des élèves en commission | Collège public | Ne pas tomber sur le dossier d'une camarade |
| P13 | **Mme Morin**, présidente de l'APEL | Lycée pilote | Aider des familles pour le voyage, rendre compte à son bureau |
| P14 | **Trois attaquants** : **M. Garnier** (P07) qui veut savoir si son ex-épouse touche une bourse ; **Mme Sorel**, secrétaire curieuse et parente ; **Lucas**, élève de 1re qui teste l'API | Lycée pilote | Toutes les portes : guichet, téléphone, portail, IA, routes, pièces, compteurs, journaux |
| P15 | **Mme Klein**, déléguée à la protection des données (auditrice RGPD) | OGEC | Base légale, minimisation, durées, droits, tiers, sous-traitants, AIPD |
| P16 | **M. Aubert**, contrôleur des services académiques | Rectorat | État de réalisation, emploi des fonds, bilans par année civile |
| P17 | **Famille Petit**, arrivée en février d'un collège public ; père licencié en janvier | Lycée pilote | Transfert de bourse, fonds social en attendant, ne pas tout recommencer |

---

## 2. Personas, années, manques

### P01 — Mme Fabre, cheffe d'établissement
**Année.** *Septembre* : campagne, information. *Octobre* : trois urgences (cantine), une le samedi. *Décembre* : propositions au recteur. *Mars* : retenue proposée pour un élève décrocheur. *Juin* : bilan pour l'OGEC.
**Irritants.** Signer chaque dossier ; savoir où en est l'enveloppe. **Peurs.** Une fuite ; un élève qui ne mange plus ; retenir la bourse d'un élève en grande difficulté. **Ce qui la ravirait.** Une page « décisions à prendre » sur son téléphone ; des propositions prêtes.

| # | Manque | Pourquoi | Gravité | Socles | Réponse |
|---|---|---|---|---|---|
| M-BA01 | **Décision d'urgence depuis l'appli mobile** (samedi), motif, effet immédiat sur la cantine du lundi | La conception parle d'urgence sans canal ; la cantine (40) doit savoir que le repas est pris en charge sans savoir pourquoi | important | 26, 40, 11 | BRS-300 |
| M-BA02 | **Délégation nommée et datée** de `aides.decider` à l'adjoint pendant une absence | Repris du 24 (M-669) ; sans elle, tout s'arrête | important | 26, 1 | BRS-301 |
| M-BA03 | **Retenue : vue « accompagnement en cours »** sans détail, pour ne pas sanctionner deux fois un élève suivi | CBA-14 existe ; la cheffe veut un rappel explicite et l'avis du CPE en un clic | confort | 26, 12, 13 | BRS-302 |
| M-BA04 | **Prise en charge provisoire** du repas en attendant la décision (« pas de refus de repas pendant l'instruction ») | R-BA24 le dit pour les relances, pas pour l'accès au service | important | 26, 40 | R-BA24 précisé, BRS-303 |

### P02 — Mme Lambert, comptable de l'OGEC
**Année.** Virements de l'État fin décembre, mars, juin, **sans liste** détaillée dans le relevé [H] ; 46 boursiers dont 31 avec procuration ; fonds cantines reçu en janvier ; état de réalisation en juin.
**Irritants.** Rapprocher un virement global ; refaire les calculs après chaque changement de régime. **Peurs.** Reverser au mauvais parent ; déduire sans procuration. **Ce qui la ravirait.** Importer l'état de paiement des services académiques et voir la ventilation prête.

| # | Manque | Pourquoi | Gravité | Socles | Réponse |
|---|---|---|---|---|---|
| M-BA05 | **Import de l'état de paiement** (fichier ou PDF saisi) pour ventiler un virement global | Sans liste, la ventilation proposée n'est qu'une hypothèse | important | 26, 25, 9 | BRS-304 |
| M-BA06 | **Lot de reversements** des reliquats d'un trimestre, validé en une fois (chaque ligne contrôlée) | 31 virements un par un | confort | 26, 25 | BRS-305 |
| M-BA07 | **Procurations à régulariser** (reprise sans pièce) : liste et relance groupée aux bénéficiaires | Reprise §24 | important | 26 | BRS-306 |
| M-BA08 | **Écart de barème** : montant notifié ≠ barème (bourse au mérite, retenue décidée par le rectorat) expliqué par ligne | R-BA28 exige un motif ; il faut la liste | confort | 26 | BRS-307 |

### P03 — M. Ndiaye, secrétaire de scolarité
**Année.** 200 dossiers papier, 40 incomplets ; appels « où en est mon dossier ? » ; un père qui demande au guichet si la mère « a fait la bourse ».
**Irritants.** Répondre au téléphone sans savoir qui appelle. **Peurs.** Dire ce qu'il ne faut pas ; perdre un dossier. **Ce qui le ravirait.** Saisie au guichet en 20 secondes, accusé imprimé, scanner de pièces.

| # | Manque | Pourquoi | Gravité | Socles | Réponse |
|---|---|---|---|---|---|
| M-BA09 | **Registre consultable seulement par demande** (numéro d'accusé ou demandeur présent et authentifié), jamais en liste par élève au guichet | Un parent au guichet peut faire chercher « l'élève X » et apprendre si l'autre parent a déposé | **bloquant** | 26, 24 R-F37 | **R-BA41**, BRS-308 |
| M-BA10 | **Réponse téléphonique** : rien sans authentification ; renvoi vers l'espace ou rappel par l'instructeur | Usurpation par téléphone (24 CF-95) | important | 26, 14 | R-BA41, BRS-309 |
| M-BA11 | **Numérisation au guichet** (scanner, tri automatique des pages) sans que le secrétaire voie le contenu après dépôt | R-BA34 : présence seulement ; mais il scanne : il voit au moment du scan, pas après | confort | 26, 7 | BRS-310 |
| M-BA12 | **Retour des originaux** à la famille (jamais conservés) | Pratique ; minimisation | confort | 26 | BRS-311 |

### P04 — Mme Roussel, assistante de service social
**Année.** Présente le mardi au lycée, jeudi et vendredi dans deux collèges publics ; 25 familles accompagnées.
**Irritants.** Trois outils ; ressaisir. **Peurs.** Que la commission lise son évaluation ; que « suivi par le service social » devienne une étiquette. **Ce qui la ravirait.** Un seul écran pour ses trois établissements, sans mélange.

| # | Manque | Pourquoi | Gravité | Socles | Réponse |
|---|---|---|---|---|---|
| M-BA13 | **Vue multi-établissements** de ses seuls dossiers, par établissement, sans donnée croisée | R-BA36 interdit le croisement ; il faut l'agrégation **sur l'appareil** (comme 29) | important | 26, 29 | BRS-312 |
| M-BA14 | Le drapeau **« situation connue du service social »** sur la fiche de commission révèle un suivi | Donnée d'accompagnement social (socle 36) qui sort par le 26 | **bloquant** | 26, 36 | **R-BA45** (retiré de la fiche ; chef seul) |
| M-BA15 | **Avis donné à distance** (jour d'absence) et **délai** de l'avis (sans avis au-delà de 7 jours, le chef n'est pas bloqué) | CBA-35 | confort | 26 | BRS-313 |
| M-BA16 | **Orientation vers les aides externes** (CAF, CCAS, département) avec fiches à jour, sans donnée | Circulaire : accompagner vers d'autres dispositifs [V] | confort | 26 | BRS-314 |

### P05 — M. Perrin, CPE (collège public)
**Année.** Un élève qui ne déjeune plus ; un autre à 17 journées injustifiées dont la famille conteste 5.
**Irritants.** Ne pas savoir à qui signaler. **Peurs.** Que la famille pense que la vie scolaire « coupe la bourse ». **Ce qui le ravirait.** Signaler en deux gestes et savoir que c'est pris en charge.

| # | Manque | Pourquoi | Gravité | Socles | Réponse |
|---|---|---|---|---|---|
| M-BA17 | **Accusé de prise en charge** du signalement (« pris en charge », sans suite ni nom) | R-BA21 interdit la suite ; le signalant a besoin de savoir que ce n'est pas perdu | important | 26 | BRS-315 |
| M-BA18 | La **retenue** ne doit jamais apparaître dans les écrans de justification des absences (pas de « attention : bourse ») | Sinon pression sur la vie scolaire et inégalité de traitement des justifications | important | 12, 26 | R-BA14 précisé, CBA-61 |
| M-BA19 | Le repérage « ne déjeune plus » vient du socle 40 ; **aucune alerte automatique** de ScolIA sur ce motif (profilage) | R-BA17 ; le repérage reste humain | confort | 40, 26 | CBA-62 |

### P06 — Famille Haddad
**Situation.** Yasmine, 1re, demi-pensionnaire, bourse de lycée échelon 5 (319 € par trimestre), procuration ; mère demandeuse, père payeur à 0 % ; Mme Haddad lit mieux l'arabe.
**Année.** *Septembre* : échéancier net. *Janvier* : versement du 1er trimestre, demi-pension 198 €, excédent 121 €. *Avril* : un professeur demande en classe si « la bourse paie le voyage ».
**Irritants.** Ne pas comprendre le net ; attendre l'excédent. **Peurs.** Être repérée. **Ce qui la ravirait.** Tout comprendre dans sa langue, l'excédent sans le demander.

| # | Manque | Pourquoi | Gravité | Socles | Réponse |
|---|---|---|---|---|---|
| M-BA20 | **Explication traduite** de la déduction et du reversement (phrases fixes traduites, montant et date) | §16.1 l'a en français | confort | 26, 8 | BRS-316 |
| M-BA21 | **Calendrier prévisionnel** des versements de l'État et des reversements | Elle attend sans savoir | confort | 26 | BRS-317 |
| M-BA22 | **Aucune question d'un professeur ne trouve de réponse dans ScolIA** ; rappel de discrétion dans la charte des personnels | Fuite humaine ; ScolIA ne peut que ne rien montrer et rappeler | important | 26, 6 | R-BA2 (rappel), BRS-318 |
| M-BA23 | **Bourse au mérite** future : informer en 3e que la mention au DNB compte (sans demande séparée) | Information générale, pas ciblée | confort | 26 | BRS-126 précisé |

### P07 — Mme et M. Garnier, séparés, résidence alternée
**Situation.** Résidence alternée ; Mme Garnier demande la bourse (ressources seules) ; demi-pension payée 50/50 ; M. Garnier dépose aussi une demande papier « pour être sûr ».
**Année.** Double demande en septembre ; demi-pension 50/50 ; trimestre de 244 €.

| # | Manque | Pourquoi | Gravité | Socles | Réponse |
|---|---|---|---|---|---|
| M-BA24 | **Information séparée** sur la double demande sans révéler l'autre (« une seule demande est possible par élève ; si l'autre parent a aussi déposé, les deux sont irrecevables ») : phrase **identique** pour les deux, même si l'autre n'a pas déposé | Sinon le message lui-même révèle la démarche de l'autre | important | 26 | BRS-136 précisé, CBA-63 |
| M-BA25 | **Déduction partielle** : la part du payeur bénéficiaire (50 % de la demi-pension) ; reliquat reversé ; la facture de M. Garnier inchangée | CBA-03 ; à tester | important | 24, 26 | test §27 |
| M-BA26 | **Changement de bénéficiaire en cours d'année** (la résidence devient exclusive chez le père) : retrait pour Mme, information générique à M. | F984 : versement retiré, nouveau responsable peut demander [V] | important | 26, 4 | BRS-194, BRS-195 (précisés) |
| M-BA27 | **Quotient** : chacun le sien ; jamais celui de l'autre visible ou déductible d'un tarif affiché | 24 §10.4 | confort | 24, 26 | R-BA19 |

### P08 — Théo, 19 ans
**Situation.** Non rattaché au foyer fiscal de ses parents ; bourse à son nom ; ses parents paient encore la demi-pension par prélèvement.

| # | Manque | Pourquoi | Gravité | Socles | Réponse |
|---|---|---|---|---|---|
| M-BA28 | **Compte d'élève majeur payeur/bénéficiaire** (FD-15 du 24) prérequis du parcours | Sans compte, il ne voit pas ses décisions | important | 24, 26, 1 | dépendance BA10 |
| M-BA29 | Ses parents payeurs ne voient **ni** sa bourse **ni** son reversement ; leur facture est brute (pas de déduction : il n'est pas payeur) | CBA-02 appliqué à un majeur | important | 24, 26 | R-BA35, CBA-64 |

### P12 — Inès, 15 ans, déléguée en commission (et P11 Mme Leroy)
**Situation.** Commission de novembre, 14 dossiers ; un dossier « 4e, demi-pension, sortie à Paris, 85 € » : la seule sortie à Paris est celle de sa classe.

| # | Manque | Pourquoi | Gravité | Socles | Réponse |
|---|---|---|---|---|---|
| M-BA30 | **Dossiers de la classe** (et de la fratrie) d'un délégué élève ou d'un parent élu **masqués** pour lui ; l'objet « sortie » affiché sans destination | R-BA8 ne couvre pas la connaissance propre du membre | **bloquant** | 26 | **R-BA44**, CBA-65 |
| M-BA31 | **Engagement de discrétion d'un mineur** : expliqué simplement ; information de son responsable [R] | Q-BA10 | important | 26 | BRS-319 |
| M-BA32 | Membre externe **sans compte** : lien nominatif à usage limité, pendant la séance, aucune copie possible (pas de téléchargement, pas d'impression) | R-BA29 ; captures d'écran inévitables → minimisation de la fiche | important | 26, 7 | BRS-320 |
| M-BA33 | **Séance à distance** (visio du socle 33) : projection de la fiche sans envoi | Parents élus indisponibles | confort | 26, 33 | BRS-321 |

### P09 — M. Caron, organisateur et professeur principal
| # | Manque | Pourquoi | Gravité | Socles | Réponse |
|---|---|---|---|---|---|
| M-BA34 | **Coût annoncé avant engagement** incluant « une aide est possible, demandez-la discrètement » sur la circulaire du séjour | 42 §11 ; la phrase doit être partout | confort | 42, 26 | BRS-322 |
| M-BA35 | **Agrégat « moins de 3 »** et **reste à charge global** pour boucler le budget, sans nombre exact si < 3 | R-BA8, CBA-45 | important | 42, 26 | BRS-253 précisé |

### P13 — Mme Morin, présidente de l'APEL
| # | Manque | Pourquoi | Gravité | Socles | Réponse |
|---|---|---|---|---|---|
| M-BA37 | **L'APEL verse à l'établissement** (aide affectée) : elle n'a pas besoin des noms ; état agrégé par séjour, attestation de l'établissement | Q-V5 du 42 ; minimisation | important | 26, 42 | BRS-323 |
| M-BA38 | **Décision APEL tracée** (qui a accordé : bureau de l'APEL, sur proposition du chef) sans compte ScolIA pour le bureau | Le décideur est externe | confort | 26 | BRS-324 |

### P14 — Les attaquants
**M. Garnier** essaie : le guichet, le téléphone, le portail de l'enfant (compteurs, tâches, documents), l'assistant IA (« ma fille est-elle boursière ? », « combien la mère paie-t-elle de cantine ? »), la comparaison de sa facture avec celle de l'an dernier, une demande de droit d'accès RGPD. **Mme Sorel** consulte des dossiers d'élèves de sa connaissance, cherche « boursier » dans la recherche globale, exporte la fiche d'un élève. **Lucas** énumère `/aides/demandes/{id}`, `/objets/{cle}`, rejoue un lien de séance.

| # | Manque | Pourquoi | Gravité | Socles | Réponse |
|---|---|---|---|---|---|
| M-BA36 | **Tâches et compteurs du portail** (« 1 document à signer », « 1 démarche ») liés aux aides : adressés au **seul bénéficiaire**, jamais à la vue de l'élève ni du foyer | Le 29 indexe des tâches par élève ; l'autre parent verrait la procuration à signer | **bloquant** | 26, 29 | **R-BA42**, CBA-66 |
| M-BA39 | **Réponse IA identique** qu'il existe ou non une aide chez l'autre parent (refus générique), testée par paires | R-BA2 ; l'oracle par différence de réponse | important | 26, 1 bis | BRS-325, test |
| M-BA40 | **Comparaison de factures** : sa facture ne varie pas selon les aides de l'autre (le net de l'autre n'intervient jamais dans sa part) | Répartition 24 sur le brut | important | 24, 26 | CBA-67 |
| M-BA41 | **Droit d'accès** : sans les données dont l'autre parent est la personne concernée (demandes, pièces, décisions) | CBA-49 ; DPO | important | 26, 27 | CBA-49 confirmé |
| M-BA42 | **Recherche globale** : aucun résultat ni suggestion « boursier », « fonds social » hors droit ; aucune différence de temps de réponse exploitable | 19.2 point 12 | important | 26, 29 | test |
| M-BA43 | **Consultations inhabituelles** (secrétaire qui ouvre 15 dossiers d'aide en une heure, ou des dossiers hors de ses demandes) : alerte au DPO | R-BA18 trace ; il faut alerter | important | 26, 27 | BRS-326 |
| M-BA44 | **Routes** : identifiants non séquentiels, refus uniforme (404) hors droit, objets du socle 7 servis par URL pré-signée courte après contrôle, liens de séance à usage limité et expirés | CLAUDE.md, D-026 tests générés | important | 26, 7 | test |
| M-BA45 | **Personnel parent** : son propre dossier protégé ; il ne voit pas qu'il est protégé autrement que par sa propre demande | R-BA23 | confort | 26 | CBA-48 |
| M-BA46 | **Export de fiche élève** (socle 2) : jamais de données d'aide | R-BA37 | important | 2, 26 | test |

### P15 — Mme Klein, DPO (auditrice RGPD)
**Questions.** Base légale ? Information des personnes ? Données d'un tiers (concubin) ? Durées ? Sous-traitants (hébergeur, IA) ? AIPD ? Droits ? Décision automatisée ?

| # | Manque | Pourquoi | Gravité | Socles | Réponse |
|---|---|---|---|---|---|
| M-BA47 | **État civil du concubin** (étude automatique) : donnée d'un tiers, collectée pour une transmission ; effacée après transmission ou à la clôture de la campagne, jamais réutilisée | Minimisation, limitation de la finalité | **bloquant** | 3, 26 | **R-BA43**, BRS-327 |
| M-BA48 | **Fiche du registre des traitements** par famille d'aides (bourses : mission d'intérêt public / obligation ; fonds sociaux : mission ; caisse de l'OGEC : contrat ou intérêt légitime [H]) et **mentions d'information** sur chaque formulaire | Socle 27 ; à valider par la DPO | important | 26, 27 | BRS-328, Q-BA1 |
| M-BA49 | **Aucune décision entièrement automatisée** (RGPD art. 22) : chaque décision porte un auteur humain ; les propositions automatiques (retenue, ventilation) sont marquées comme telles | R-BA14, R-BA30 | important | 26 | R-BA31 précisé |
| M-BA50 | **AIPD** avant données réelles ; données sociales de mineurs ; liste CNIL à vérifier | Q-BA2 | important | 26, 24 | Q-BA2 |
| M-BA51 | **Durées et purge démontrées** (rapport de purge, preuve d'effacement des ressources du quotient) | §23 | important | 26, 27 | BRS-329 |
| M-BA52 | **Sauvegardes** : effacement dans les sauvegardes à l'expiration de leur rétention (D-043) mentionné dans l'information | PRA | confort | 26, PRA | BRS-328 |

### P16 — M. Aubert, contrôleur des services académiques
| # | Manque | Pourquoi | Gravité | Socles | Réponse |
|---|---|---|---|---|---|
| M-BA53 | **Documents de contrôle nominatifs** quand l'autorité les exige (état de réalisation, liste des bénéficiaires du fonds cantines), produits par la direction, journalisés, transmis par voie sûre ; jamais un lien public | Conflit apparent avec la pseudonymisation | **bloquant** | 26, 7 | **R-BA46**, BRS-330 |
| M-BA54 | **Bilan par année civile** croisant deux années scolaires (janvier–août de N-1/N, septembre–décembre de N/N+1) | R-BA26 ; requêtes à prévoir | important | 26 | BRS-331 |

### P17 — Famille Petit, arrivée en février ; père licencié en janvier
| # | Manque | Pourquoi | Gravité | Socles | Réponse |
|---|---|---|---|---|---|
| M-BA55 | **Passage du public au privé** : la bourse déduite par l'agent comptable devient versée par l'État ; **sans procuration, aucune déduction** ; proposer la procuration à l'arrivée | Sinon déduction anticipée sans procuration (R-BA12) | **bloquant** | 3, 26 | BRS-334, CBA-68 |
| M-BA56 | **Changement de situation après le 1er janvier** non pris en compte par la bourse → **orientation fonds social** proposée à la famille (information générique) | Circulaire [V] | important | 26 | BRS-332 |
| M-BA57 | **Arrivée = nouveau dossier administratif** : pièces de la bourse non redemandées si la notification suffit | Ne pas tout recommencer | confort | 26 | BRS-333 |
| M-BA58 | **Pièces transmises par l'ancien établissement** : jamais via le dossier scolaire (R-BA37) ; c'est la famille qui fournit sa notification | Minimisation | confort | 26 | CBA-69 |

### P10 — Mme Vidal (adjointe gestionnaire) et M. Brun (agent comptable), collège public
Manques intégrés aux actions BRS-335 à 337 (séparation ordonnateur / comptable, mandats et écritures, bilan Orquestra) et au lot BA13 ; voir §3.

---

## 3. Actions ajoutées (BRS-300 à BRS-337)
| # | Action | Qui · droit | Garde | Lot |
|---|---|---|---|---|
| BRS-300 | Décider une urgence depuis l'appli mobile, avec effet sur la restauration (repas pris en charge, sans motif pour le 40) | DIR ★ | motif « urgence » ; information a posteriori | BA5 |
| BRS-301 | Déléguer `aides.decider` à une personne nommée, bornée dans le temps | DIR ★ | journalisée | BA1 |
| BRS-302 | Voir « accompagnement en cours : oui » et demander l'avis du CPE sur une retenue proposée | DIR | sans détail | BA4 |
| BRS-303 | Prise en charge provisoire du repas pendant l'instruction | DIR, INS | durée bornée ; régularisée par la décision | BA5 |
| BRS-304 | Importer un état de paiement des services académiques pour ventiler un virement | CPT | aperçu ; écarts | BA3 |
| BRS-305 | Valider un lot de reversements de reliquats | CPT ★ | chaque ligne contrôlée ; gardes du 25 | BA3 |
| BRS-306 | Lister et relancer les procurations à régulariser | CPT, INS | bénéficiaires seuls | BA14 |
| BRS-307 | Lister les écarts entre montants notifiés et barème, avec motif | CPT, INS | — | BA2 |
| BRS-308 | Retrouver un dépôt par numéro d'accusé ou demandeur présent authentifié | SEC | pas de recherche par élève (R-BA41) | BA1 |
| BRS-309 | Proposer un rappel par l'instructeur à un appelant | SEC | aucune information au téléphone | BA1 |
| BRS-310 | Numériser au guichet avec tri des pages ; contenu masqué après dépôt | SEC | R-BA34 | BA1 |
| BRS-311 | Noter la restitution des originaux | SEC | — | BA1 |
| BRS-312 | Vue de l'assistante sociale sur ses établissements, réunie sur l'appareil | ASS | aucun croisement serveur | BA5 |
| BRS-313 | Donner un avis à distance ; avis réputé « sans avis » après le délai | ASS, SYS | délai 7 jours (S-2616) | BA5 |
| BRS-314 | Consulter les fiches d'orientation vers les aides externes | ASS, INS, FAM | sans donnée | BA10 |
| BRS-315 | Accuser la prise en charge d'un signalement au signalant | SYS | sans suite ni nom | BA15 |
| BRS-316 | Expliquer déduction et reversement dans la langue de la famille | FAM, IA | phrases fixes traduites | BA10 |
| BRS-317 | Voir le calendrier prévisionnel des versements et reversements | FAM | — | BA10 |
| BRS-318 | Rappel de discrétion dans la charte des personnels et à la première ouverture d'un dossier d'aide | SYS | — | BA1 |
| BRS-319 | Engagement de discrétion adapté au mineur ; information de son responsable | DIR, COM | Q-BA10 | BA5 |
| BRS-320 | Lien de séance sans téléchargement ni impression | SYS | expiré à la clôture | BA5 |
| BRS-321 | Tenir une séance à distance (projection sans envoi) | DIR | socle 33 | BA5 |
| BRS-322 | Phrase « une aide est possible » sur la circulaire et la page du séjour | SYS | toutes les familles | BA9 |
| BRS-323 | Attestation à l'APEL des aides affectées, agrégées par séjour | CPT | nominatif seulement avec convention | BA9 |
| BRS-324 | Enregistrer une décision d'un financeur externe (APEL) avec son auteur externe | INS | pièce | BA7 |
| BRS-325 | Réponse générique de l'IA aux questions sur la situation d'un tiers | IA | identique dans tous les cas | BA12 |
| BRS-326 | Alerter la DPO sur des consultations inhabituelles | SYS | seuils (S-2617) | BA15 |
| BRS-327 | Effacer l'état civil du concubin après transmission ou à la clôture de la campagne | SYS | R-BA43 | BA1 |
| BRS-328 | Produire la fiche du registre et les mentions d'information par dispositif | DIR, DPO | socle 27 | BA14 |
| BRS-329 | Produire le rapport de purge | SYS, DPO | — | BA14 |
| BRS-330 | Produire un document de contrôle nominatif pour l'autorité | DIR ★ | journalisé, voie sûre (R-BA46) | BA6 |
| BRS-331 | Bilan par année civile croisant deux années scolaires | DIR | — | BA11 |
| BRS-332 | Orienter une famille vers le fonds social après un changement de situation | INS | message générique | BA5 |
| BRS-333 | Créer l'attribution d'arrivée sur la seule notification | INS | pas de nouvelles pièces | BA4 |
| BRS-334 | À l'arrivée d'un boursier venu du public dans le privé : mode `direct_etat` puis procuration proposée | SYS, INS | aucune déduction avant la procuration | BA4 |
| BRS-335 | Séparer l'ordonnateur (décide) et l'agent comptable (paie, reverse) dans les droits par défaut d'un EPLE | DIR | public | BA13 |
| BRS-336 | Produire les mandats et écritures des aides (6576, 44116, 7411) pour Op@le | CPT | public ; format à confirmer | BA13 |
| BRS-337 | Préparer la réponse à l'enquête Orquestra | DIR | agrégé, année civile | BA11 |

## 4. Contradictions relevées
1. **`24_FINANCES.md` §5.7** : « fonds social collégien / lycéen versé directement aux familles » dans le privé — **confirmé** par la circulaire [V] ; mais le même paragraphe range le fonds cantines dans les « aides affectées saisies par décision anonymisée » : **précision** : la décision est celle du **recteur** sur proposition du chef (R-BA11), et l'état de réalisation exigé est **nominatif** si l'autorité le demande (R-BA46).
2. **`24_FINANCES.md` §10.4** : « le calcul [du quotient] est au socle 26 » et `ent_foyer_quotient` est au niveau du foyer [F] : le 26 le passe **par payeur** (BD-08) ; aucune contradiction de fond avec « deux payeurs séparés : chacun son quotient » [F].
3. **`MIGRATIONS.md`** réservait **5600–5699 / 560–579** au socle 26 ; le pilote impose **2520–2539** : l'ancienne réservation est **abandonnée** (inscrit au registre).

## 5. Cas limites ajoutés (CBA-61 à CBA-74)
| # | Cas | Traitement |
|---|---|---|
| CBA-61 | Justification d'absence en cours chez un élève proche du seuil de retenue | l'écran de justification ne mentionne jamais la bourse |
| CBA-62 | Élève qui ne passe plus à la cantine | aucune alerte « aide » automatique ; repérage humain, signalement |
| CBA-63 | Un seul parent a déposé ; l'autre demande au guichet ou en ligne | message générique identique ; rien sur l'autre |
| CBA-64 | Élève majeur bénéficiaire, parents payeurs | facture parentale brute ; reversement intégral à l'élève |
| CBA-65 | Délégué de la classe concernée par un dossier | dossier masqué pour lui ; avis rendu sans lui |
| CBA-66 | Procuration à signer pendant que l'autre parent consulte la vue de l'enfant | aucune tâche ni compteur visible de lui |
| CBA-67 | Parent qui compare sa facture d'une année sur l'autre | aucune variation liée aux aides de l'autre parent |
| CBA-68 | Boursier arrivant du public en février | aucune déduction avant procuration ; trimestres déjà versés exclus |
| CBA-69 | Ancien établissement qui envoie la notification de bourse dans le dossier scolaire | pièce non intégrée au dossier élève ; reclassée dans le domaine `aides` par l'instructeur, ou rendue |
| CBA-70 | Membre de commission qui prend une photo de l'écran | fiche minimisée (pas de nom) ; engagement de discrétion ; pas de parade technique au-delà |
| CBA-71 | Urgence un samedi, cantine du lundi | repas pris en charge provisoirement (BRS-303) |
| CBA-72 | Reversement d'un reliquat à un bénéficiaire sans IBAN | chèque ou paiement au guichet (25) ; jamais à l'autre parent |
| CBA-73 | Reprise : `ent_eleve_bourse` acceptée avec procuration, sans bénéficiaire | aucune déduction après la date de reprise ; tâche |
| CBA-74 | Contrôleur qui demande l'accès direct à ScolIA | refus ; documents produits par la direction (R-BA46) |

## 6. Règles ajoutées
| # | Règle |
|---|---|
| R-BA41 | Le **registre des dépôts** ne se consulte au guichet que par numéro d'accusé ou pour un demandeur présent et authentifié ; aucune recherche par élève ; aucune information par téléphone |
| R-BA42 | Tâches, compteurs, avis et documents liés à une aide sont adressés au **seul bénéficiaire** (personne), jamais à l'élève, au foyer ni à « la famille » dans les index du socle 29 |
| R-BA43 | L'**état civil du concubin** recueilli pour l'étude automatique est effacé après transmission ou à la clôture de la campagne |
| R-BA44 | Un membre de commission ne voit jamais un dossier d'un élève de sa classe, de sa fratrie ou de son entourage déclaré ; l'objet est affiché sans destination ni libellé identifiant |
| R-BA45 | La fiche de commission ne porte aucun indice de suivi social (« situation connue du service social » réservé au chef) |
| R-BA46 | Les documents exigés par l'autorité de contrôle contiennent ce qu'elle requiert (nominatif si nécessaire), sont produits par la direction, journalisés et transmis par voie sûre, jamais par un accès direct ni un lien public |

Réglages ajoutés : **S-2616** délai de l'avis social (7 jours), **S-2617** seuils d'alerte de consultations (10 dossiers d'aide par heure hors demandes traitées).

## 7. Ce que la passe change dans la conception (appliqué, §30 de `26_BOURSES.md`)
Bloquants 1 à 7 du §0 ; 38 actions ; 6 règles ; 14 cas limites ; R-BA14 précisé (jamais visible des écrans de justification) ; R-BA24 précisé (accès au service maintenu pendant l'instruction) ; R-BA31 précisé (auteur humain de chaque décision, propositions automatiques marquées) ; BRS-126, BRS-136, BRS-194, BRS-195, BRS-253 précisés.

## 8. Faits, hypothèses, questions
- **Faits [F]** : défauts BD-01 à BD-09 relus dans le code (`afeb8c4`) ; aucun module Next.js d'aides.
- **Hypothèses [H]** : virements de l'État sans liste détaillée ; assistante sociale partagée ; pratique de l'APEL.
- **Questions** (à Florian, via la conception §29.2) : Q-BA5 (usage réel de la commission), Q-BA7 (format des propositions), Q-BA10 (mineurs en commission), Q-BA11 (caisse et APEL réelles).
