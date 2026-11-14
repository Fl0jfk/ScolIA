# Passe « personas » sur le socle 18 — Formulaires, autorisations et consentements

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** ; recommandations appliquées par défaut (D-031), sauf §7.2. Aucun code écrit ni modifié ; aucun accès à une base, à un secret ou à un service de production ; Next.js lu seulement.
**Branche :** `ent-socle-17-18-conception`.
**Éprouve :** `docs/socles/18_FORMULAIRES.md` (lu en entier), avec `17_ANNONCES.md`, `07_DOCUMENTS.md` §10, `04_ADMINISTRATIF.md` §8, `02_ELEVE.md` §7.5, `42_SORTIES_VOYAGES.md` §5.4, §11.2, §32, `PERSONAS_42.md` (bloquant B : aucune autorisation dans Next.js), D-034, D-035.
**Ajouts :** faits dans `18_FORMULAIRES.md`, nouveau **§35** (« ajout personas ») et une ligne au §0. Rien n'est supprimé.
**Numérotation :** manques **M18-001 à M18-062** ; actions **FRM-280 à FRM-294** ; règles **R-FC57 à R-FC72** ; cas limites **CK-77 à CK-92** ; questions **Q-FC10 à Q-FC13** ; réglages **S18-13, S18-14**.

---

## 0. À lire en premier : quatre bloquants et leur recommandation (appliquée par défaut, D-031)

| # | Point | Pourquoi c'est bloquant | Recommandation | Lots |
|---|---|---|---|---|
| **A** | **L'anonymat est trahi par les journaux du serveur.** La conception sépare participation et réponse en base et écrit par lots (§12.1), mais le **proxy et le serveur web** journalisent chaque requête avec l'heure à la seconde, l'adresse IP et la route (`POST /formulaires/{id}/repondre`) ; la participation est enregistrée au même instant. La corrélation est triviale pour qui lit les journaux (P13, P14). | « Anonyme » serait faux : tromperie des répondants (élèves, personnels), RGPD art. 5.1.a. | **Route anonyme dédiée** (R-FC57) : la réponse anonyme est envoyée sur une route **distincte** de la participation, **exclue des journaux d'accès** (ou journalisée sans IP ni identifiant, à la minute), après un **délai aléatoire** côté client (0–120 s) et **sans cookie de session** (jeton de participation à usage unique, aveugle, émis à l'enregistrement de la participation) ; test : à partir des journaux et de la base, aucune jointure ne relie un répondant à sa réponse. | FO4 |
| **B** | **Avec deux titulaires, tout état global révèle la décision de l'autre.** §7.2 montre « en attente d'une autre réponse » ou « un accord commun est nécessaire » : pour des parents séparés, cela **dit** que l'autre a refusé ou n'a pas répondu (P03). | FD-03 corrigé d'un côté, recréé de l'autre. | **Aucun état global aux familles** (R-FC58) : chaque titulaire voit **sa** décision et sa preuve ; il ne voit pas si l'enfant « peut » ou « ne peut pas » ; l'établissement explique sur demande, **sans** dire qui a décidé quoi ; exception : l'élève et ses titulaires voient le **résultat d'une autorisation d'événement** (« Emma participe / ne participe pas à la sortie »), car il est observable de toute façon. | FO0 |
| **C** | **La saisie en masse à la bascule est une porte de fraude.** « Attester des autorisations papier » (FRM-185) permet à une seule personne de rendre « autorisés » 120 élèves d'un geste, sans contrôle ; or c'est **le seul chemin** pour reprendre l'existant (Next.js n'a rien, bloquant B du 42). | Une erreur ou une fraude (organisateur pressé) fait partir des élèves sans autorisation réelle, la preuve affichant « attestée ». | **Attestation à deux** (R-FC59) : en masse, l'attestation exige un **second** personnel (direction ou secrétariat) qui confirme ; **scan obligatoire** au-delà de 20 élèves pour un échantillon tiré au sort de 10 % ; l'attestation n'est permise que pour un **événement repris** (bascule), jamais pour un formulaire né dans ScolIA. | FO3 |
| **D** | **L'élève placé n'a personne pour son image.** L'image exige `tous` les titulaires ; pour un enfant confié à l'ASE dont les parents sont injoignables, le service gardien ne peut signer qu'en `signer_quotidien` : l'enfant reste **toujours** « non autorisé » — il disparaît de toutes les photos de classe, de l'album de fin d'année, du journal (P07). | Exclusion d'un enfant déjà fragilisé ; et la loi prévoit une **délégation** du droit à l'image (C. civ. 377, RF-3). | **Qualité `delegataire_image`** (R-FC60) : sur pièce (jugement de délégation, ou décision du juge autorisant le service gardien), le délégataire décide seul de l'image ; à défaut, **pour l'usage `fil_classe` seulement**, l'accord du service gardien vaut (acte d'organisation quotidienne [NV, Q-FC10]) ; les usages publics restent `non autorisé`. | FO1 |

---

## 1. Méthode
Seize personas (§3 de la conception), privé et public, familles ordinaires et particulières, personnels, un attaquant, un DPO et un administrateur de bascule. Même grille que `PERSONAS_17.md` ; gravité : bloquant · important · confort · différenciant. Personas construites, non interviewées [H].

## 2. Les personas en un tableau
| # | Persona | Établissement | Ce qu'elle éprouve surtout |
|---|---|---|---|
| P01 | **Mme Roche**, professeure d'EPS | lycée pilote | série piscine, sorties de proximité, autorisation annuelle |
| P02 | **M. Lemaire**, organisateur du séjour à Séville | lycée pilote | participation, image, soins, paiement, parent à l'étranger sans compte |
| P03 | **M. et Mme Garnier**, séparés, désaccord sur l'image | lycée pilote | ce que chacun voit |
| P04 | **Mme Petit**, ordonnance de protection, domicile caché | collège public | preuve et dossier lisibles du père |
| P05 | **Inès**, 13 ans, refuse sa photo | collège public | avis de l'enfant face aux parents |
| P06 | **Hugo**, 18 ans | lycée pilote | décide seul, parents payeurs |
| P07 | **Yanis**, 12 ans, confié à l'ASE, famille d'accueil | collège public | qui décide de son image et de ses sorties |
| P08 | **M. Roux**, secrétariat, campagne de rentrée | lycée pilote | 1 300 élèves, papiers, douchette, « comme l'an dernier » |
| P09 | **Mme Benali**, CPE | collège public | régime de sortie, contrôle à la porte |
| P10 | **Mme Dubois**, infirmière | lycée pilote | soins d'urgence, questionnaire de santé sans HDS |
| P11 | **M. Perrin**, président de l'APEL | lycée pilote | liste des parents (D111-8), retraits |
| P12 | **Mme Duval**, cheffe d'établissement | lycée pilote | enquête de climat anonyme, alerte dans un texte libre |
| P13 | **M. Blanc**, DPO | les deux | bases légales, journaux, sous-traitants, droit d'accès |
| P14 | **Les attaquants** : un élève de 15 ans, une secrétaire pressée, un parent | — | signer pour autrui, réidentifier, falsifier |
| P15 | **Florian**, administrateur, bascule d'avril et rentrée 2027 | lycée pilote | textes non relus, Next.js « OK » par défaut |
| P16 | **Mme Leroy**, professeure de français, projet de journal et de podcast | collège public | productions, voix, nom des élèves |

---

## 3. Parcours et manques

### P01 — Mme Roche, EPS
- **M18-001 (important)** **série** de dix séances de piscine (sortie obligatoire, dans l'emploi du temps) : **aucune autorisation** (RF-14) — la conception le dit (nature « information ») ; mais l'écran de création de sortie (42) doit proposer **par défaut « information »** pour une sortie obligatoire, et refuser un formulaire « autorisation » (sinon on fait signer à tort et l'on crée l'idée qu'un refus est possible) (R-FC61).
- **M18-002 (important)** **sorties facultatives de proximité** régulières (cross du district, UNSS le mercredi) : une autorisation **annuelle** couvrant « sorties facultatives de proximité sans nuitée, listées à l'avance » évite dix coupons ; objet `sorties_proximite_annuelles` (validité année, usages listés), chaque sortie de la liste restant informée (FRM-280).
- **M18-003 (confort)** UNSS : licence et autorisation parentale de l'association sportive : c'est l'**association** qui recueille [H] ; ScolIA peut porter le formulaire pour le compte de l'AS (objet à part, responsable de traitement l'établissement ou l'AS : Q-FC11).

### P02 — M. Lemaire, Séville
- **M18-004 (important)** un formulaire unique « participation + image + soins + paiement » (42 §11.2) : chaque accord **distinct** est bien prévu ; il manque **l'ordre** : on ne demande le paiement qu'**après** l'autorisation de participation (sinon paiement d'un élève non autorisé) (R-FC62).
- **M18-005 (important)** le père vit en Espagne, sans compte, numéro espagnol : lien nominatif + code par SMS **international** (coût, socle 8) ou e-mail vérifié ; le texte dans sa langue (espagnol) relu.
- **M18-006 (important)** voyage à l'étranger en règle `tous` : le père ne répond pas à J-10 ; l'organisateur doit pouvoir **basculer en papier** (AST du père de toute façon nécessaire) ; relance téléphonique tracée (« appelé le … ») (FRM-281).
- **M18-007 (confort)** la fiche « accords » hors ligne de l'encadrant (FRM-206) doit tenir sans réseau à Séville, **verdicts seulement**, effacée au retour.

### P03 — M. et Mme Garnier, désaccord sur l'image
- **M18-008 (bloquant)** état global : bloquant **B** (R-FC58).
- **M18-009 (important)** la mère accepte, le père refuse : `desaccord` ; la direction reçoit « désaccord à traiter » ; ce qu'elle peut faire est limité : **ne rien publier** et informer les deux, neutrement, qu'un accord commun ou une décision de justice est nécessaire (comme le socle 3 pour l'inscription) ; la conception le dit pour l'inscription, pas pour l'image : FRM-201 précisée.
- **M18-010 (important)** le père demande la **copie** de la décision de la mère : refusée (R-FC16) ; il peut demander son propre extrait (droit d'accès) ; le dossier de preuve du socle 7 « lisible par tous les titulaires invités » (R-D31) **contredit** R-FC16 : pour un **accord** du socle 18, le dossier de preuve n'est lisible que du décideur et de l'établissement (R-FC63, contradiction signalée au socle 7).

### P04 — Mme Petit, ordonnance de protection
- **M18-011 (important)** le père garde l'autorité parentale, n'est pas sous interdiction de communication avec l'établissement, mais la mère a une adresse protégée : il signe une autorisation ; **aucune** donnée de la mère (nom, date, canal) n'apparaît dans ce qu'il voit ; et l'inverse.
- **M18-012 (important)** l'ordonnance interdit au père d'**entrer en contact avec l'enfant** : c'est une interdiction de contact du socle 4 (objet `contact_eleve`), pas de communication avec l'établissement ; il reste décideur ; mais les autorisations qui **révèlent un lieu et une heure** (sortie, voyage, retour) ne doivent pas lui donner le lieu ni l'heure : il signe sur une **version épurée** (« sortie scolaire au musée, le 12 mars ») sans horaires ni lieu de retour (R-FC64) [NV, Q-FC12 au juriste].

### P05 — Inès, 13 ans
- **M18-013 (important)** son refus l'emporte (R-FC8) ; ses parents le voient (ils exercent le droit « en associant l'enfant ») ; elle subit une pression pour changer d'avis : son avis ne peut être modifié **que depuis son propre compte** (R-FC51), et un changement d'avis dans les 24 h qui suivent un accord parental est signalé discrètement au CPE si l'élève l'a demandé (« je veux qu'on respecte mon refus ») [R, différenciant] (FRM-282).
- **M18-014 (confort)** texte de l'avis en **langage d'enfant** (FALC), pas le texte juridique.

### P06 — Hugo, 18 ans
- **M18-015 (important)** ses parents payent le voyage : l'autorisation est **la sienne**, la demande de paiement va au **payeur** (socle 4/24) ; le payeur voit le montant et l'échéancier, **pas** les accords d'Hugo (image, soins).
- **M18-016 (confort)** à ses 18 ans, ses accords parentaux d'image cessent (R-FC12) : il est sollicité **une fois** avec « reprendre ce que vos parents avaient choisi » proposé, décoché.

### P07 — Yanis, confié à l'ASE
- **M18-017 (bloquant)** image : bloquant **D** (R-FC60).
- **M18-018 (important)** sorties facultatives : service gardien `signer_quotidien` ; voyage à l'étranger : non usuel → titulaires ou juge ; la conception renvoie au socle 4 (« [NV] ») : écrire que **l'écran de l'organisateur** affiche « décision du service gardien requise » ou « décision du juge requise » selon la nature, pour éviter qu'on fasse signer la famille d'accueil (R-FC65).
- **M18-019 (important)** la famille d'accueil n'est **jamais** décideur d'un consentement d'image, sauf délégation écrite du service (pièce).

### P08 — M. Roux, campagne de rentrée
- **M18-020 (important)** 1 300 élèves, 30 % de papier : **lecture du code** du coupon par douchette ou caméra (FRM-183) ; il faut un **mode rafale** (un coupon toutes les 5 secondes, retour sonore, annulation du dernier) (FRM-283).
- **M18-021 (important)** coupon signé par « un parent » sans nom lisible : CK-25 dit `en_attente` ; trop strict pour 400 coupons : si l'élève a **un seul** titulaire décideur, l'attribution est automatique ; sinon choix (R-FC66).
- **M18-022 (important)** « comme l'an dernier » : 70 % des familles le choisiraient [H] ; il doit fonctionner **par enfant** et **par objet**, et rester **explicite** (texte affiché).
- **M18-023 (confort)** la fiche de rentrée du socle 4 demande déjà l'image : **une seule** demande (rebranchement FO7) — sinon la famille signe deux fois.
- **M18-024 (important)** échéance : à la rentrée, la photo de classe a lieu en septembre, avant que les accords soient revenus : le photographe prestataire (`photo_classe_prestataire`) photographie tout le monde ? → la **prise de vue** pour la photo de classe vendue aux familles est un usage distinct ; sans accord, l'élève est photographié **seul** pour sa famille, pas sur la photo de groupe diffusée, ou pas du tout selon la règle de l'établissement (S18-13).

### P09 — Mme Benali, CPE
- **M18-025 (important)** régime de sortie (`sortie_seul`) : à la porte, l'AED doit savoir **en une seconde** si un élève peut sortir à 15 h : verdict lu par le socle 14 sur la **carte de l'élève**, **hors ligne** sur tablette (projection chiffrée, mise à jour toutes les 5 minutes) (FRM-284).
- **M18-026 (important)** changement en cours d'année (un parent retire l'autorisation de sortie libre) : effet **le jour même**, avis à la vie scolaire.
- **M18-027 (confort)** autorisation de sortie « en cas d'absence d'un professeur en dernière heure » : usage distinct déjà listé ; il faut l'afficher dans l'écran du 14 de façon lisible.

### P10 — Mme Dubois, infirmière
- **M18-028 (important)** sans HDS, le questionnaire de santé du voyage reste **papier sous pli** ; elle veut au moins savoir **qui a rendu le pli** : collecte C0 « pli reçu oui / non » (sans contenu) (FRM-285).
- **M18-029 (important)** `soins_urgence` : texte à faire relire (il ne doit pas laisser croire qu'un refus empêche les secours) (R-FC67).
- **M18-030 (confort)** autorisation de **prise de médicaments** pendant la sortie : relève du PAI (socle 34), pas du 18.

### P11 — M. Perrin, APEL
- **M18-031 (important)** il reçoit la liste des parents qui ont accepté (D111-8) : par **export du secrétariat** (socle 4 ADM-67) limité aux accords valides **à la date de l'export**, avec la date en tête ; jamais par un accès direct de l'APEL à ScolIA.
- **M18-032 (important)** un parent retire son accord après la transmission : alerte au secrétariat (CK-64) ; l'APEL reçoit une **liste des retraits** depuis la dernière transmission (identité seulement, sans motif) (FRM-286).
- **M18-033 (confort)** l'APEL veut sonder les parents : un sondage **de l'association** relayé par l'établissement, anonyme, résultats agrégés à l'association (Q-FC11).

### P12 — Mme Duval, enquête de climat anonyme
- **M18-034 (important)** ventilation par classe : classes de 25, 12 réponses : cellules ≥ 5 → affichées ; elle veut croiser classe × sexe : refusé (cellules < 5) ; l'écran explique pourquoi.
- **M18-035 (important)** un texte libre dit « je suis harcelée par X en 4ᵉ2 » : l'établissement **ne peut pas** retrouver l'auteur (c'est la promesse) ; il doit pouvoir **agir sur l'information** (la classe nommée) ; et avant le texte libre, un **encadré** renvoie au parcours « signaler un harcèlement » (socle 16 MSG-336), au 3018 et au 119 (R-FC68).
- **M18-036 (important)** les résultats ne doivent pas être lus **pendant** la collecte (on devinerait les derniers répondants) : résultats visibles **à la clôture** seulement pour un anonyme (R-FC69).

### P13 — M. Blanc, DPO
- **M18-037 (bloquant)** journaux et anonymat : bloquant **A** (R-FC57).
- **M18-038 (important)** **base légale** affichée pour chaque objet et chaque formulaire, générée depuis la nature (FRM-251) ; il veut aussi la **finalité** (texte court obligatoire) et le **destinataire** des réponses.
- **M18-039 (important)** le **photographe** prestataire : contrat de sous-traitance, transfert des seuls élèves autorisés, effacement après la livraison ; l'export vers le prestataire est une action à part, journalisée (FRM-287).
- **M18-040 (important)** **droit d'accès** : un parent demande « tout ce que vous avez sur moi » : décisions, soumissions **nominatives**, participations aux sondages anonymes (pas les réponses, que l'établissement ne peut pas relier) — c'est à dire dans l'extrait (FRM-254 précisée).
- **M18-041 (important)** **conservation des preuves après retrait** : la preuve du consentement retiré est conservée (preuve de l'usage passé) le temps de la prescription utile [NV] ; le contenu de l'usage, lui, est retiré.
- **M18-042 (important)** **IA** : synthèse des réponses libres nominatives → texte envoyé au fournisseur ; pseudonymisation des noms avant envoi, et jamais pour un formulaire C2 (R-FC70).
- **M18-043 (confort)** AIPD : un modèle pré-rempli **par objet du catalogue** (image, soins, sondage anonyme).

### P14 — Les attaquants
**(a) Théo, 15 ans**
- **M18-044 (important)** il signe l'autorisation de sortie **avec le téléphone de sa mère** (code SMS reçu sur ce téléphone) : non détectable sûrement (CK-56) ; atténuations : avis **à l'autre titulaire** (« une autorisation a été donnée pour Théo ») qui peut s'opposer (R-FC7) ; S2 (passkey) pour les actes non usuels ; une autorisation donnée **pendant les heures de cours depuis l'établissement** (réseau de l'établissement) est signalée à la vie scolaire [R] (R-FC71).
- **M18-045 (important)** il « vote » deux fois à un sondage anonyme avec le compte de sa sœur : la participation est par **personne** (sa sœur peut participer, une fois) ; rien à faire de plus sans casser l'anonymat — admis.
- **M18-046 (important)** il répond juste après son ami pour l'identifier dans les résultats : lots mélangés + résultats à la clôture (R-FC69) le rendent inopérant.
- **M18-047 (important)** il modifie l'identifiant dans `GET /formulaires/{id}/reponses` : refus (auteur ou habilité), test d'isolation.
- **M18-048 (confort)** injection de formule dans un texte libre : neutralisée à l'export (R-FC40).
**(b) Une secrétaire pressée à la bascule**
- **M18-049 (bloquant)** attestation en masse : bloquant **C** (R-FC59).
**(c) Un parent**
- **M18-050 (important)** il tente de répondre pour l'enfant d'une autre famille en changeant l'identifiant de l'élève : décideurs calculés côté serveur, réponse « introuvable » (pas de fuite d'existence) — conservé de l'existant.
- **M18-051 (important)** il partage le **lien nominatif** de l'autre parent (sans compte) qu'il a vu passer : le lien seul ne suffit pas, le code part au **canal vérifié** du titulaire visé (R-FC48).

### P15 — Florian, bascule et rentrée 2027
- **M18-052 (important)** les **textes** du catalogue n'ont pas été relus par un juriste : FO3 « production » ne doit pas être activé sur des données réelles avant la relecture (garde : objet « non relu » ⇒ bandeau « texte provisoire » et interdiction d'usage réel) (R-FC72).
- **M18-053 (important)** Next.js « droit à l'image OK » pour 600 élèves de voyages passés : `inconnu` ; mais les **photos déjà publiées** dans le journal de voyage Next.js (blog parents) ne sont **pas** reprises (perdables, T0 M01) : à écrire.
- **M18-054 (important)** rentrée 2027 : le renouvellement d'août (FO8) est le **premier** recueil réel ; la campagne doit partir **après** l'activation des comptes familles (socle 4), sinon 1 300 courriers.
- **M18-055 (confort)** pendant le doublon, Charlemagne a peut-être un champ « droit à l'image » : **ne pas l'importer** comme consentement (même règle que Next.js) ; au plus comme indice « oui l'an dernier » affiché à la famille dans « comme l'an dernier » ? Non : rien qui ressemble à un accord sans preuve (R-FC43 confirmée).

### P16 — Mme Leroy, journal et podcast
- **M18-056 (important)** **productions** : un article écrit par trois élèves ; chacun est auteur ; l'accord `publication_productions` de **chacun** pour l'usage visé ; le mode de signature le plus **restrictif** des trois s'applique à la signature commune (« prénoms » si l'un refuse le nom) (FRM-288, précise R-PB27 du socle 17).
- **M18-057 (important)** **voix** dans un podcast diffusé sur le site : usage `diffusion_publique` (site N3 absent) → en attendant, diffusion **interne** seulement.
- **M18-058 (confort)** un élève quitte l'établissement : ses productions déjà publiées restent ? R-FC49 retire les usages à départ + 30 jours ; pour une **œuvre** (journal annuel imprimé), le retrait ne vaut pas pour l'exemplaire imprimé (CK-33) ; dans l'ENT, l'accord reste valable jusqu'à sa date (année) puis expire : précisé.
- **M18-059 (confort)** nom de l'élève dans un palmarès (concours) : `mention_nominative`.

### Manques transverses : mobile, IA, tests
- **M18-060 (important)** mobile : « mes accords » et le retrait doivent être **aussi simples sur téléphone** que le don (RGPD) — FO10 avancé pour cet écran.
- **M18-061 (important)** jeu d'évaluation IA : formulaires proposés contenant une question de santé ou de religion (le lint doit l'attraper), réponses libres avec injection.
- **M18-062 (important)** tests de corrélation de l'anonymat **avec les journaux** (bloquant A).

---

## 4. Vérification réglementaire (2026-11-11)
Faite par sous-agent pour les deux socles ; versée au §4 de `18_FORMULAIRES.md` (RF-1 à RF-20). La passe ajoute : C. civ. **377** (délégation partielle du droit à l'image, RF-3) comme fondement de la qualité `delegataire_image` (bloquant D) ; la recommandation 4 de la CNIL (RF-8 : un parent, l'autre « mis en capacité » de s'opposer) comme fondement de l'avis au second titulaire (M18-044).

## 5. Manques qui relèvent d'autres socles
| Manque | Socle |
|---|---|
| Dossier de preuve d'un accord lisible du seul décideur (M18-010, contradiction avec R-D31) | 7 |
| Interdiction de contact avec l'élève et version épurée (M18-012) | 4, 42 |
| Verdict de sortie hors ligne à la porte (M18-025) | 14, 11 |
| Ordre participation → paiement (M18-004) | 42, 24 |
| Comptes familles activés avant la campagne de rentrée (M18-054) | 4 |
| Journaux d'accès sans IP pour la route anonyme (M18-037) | 1 (observabilité), infra |

## 6. Synthèse
**Par gravité** : 4 bloquants (A à D), 44 importants, 12 confort, 2 différenciants.
**Les dix ajouts les plus importants** : route anonyme hors journaux (A) ; aucun état global aux familles (B) ; attestation à deux (C) ; délégataire de l'image (D) ; sortie obligatoire = information par défaut ; autorisation annuelle de proximité ; avis de l'enfant protégé contre la pression ; résultats anonymes à la clôture seulement ; version épurée pour un titulaire sous interdiction de contact ; textes non relus interdits d'usage réel.
**Ce que la passe a montré** : le cœur juridique (natures, décideurs, versions, port) tient ; les faiblesses étaient **hors de la base** (journaux, téléphone partagé, papier en masse) et dans les **familles réelles** (ASE, protection, enfant sous pression).

## 7. Décisions
### 7.1 Appliquées par défaut (D-031)
A, B, C, D ; R-FC57 à R-FC72 ; FRM-280 à FRM-294 ; S18-13, S18-14.
### 7.2 Soumis à Florian, au DPO, au juriste
1. **Juriste** : accord du service gardien pour `fil_classe` (Q-FC10) ; version épurée sous interdiction de contact (Q-FC12).
2. **DPO** : journaux d'accès sans IP pour la route anonyme (exploitation), durée de conservation des preuves après retrait.
3. **Florian** : règle de la photo de classe sans accord (S18-13) ; responsable de traitement des formulaires de l'AS et de l'APEL (Q-FC11) ; relecture des textes avant la rentrée 2027 (R-FC72).

## 8. Faits, hypothèses, contradictions
- **Contradiction** : `07_DOCUMENTS.md` R-D31 (dossier de preuve lisible par tous les titulaires invités) contre R-FC16 / R-FC63 (pour les accords du 18 : décideur et établissement seulement). Recommandation : R-FC63 pour les accords ; R-D31 reste pour les documents à signer du 7.
- **Contradiction** : `18_FORMULAIRES.md` §7.2 (état global affiché) → remplacé par R-FC58.
- **Hypothèses** : part des familles « papier » (30 %), part de « comme l'an dernier » (70 %), pratiques UNSS.
