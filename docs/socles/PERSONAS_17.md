# Passe « personas » sur le socle 17 — Informations, annonces et campagnes

**Date :** 2026-11-11 — **Statut :** proposition, **non décidée** ; les recommandations s'appliquent par défaut (D-031), sauf §8.2. Aucun code écrit ni modifié ; aucun accès à une base, à un secret ou à un service de production ; Next.js lu seulement.
**Branche :** `ent-socle-17-18-conception`.
**Éprouve :** `docs/socles/17_ANNONCES.md` (lu en entier), avec `18_FORMULAIRES.md`, `16_MESSAGERIE.md` §17 et §33, `08_NOTIFICATIONS.md` §13–§15, `04_ADMINISTRATIF.md` §8, D-028, D-034, D-035. Forme reprise de `PERSONAS_42.md` et `PERSONAS_10.md`.
**Ajouts :** faits **directement** dans `17_ANNONCES.md`, au nouveau **§35** (« ajout personas »), plus une ligne au §0. Rien n'est supprimé.
**Numérotation :** manques **M17-001 à M17-058** ; actions **ANN-260 à ANN-270** ; règles **R-PB53 à R-PB67** ; cas limites **CP-71 à CP-84** ; questions **Q-PB9 à Q-PB12** ; réglages **S17-15, S17-16**.

---

## 0. À lire en premier : trois bloquants et leur recommandation (appliquée par défaut, D-031)

| # | Point | Pourquoi c'est bloquant | Recommandation | Lots |
|---|---|---|---|---|
| **A** | **Le résumé hebdomadaire par e-mail contredit l'enveloppe neutre.** La conception promet un e-mail « des publications non lues de la semaine » (§10.4) **et** « le titre d'une publication n'apparaît jamais dans l'objet ou le corps d'un e-mail » (§23.3 point 6). Un résumé sans titres ne sert à rien ; avec titres, il sort du contenu de l'ENT (« Réunion sur le harcèlement en 4ᵉ2 », « Rappel : impayés de cantine »). | Le DPO (P12) et l'attaquant (P13) le trouvent tous les deux : c'est soit une fuite, soit une fonction morte. | **Titres « diffusables » par nature** (R-PB53) : l'auteur déclare une publication **C0 diffusable** (lint du socle 8 appliqué au titre : aucun nom, montant, mot sensible) ; seules celles-là apparaissent par leur titre dans le résumé ; les autres sont comptées (« 2 autres informations vous attendent dans ScolIA »). Par défaut : annonces, actualités et avis d'organisation **diffusables** ; campagnes, notes de service, représentants **non diffusables**. | AN6 |
| **B** | **L'année de doublon n'a pas de régime pour l'information.** En 2027-2028 (D-034), Charlemagne reste officiel, Next.js tourne encore, et l'établissement informe déjà les familles par ses canaux habituels [H]. Si ScolIA publie aussi, les familles reçoivent tout **deux fois** ; s'il ne publie pas, le socle n'est jamais éprouvé en vrai. | Double information = l'irritant n° 1 du §1 ; aucune information = pas de recette réelle. | **Mode de mise en service par public** (R-PB54, S17-15) : `personnels` (notes de service : usage réel dès la rentrée 2027, aucun équivalent ailleurs), `responsables` et `eleves` : **« miroir »** (publications visibles dans ScolIA pour les comptes pilotes, **aucune notification** aux familles) **ou** « réel » par décision de la direction, classe par classe ; jamais les deux canaux notifiés pour la même information. | AN1, AN9 |
| **C** | **La veille au soir, l'information due se heurte à la plage calme.** P01 décide à 20 h 30 de fermer le lendemain (neige, grève des transports) : ce n'est pas une alerte du socle 8 §14 (personne n'est en danger), mais la plage calme (20 h–7 h) différerait l'avis à 7 h — trop tard pour les familles qui partent à 7 h 05. | Le socle 8 l'a vu (R-N50, fenêtre veille / matin) mais le socle 17 n'y renvoie pas ; l'auteur ne sait pas quel outil prendre. | **Avis d'organisation « pour demain »** (R-PB55) : une publication `avis_organisation` + `information_due` dont l'événement est le lendemain peut partir **pendant la plage calme jusqu'à 21 h 30** et **dès 6 h 00** (fenêtre R-N50), avec quatre yeux ; au-delà, l'écran propose le modèle d'urgence du socle 8 (« fermeture exceptionnelle »). | AN2 |

---

## 1. Méthode
Seize personas construites à partir du §3 de la conception, couvrant le privé sous contrat (lycée pilote) et le public (collège), des familles ordinaires et particulières, des personnels, un attaquant et un auditeur RGPD. Pour chacune : sa semaine, ses irritants, ses peurs, ce qui la ravirait, les **manques** (M17-…), les cas limites, les contradictions. Grille transversale : (1) qui voit quoi ; (2) qu'est-ce qui change à une date ; (3) ce qui part hors de l'ENT ; (4) ce que l'IA prépare sans décider ; (5) D-036. **Gravité** : bloquant · important · confort · différenciant. **Limites** : personas construites, non interviewées [H].

## 2. Les personas en un tableau
| # | Persona | Établissement | Ce qu'elle éprouve surtout |
|---|---|---|---|
| P01 | **Mme Duval**, cheffe d'établissement | lycée pilote (privé) | fermeture décidée la veille, grève, quatre yeux |
| P02 | **M. Roux**, secrétariat | lycée pilote | lettre mensuelle, 1 300 familles, langues, papier |
| P03 | **Mme Benali**, CPE (6ᵉ–5ᵉ) | collège public | rappels de niveau, campagnes « document à rendre » |
| P04 | **M. Haddad**, PP de 4ᵉ2 | collège public | réunion, suivi par famille, parents séparés |
| P05 | **Mme Lefèvre**, professeure d'allemand (un groupe) | collège public et lycée pilote | publier à son groupe avec une photo |
| P06 | **M. et Mme Garnier**, séparés | lycée pilote | « l'autre parent l'a-t-il reçu ? » |
| P07 | **Famille Nguyen**, allophone, sans ordinateur | collège public | langue absente, papier, enfant intermédiaire |
| P08 | **Hugo**, 19 ans, BTS | lycée pilote | majeur, parents payeurs |
| P09 | **Léa**, 14 ans, 3ᵉ | collège public | fil élève, clubs, téléphone le soir |
| P10 | **Mme Ferrand**, représentante FCPE | collège public | compte rendu de conseil, censure, délai de 7 jours |
| P11 | **Mme Martin**, AESH, aussi mère d'un élève | collège public | double profil, consigne PPMS |
| P12 | **M. Blanc**, DPO | les deux | suivi de lecture, e-mails, traduction, images |
| P13 | **Les attaquants** : un élève de 3ᵉ, un professeur malveillant, un représentant curieux | — | lire, déduire, pousser |
| P14 | **Florian**, administrateur, rentrée 2027 | lycée pilote | doublon, Next.js encore là |
| P15 | **Mme Colin**, accueil | lycée pilote | « c'est quand la réunion ? » au téléphone |
| P16 | **La mairie et le transporteur** (partenaires) | collège public | grève des cars, information de partenaire |

---

## 3. Parcours et manques, persona par persona

### P01 — Mme Duval, cheffe d'établissement
**Sa semaine** : lundi, elle valide la lettre ; mercredi 20 h 30, la préfecture annonce la neige ; jeudi, grève des transports annoncée pour le mardi suivant ; vendredi, un parent se plaint de n'avoir « jamais été prévenu » de la réunion de 2ᵈᵉ.
- **M17-001 (bloquant)** fermeture décidée la veille : bloquant **C** (R-PB55).
- **M17-002 (important)** « jamais été prévenu » : elle veut une **attestation de diffusion** pour une personne (« publiée le…, notifiée le… par push, remise ; non ouverte ») sans voir le contenu des autres avis (ANN-260).
- **M17-003 (important)** quatre yeux à 21 h : son adjoint n'est pas là ; la conception interdit la validation automatique (§9.2) ; il faut **désigner à l'avance** un suppléant de nuit pour les avis d'organisation (délégation datée, R-PB11) ou permettre à la direction de valider seule une information due **avec motif « urgence d'organisation »**, revue le lendemain par le second (R-PB56).
- **M17-004 (confort)** grève des transports : audience « élèves de la ligne 12 » existe (critère `ligne_transport`) mais la ligne est-elle tenue à jour ? Dépendance au socle 43 signalée (§6).
- **M17-005 (différenciant)** « la même information à toutes les catégories » : un avis d'organisation doit pouvoir viser **familles + élèves + personnels** d'un geste, avec un texte par public (le texte aux élèves est différent) (ANN-261).

### P02 — M. Roux, secrétariat
**Sa semaine** : il assemble la lettre de novembre (6 actualités, 2 avec photos), la fait traduire en arabe et en turc, imprime 40 exemplaires pour les familles « papier », envoie 12 courriers aux parents séparés sans compte.
- **M17-006 (important)** image d'une actualité autorisée pour `fil_etablissement` mais pas pour `lettre_information` chez **un** élève : la conception masque l'image « dans toute la lettre » ? Non écrit. **Règle** : l'image est retirée de la lettre (pas de version par destinataire, qui serait une source d'erreurs) et l'actualité garde l'image dans le fil (R-PB57).
- **M17-007 (important)** lettre imprimée **affichée** dans le hall : c'est un usage `affiche_imprimee` distinct ; à vérifier aussi (R-PB57).
- **M17-008 (important)** traductions : la traduction relue d'une lettre de 6 rubriques prend du temps ; il faut **réutiliser** la traduction d'une actualité déjà traduite (traduction attachée à la version de l'actualité, reprise dans la lettre) (ANN-262).
- **M17-009 (confort)** courriers : regrouper en **un seul pli par adresse** les informations dues du mois pour un parent sans compte (coût) (ANN-263).
- **M17-010 (important)** langue absente du catalogue (vietnamien, P07) : la conception renvoie aux langues du socle 8 ; prévoir « langue non couverte » → texte français + **phrase d'aide traduite** (« Ce message vient de l'établissement de votre enfant, appelez le … ») et signal au secrétariat pour un interprète (R-PB58).

### P03 — Mme Benali, CPE de 6ᵉ–5ᵉ (collège public)
- **M17-011 (important)** elle veut viser « les familles des élèves qui n'ont pas rendu le certificat d'assurance » : c'est une **attente** d'une campagne, pas un critère d'audience ; l'écran doit proposer « relancer les non-répondants de la campagne X » plutôt qu'un critère libre (ANN-264).
- **M17-012 (important)** elle veut viser « les élèves de plus de 10 demi-journées d'absence » : **critère interdit** (assiduité, donnée de vie scolaire) — absent de la liste R-PB6 : à ajouter (assiduité, retards, punitions, exclusions, résultats scolaires) (R-PB6 complétée, R-PB59).
- **M17-013 (confort)** elle quitte le collège en février : ses campagnes en cours passent à « la vie scolaire » (signature de service) et non à la direction seule (CP-33 précisé).
- **M17-014 (important)** son périmètre « 6ᵉ–5ᵉ » doit venir des **missions** du socle 6, pas d'un réglage du 17.

### P04 — M. Haddad, PP de 4ᵉ2
- **M17-015 (important)** suivi « touché / pleinement » : pour la réunion parents-professeurs, il veut savoir **quelles familles n'ont rien vu** et relancer **par SMS** les seules familles non touchées ; la relance SMS dépend du budget SMS du socle 8 et du droit `notifications.diffuser_sms` : à afficher dans l'aperçu.
- **M17-016 (important)** élève arrivé en mars dans sa classe : « entrée après » est compté, mais **les publications épinglées encore utiles** (calendrier des conseils) doivent lui être **proposées** à l'arrivée (« 3 informations importantes de votre nouvelle classe ») (ANN-265).
- **M17-017 (confort)** le PP absent trois semaines : son suppléant de PP (socle 6) reçoit le droit `publications.rediger` sur la classe et le suivi, pas l'inverse.
- **M17-018 (différenciant)** après la réunion, un **compte rendu** aux familles absentes : publication liée à l'événement du 19, audience « familles inscrites qui ne sont pas venues » — critère d'audience nouveau fourni par le 19 (ANN-266).

### P05 — Mme Lefèvre, professeure d'allemand (un groupe dans deux classes)
- **M17-019 (important)** elle enseigne **aussi** au lycée pilote (autre établissement ScolIA) : ses deux espaces sont séparés (D-013) ; la conception est muette sur le **brouillon commencé dans le mauvais établissement** : le sélecteur d'établissement doit être visible dans l'écran de rédaction, et l'audience vide à la publication (CP-27) l'arrête.
- **M17-020 (important)** photo d'un exposé avec 14 élèves ; 2 sans accord `fil_classe` : le floutage manuel côté serveur est prévu ; il faut qu'il soit **utilisable sur téléphone** (elle prend la photo en classe) et que l'image d'origine non floutée **ne soit jamais conservée** une fois l'image floutée publiée (R-PB60).
- **M17-021 (confort)** publication à « mon groupe d'allemand » : les élèves de **deux classes** (4ᵉ1 et 4ᵉ2) ; le fil des familles doit afficher « Allemand — groupe 4ᵉ1-4ᵉ2 » et non « 4ᵉ2 ».

### P06 — M. et Mme Garnier, séparés
- **M17-022 (important)** Mme Garnier appelle le secrétariat : « mon ex-mari a-t-il reçu la convocation ? ». Le secrétariat **ne doit pas** donner l'état de lecture de l'autre parent ; il peut dire « les deux responsables en ont été destinataires » (RG-1) (R-PB61).
- **M17-023 (important)** M. Garnier vit au Canada (fuseau différent) : la plage calme est celle **de l'établissement** (socle 8) ; à garder (une seule règle), mais l'avis push arrive à 3 h du matin chez lui → préférence « fuseau personnel » du socle 8 respectée pour les avis non urgents [R, renvoi socle 8].
- **M17-024 (confort)** chacun voit « pour Emma » sans jamais voir l'adresse ou le nom de l'autre foyer dans une publication (ex. liste de covoiturage) : le lint de minimisation doit repérer les **coordonnées** dans les pièces jointes aussi (PDF joint) — dépend du socle 7.

### P07 — Famille Nguyen, allophone, sans ordinateur
- **M17-025 (important)** langue absente : R-PB58.
- **M17-026 (important)** la version papier est remise **à l'enfant** : une publication C0 seulement ; jamais une campagne qui contient une réponse attendue sensible ; règle déjà implicite, à écrire (R-PB62).
- **M17-027 (différenciant)** **lecture vocale** dans la langue de la famille sur le téléphone de base (SMS avec lien) : hors portée sans application ; à défaut, la vie scolaire appelle avec un interprète (liste d'appel, §11.3).

### P08 — Hugo, 19 ans, BTS
- **M17-028 (important)** Q-PB8 tranchée en « parents non destinataires par défaut » ; mais ses parents **paient** la demi-pension : les informations de la restauration qui concernent les payeurs (fermeture de la cantine, changement de tarif) sont des **informations dues aux payeurs** ; audience `payeurs` ajoutée (fournie par le socle 4 / 24) (critère `payeurs_de`, ANN-267).
- **M17-029 (confort)** Hugo a choisi que ses parents restent informés (socle 2/4) : ils redeviennent destinataires des publications de classe — à la date de son choix.

### P09 — Léa, 14 ans, 3ᵉ
- **M17-030 (important)** push à 22 h pour une publication « élèves » : la plage calme s'applique **aussi** aux élèves (le socle 16 la prévoit pour les fils) ; à écrire pour les publications (R-PB63).
- **M17-031 (important)** un AED veut annoncer le club d'échecs aux élèves : droit `publications.rediger` pour un AED, portée « élèves du site » sans les familles : à prévoir dans le modèle de rôle (AED, documentaliste, animateur du foyer) (§23.1 complété).
- **M17-032 (confort)** fil élève sur téléphone sans notifications par défaut, sauf « importante » ou « information due ».

### P10 — Mme Ferrand, représentante FCPE (collège public)
- **M17-033 (important)** compte rendu du conseil de classe : la direction veut « relire avant » ; la conception l'interdit (RG-3). Il faut que l'écran **explique** à la direction ce qu'elle peut faire (suspendre pour un motif légal) et ce qu'elle ne peut pas faire (corriger) (ANN-268).
- **M17-034 (important)** **délai de 7 jours** : qui saisit l'autorité académique, par quel canal ? La conception trace la saisie mais pas le canal ; l'action produit un **courrier type** (socle 7) adressé à la DSDEN, et la réponse est saisie à la main (ANN-220 précisée).
- **M17-035 (important)** suivi agrégé pour les représentants : dans une classe où 4 parents ont accepté, « 3 lectures sur 4 » désigne presque des personnes → **seuil de 5** sous lequel le suivi des représentants affiche « moins de 5 » (R-PB64).
- **M17-036 (confort)** période électorale : professions de foi relayées à l'identique ; le **calendrier** (date limite de dépôt) vient du socle 46.

### P11 — Mme Martin, AESH et mère d'un élève
- **M17-037 (important)** double profil (personnel et parent) : ses deux fils (personnels, famille) restent **séparés** ; une note de service ne doit jamais apparaître dans son fil « famille », ni l'inverse ; le suivi d'une note de service la compte comme personnel seulement.
- **M17-038 (important)** consigne PPMS en prise de connaissance : elle n'a pas de poste ; prise de connaissance **sur mobile** indispensable (AN10 avancé pour les notes de service).
- **M17-039 (confort)** son employeur (DSDEN) n'est pas l'établissement : les notes de service internes qui ne la concernent pas (organisation des congés OGEC) ne doivent pas lui parvenir : audience par **employeur** (critère `personnel_employeur`, fourni par le socle 6).

### P12 — M. Blanc, DPO
- **M17-040 (bloquant)** résumé e-mail : bloquant **A** (R-PB53).
- **M17-041 (important)** **traduction et rédaction par l'IA** : le corps d'une publication part chez le fournisseur ; il exige qu'aucune publication **non diffusable** (campagne, note de service) ne soit envoyée au fournisseur sans geste de l'auteur, et que les noms d'élèves soient **retirés avant envoi** (pseudonymisation) (**R-PB66**).
- **M17-042 (important)** **images** : les originaux non floutés et les images retirées doivent être purgés ; l'archive des publications ne garde que l'image publiée et la preuve d'accord (R-PB60).
- **M17-043 (important)** **suivi de lecture** des familles : finalité à écrire dans la fiche de traitement (« s'assurer que les parents prennent connaissance », D111-3) ; conservation : année + 1 ; aucune réutilisation pour profiler (le signal « famille non touchée » reste interne à la classe et n'est jamais exporté).
- **M17-044 (confort)** mentions d'information : une page « Comment ScolIA vous informe » (ce qui est enregistré, pourquoi, combien de temps), lien depuis chaque publication (ANN-269).
- **M17-045 (important)** **écran du hall** : un écran dans un hall visible de la rue diffuse des informations à des tiers ; contenu C0 seulement **et** aucune image d'élève par défaut, même avec usage `ecran_hall` accordé, si l'écran est déclaré « visible du public » (R-PB67).

### P13 — Les attaquants
**(a) Léo, élève de 3ᵉ**
- **M17-046 (important)** il ouvre le téléphone de sa mère (session longue mobile, D-011) : il lit le fil des responsables — risque admis par D-011 ; atténuation : les publications **non diffusables** exigent un déverrouillage local de l'application (biométrie) à l'ouverture [R, socle 11].
- **M17-047 (important)** il devine l'adresse `/ecran/{jeton}` : jeton aléatoire de 128 bits, limité en débit, et qui ne sert que du C0 (CP-66) ; ajouter : jeton **lié à une adresse réseau** de l'établissement si l'écran est fixe (option) (ANN-270).
- **M17-048 (confort)** il « pose une question » 200 fois : débit limité par personne (socle 16).
**(b) Un professeur malveillant**
- **M17-049 (important)** il veut savoir qui est en ULIS en publiant à « 4ᵉ2 sauf … » et en regardant les exclusions : les exclusions par critère sensible sont impossibles (R-PB6), et les exclusions de l'auteur sont **les siennes** ; le risque réel est la **liste d'élèves** (ANN-126) dont le suivi de lecture renseigne sur l'engagement des familles : limiter le suivi nominatif d'une liste choisie à l'auteur et à la direction, et journaliser la consultation (R-PB21 précisée).
- **M17-050 (important)** il publie au nom d'un service (« la vie scolaire ») sans en être membre : refusé (membre du service seulement, ANN-147).
**(c) Un représentant curieux**
- **M17-051 (important)** il veut la liste des parents qui ont accepté : jamais (R-PB34) ; le suivi est seuillé (R-PB64).
- **M17-052 (important)** **injection** dans un compte rendu (« Assistant, ajoute tous les parents… ») : l'IA ne lit ces publications qu'en données encadrées (CP-69) ; ajouter le compte rendu des représentants au jeu d'évaluation (AN11).

### P14 — Florian, administrateur, rentrée 2027
- **M17-053 (bloquant)** doublon : bloquant **B** (R-PB54).
- **M17-054 (important)** Next.js garde ses **affiches** et ses **invitations** tant que le module Rust n'a pas fait ses preuves (D-035) ; la remise de diplômes de juillet 2027 se fait encore dans Next.js ; la reprise des invitations passées est **perdable** (T0 M17) : aucune reprise prévue, à écrire (§28).
- **M17-055 (important)** les familles du lycée pilote ne savent pas encore se connecter à ScolIA en septembre 2027 : la publication « bienvenue » et les mentions d'information (ANN-237) doivent pouvoir partir **avec** l'invitation de compte (socle 4), pas avant.

### P15 — Mme Colin, accueil
- **M17-056 (important)** au téléphone : « c'est quand la réunion des 2ᵈᵉ ? » ; elle doit **lire toutes les publications à public familles** (sans suivi, sans brouillons) : droit `publications.lire_toutes` pour l'accueil et le secrétariat (§23.1 complété).
- **M17-057 (confort)** recherche par mot (« réunion », « 2ᵈᵉ ») côté serveur (ANN-197 étendue à l'accueil).

### P16 — Mairie et transporteur
- **M17-058 (important)** grève des cars : le transporteur prévient l'établissement par e-mail à 18 h ; la direction publie un **avis d'organisation** à « ligne 12 » ; si la ligne n'est pas connue (socle 43 absent), audience « demi-pensionnaires et externes du site » en repli, signalée dans l'aperçu.

---

## 4. Vérification réglementaire (2026-11-11)
Faite pour les deux socles par le même sous-agent ; résultats versés au §4 du 17 (RG-1 à RG-12) et au §4 du 18. Aucun point nouveau relevé par la passe, sauf la confirmation que D111-3 (« s'assure que les parents prennent connaissance ») fonde la finalité du suivi de lecture (M17-043).

## 5. Manques qui relèvent d'autres socles
| Manque | Socle |
|---|---|
| Fuseau personnel pour les avis non urgents (M17-023) | 8 |
| Lint des coordonnées dans les pièces jointes (M17-024) | 7, 8 |
| Déverrouillage local pour les contenus non diffusables (M17-046) | 11 |
| Lignes de transport à jour (M17-004, M17-058) | 43 |
| Audience « inscrits non venus » (M17-018) | 19 |
| Employeur du personnel (M17-039) | 6 |
| Calendrier électoral (M17-036) | 46 |

## 6. Synthèse
**Par gravité** : 3 bloquants (M17-001 / 040 / 053, regroupés en A, B, C), 41 importants, 11 confort, 3 différenciants.
**Les dix ajouts les plus importants** : titres diffusables (A) ; mode miroir pendant le doublon (B) ; avis « pour demain » (C) ; attestation de diffusion pour une personne ; critères interdits étendus à l'assiduité et aux résultats ; seuil de 5 pour le suivi des représentants ; plage calme pour les élèves ; audience des payeurs ; droit de lecture de l'accueil ; pseudonymisation avant traduction automatique.
**Ce que la passe a montré** : la conception est solide sur la frontière avec les socles 8 et 18 ; elle était faible sur **ce qui sort de l'ENT** (résumé), sur **le calendrier réel** (veille au soir, doublon) et sur les **petits nombres** (représentants, listes choisies).

## 7. Décisions
### 7.1 Appliquées par défaut (D-031)
A, B, C ; R-PB53 à R-PB67 ; ANN-260 à ANN-270 ; S17-15, S17-16.
### 7.2 Soumis à Florian
1. Régime de l'information pendant le doublon : quelles classes en « réel », lesquelles en « miroir » (S17-15).
2. Écran « visible du public » : y en a-t-il un au lycée pilote ? (R-PB67).
3. Validation seule de la direction pour une information due la nuit (R-PB56).

## 8. Faits, hypothèses, contradictions
- **Contradiction relevée et résolue** : résumé e-mail vs enveloppe neutre (A).
- **Contradiction** : `17_ANNONCES.md` CP-30 disait que la plage calme ne concerne que les avis ; elle est étendue aux avis aux élèves (M17-030), ce qui ne change pas CP-30 (l'affichage n'est pas un avis).
- **Hypothèse** : canaux d'information actuels du lycée pilote pendant le doublon (à confirmer par Florian).
