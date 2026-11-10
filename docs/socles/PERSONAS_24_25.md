# Passe « personas » sur les socles 24 (facturation) et 25 (paiements, SEPA, recouvrement)

**Date :** 2026-11-10 — **Statut :** proposition, **non décidée** (à relire par Florian) ; les recommandations s'appliquent par défaut (D-031), sauf ce qui engage l'établissement à l'extérieur (§8). Aucun code écrit ni modifié ; aucun accès à une base, à un secret, à un compte bancaire ni à un service de production ; l'application Next.js n'a pas été ouverte.
**Branche :** `ent-socle-finances-conception`, mise à jour sur `origin/ENT` `ee7706d3` (qui porte désormais les correctifs RS3-01, RS3-02, RS3-04 et les migrations 376–377).
**Éprouve :** `docs/socles/24_FINANCES.md` et `docs/socles/25_PAIEMENTS.md` (lus en entier), avec `CLAUDE.md`, `docs/DECISIONS.md` (D-013, D-014, D-017, D-023, D-025, D-027 E ; D-029, D-031, D-032 lus sur la branche `ent-personas-socle-21`), `docs/socles/04_ADMINISTRATIF.md` §8.9 (payeurs), `docs/recette/RECETTE_SECURITE_3.md` (version d'`ENT`), `docs/SOCLES.md` (Q13, T-21) ; forme reprise de `PERSONAS_10.md` et `PERSONAS_21.md`.
**Code relu pour la passe [F]** (sur `origin/ENT` `ee7706d3`) : `ent-db/src/service.rs:18194-18205` (`foyer_ids_parent` filtre `est_payeur`), `ent-db/src/fac_repo.rs:1028` (`executer_sepa_atomique`), `ent-api/src/routes.rs:11019-11038` (export historique : `cdtr_iban` en paramètre d'URL, toujours), `ent-auth/src/actions.rs:996-1000` (droits financiers par défaut), `ent-db/src/finances.rs:1567-1610` (avis financiers aux comptes des payeurs du foyer).
**Ajouts :** faits **directement** dans `24_FINANCES.md` et `25_PAIEMENTS.md`, marqués **« ajout personas »**. **Rien n'est supprimé** ; quand un ajout contredit le texte d'origine, la contradiction est écrite au §29 du 24 et au §27 du 25.
**Numérotation :** manques **M-669 à M-778** (après M-668 de `PERSONAS_21.md`) ; actions **FAC-215 à FAC-261** et **PAY-212 à PAY-239** ; règles **R-F29 à R-F41** et **R-P25 à R-P33** ; cas limites **CF-73 à CF-100** ; questions **Q-F26 à Q-F33** et **Q-P15 à Q-P20** ; réglages **S-179 à S-186** ; défaut de code nouveau **FD-27**.

---

## 0. À lire en premier : trois bloquants et leur recommandation (appliquée par défaut, D-031)

Les deux conceptions sont solides sur l'essentiel : un seul service, des pièces figées, un payeur par compte, un adressage strict, l'élève hors du circuit de l'argent. En les faisant vivre par seize personas, **trois points bloquants** apparaissent. Les deux extrêmes (A et C) changent le **schéma** des lots F2 et F4 ; le deuxième (B) change **l'ordre** des lots pour le pilote.

| # | Point | Pourquoi c'est bloquant | Recommandation (appliquée par défaut, D-031) | Lots |
|---|---|---|---|---|
| **A** | **Le débiteur du contrat n'est pas la répartition de la facture.** La conception fait de la désignation du socle 4 (« la mère 60 %, le père 40 % ») la seule vérité : « chaque payeur ne doit que sa part », « jamais sur le compte du père sans accord écrit » (R-F5, CF-58). Or, en droit, l'établissement a pour débiteurs **les signataires du contrat de scolarisation** — souvent « solidairement » — et, entre **époux** ou **partenaires de PACS**, les dettes d'éducation des enfants sont **solidaires** par la loi (C. civ. 220 et 515-4, §5 point 1). Un jugement du juge aux affaires familiales répartit les frais **entre les parents**, il ne lie pas l'établissement (P06, M-704 à M-707) | Sans distinguer les deux, (1) l'établissement perd un recours que le contrat lui donne (la part du père impayée devient irrécouvrable alors que la mère avait signé solidairement) ; (2) à l'inverse, un comptable « corrige » à la main en facturant la totalité à la mère, et la conception ne sait ni le refuser ni le tracer ; (3) la reprise de Charlemagne (un débiteur par famille) ne dit pas qui a signé. C'est un défaut de **modèle** : il se fige dans `ent_fin_engagement` (F4) et `ent_fin_compte` (F2) | **Deux notions séparées** (R-F29) : l'**engagement** porte les **signataires** du contrat et la **clause de solidarité** (oui / non / à vérifier), avec le document signé ; la **répartition** (socle 4) dit qui reçoit quelle facture. La facturation suit **toujours** la répartition ; le **recours** contre un co-signataire solidaire pour la part impayée d'un autre est une **proposition** soumise à décision de la direction (★), après mise en demeure du payeur défaillant, avec un avis séparé qui ne révèle **que** le montant de la part impayée (PAY-212, CF-74). Aucune solidarité n'est présumée : sans contrat lu, l'état est « à vérifier ». La **rédaction** de la clause est soumise à un juriste (Q-F26) | **F2**, **F4**, P6 ; Q-F26 |
| **B** | **L'an 1 du pilote n'a pas de chemin outillé.** La conception recommande (Q-F2) que les finances restent dans **Charlemagne** l'an 1, mais ne dit que « double tenue » et « reprise des soldes » (FAC-205, FAC-206). Or, dès l'an 1, ScolIA tient **les liens familiaux, les payeurs, les parts, les adresses, les séparations, les décès, l'élève majeur** (socle 4), **les arrivées et départs** (socle 3), **les régimes** (socle 2) et **les absences justifiées** (appel) (P03, M-690 à M-694) | (1) Charlemagne facture avec **ses** données : après une séparation enregistrée dans ScolIA, il continue d'envoyer la facture totale et les relances **au mauvais parent**, ou à l'ancienne adresse commune — c'est la fuite que la conception veut éviter, déplacée dans l'autre logiciel ; (2) le secrétariat **ressaisit** chaque mouvement (régime, départ, remise d'ordre) dans deux logiciels ; (3) l'espace famille de ScolIA ne montre rien de financier, la famille garde deux portails | **Lot F14 « cohabitation an 1 », juste après F0, avant F1** : (a) **export mensuel des faits facturables** (arrivées, départs, changements de régime, absences longues justifiées sans motif, internat) dans un tableur lisible par la saisie Charlemagne, et dans son format d'import s'il existe [NV, à lire sur un exemple F-03] (FAC-220) ; (b) **export des payeurs, parts et adresses de facturation** à jour, avec la liste des **changements** depuis l'export précédent (FAC-221) ; (c) **dépôt en lot des factures PDF** produites par Charlemagne au coffre du **seul** payeur concerné, rapprochées par numéro d'élève puis nom et date de naissance, jamais distribuées au hasard (FAC-222) ; (d) recommandation à l'établissement de **couper l'envoi des factures par le portail de l'éditeur** une fois (c) en place (CF-77) ; (e) **solde à une date importé en lecture** pour le guichet (« votre solde au 30/11 selon la comptabilité ») (FAC-223). F14 sert aussi la **double tenue** de l'an 1 (FAC-206) | **F14** (nouveau) ; Q-F27 |
| **C** | **Un OGEC, plusieurs établissements, un seul créancier.** La conception pose « un compte par payeur **et par établissement** » (R-F5, R-F26, D-013) et « un mandat par compte et par créancier » (25 §7.2). Or un OGEC gère souvent **l'école, le collège et le lycée** d'un même ensemble [H, matrice T-21] : **même personne morale, même ICS, même compte bancaire, même comptable** (P10, M-726 à M-729) | Une famille de quatre enfants répartis sur trois établissements du même OGEC reçoit **trois factures, trois échéanciers, signe trois mandats** avec le même créancier, ne bénéficie d'**aucune** réduction de fratrie (rang calculé par établissement) et la comptable rapproche trois prélèvements par famille. Si la colonne manque en F2, la consolidation future impose de **réécrire** comptes, mandats et RUM de toutes les familles | **Entité de facturation** (R-F33) : chaque compte et chaque mandat portent l'**entité** (organisme gestionnaire = créancier) dès F2 et P2 ; par défaut, un compte par établissement (D-013 inchangé) ; la **consolidation** (un compte, une facture à sections, un mandat, une fratrie d'entité) n'est possible que si les établissements ont déclaré le **même** organisme gestionnaire **et** que l'ensemble est un **groupe consenti** (Q13 de `SOCLES.md`, T-21) **et** que le payeur l'accepte (FAC-224, FAC-225, PAY-226) ; elle reste **désactivée** tant que Q13 n'est pas décidée | **F2**, P2, F8 ; Q-F28 |

**Trois décisions importantes** (ne changent pas le schéma mais doivent être tranchées avant F7 et P6) : **surendettement** (la recevabilité d'un dossier gèle légalement les recouvrements et les paiements des créances antérieures : un prélèvement automatique qui continue est une faute, R-P25, Q-P15) ; **le secrétariat ne navigue pas dans tous les comptes** et ne gère pas les bourses par défaut (FD-27, R-F37, Q-F30) ; **la radiation ne dépend jamais du solde** (exeat, certificat, dossier scolaire, R-F34, Q-F31).

---

## 1. Méthode

**Démarche.** Celle de `PERSONAS_21.md` §1 : seize personas réalistes, prises dans la consigne et dans le tableau « acteurs » du §3 des deux conceptions, couvrant le privé sous contrat (le lycée pilote et son OGEC) et le public (EPLE), les familles ordinaires et les cas particuliers. Pour chacune : son **année financière** (de la réinscription à la clôture), puis ses **incidents** ; ses **irritants**, **peurs** et ce qui la **ravirait** ; les **manques** et **contradictions** trouvés dans la conception ; les **cas limites** ; les **exigences nouvelles**.

**Grille transversale** posée à chaque persona : qui doit (contrat), qui reçoit (répartition), qui paie (moyen), qui voit (droit) ; ce qui se révèle par un libellé, une ligne, une absence de ligne, un avis ; ce qui change à une date (séparation, majorité, départ, décès, bourse) ; ce que la famille peut faire **elle-même** ; ce que l'IA ajoute **sans décider** ; ce que fait Charlemagne (parité, D-024).

**Couverture du marché.** La ligne §3.11 de la matrice (FIN-01 à FIN-18) et l'expérience de Florian sur Charlemagne [H] ; aucune page d'éditeur n'a été relue pour cette passe (les pages financières de Charlemagne ne détaillent pas le module [H, constat de la passe 21]).

**Vérification réglementaire.** Un sous-agent de recherche a lu les textes (Légifrance, service-public.fr, education.gouv.fr, CNIL, Banque de France) ; résultats au §5 de ce document et versés au §5 du 24 (§5.10) et au §4 du 25 (§4.5), avec leur statut [V] / [V partiel] / [NV].

**Gravité** (identique aux passes précédentes) : **bloquant** (sans lui, contournement dangereux, cas réel impossible, ou conception contradictoire au point d'empêcher de coder) ; **important** (cas fréquent sans solution propre, risque d'erreur ou d'illégalité, fonction de Charlemagne absente) ; **confort** (gain net, contournement possible) ; **différenciant** (rare sur le marché [H]).

**Conventions.** [V] lu sur une source officielle ; [V partiel] ; [NV] non lu ; [H] hypothèse ; [R] recommandation ; [F] fait vérifié dans le dépôt.

**Limites.** Les personas sont construites, pas interviewées. Les pratiques de terrain (solidarité dans les contrats, prélèvement sur compte joint après séparation, retenue de l'exeat pour impayé, fréquence du surendettement, OGEC multi-établissements) sont des hypothèses fondées sur l'usage courant et sur l'expérience de Florian [H].

---

## 2. Les personas en un tableau

| # | Persona | Établissement | Ce qu'elle éprouve surtout |
|---|---|---|---|
| P01 | **Mme Duval**, cheffe d'établissement (et **M. Ferré**, trésorier de l'OGEC) | Lycée privé sous contrat (pilote), 1 100 élèves, 180 demi-pensionnaires au réel, 60 internes | Tarifs, arbitrages, impayés délicats, délégations, rapport au conseil d'administration, fraude |
| P02 | **Mme Lambert**, comptable de l'OGEC (seule à temps plein) | Lycée pilote | Rentrée, lots SEPA, rejets, chèques, rapprochement, remboursements, clôture, être seule |
| P03 | **Florian**, secrétariat (utilise Charlemagne) | Lycée pilote | Cohabitation de l'an 1, guichet, téléphone, changements de régime, attestations |
| P04 | **Cabinet Roche**, expert-comptable (et commissaire aux comptes) de l'OGEC | — | Exports, balance auxiliaire, pièces, produits constatés d'avance, comptes de tiers |
| P05 | **M. Benoît**, père payeur, perd son emploi en novembre | Lycée pilote, deux enfants | Rejets, honte, report, plan, surendettement |
| P06 | **Mme Garnier** et **M. Garnier**, séparés, résidence alternée ; scolarité 60/40, cantine selon la semaine ; elle s'est remariée | Lycée pilote, une fille en 1ʳᵉ | Parts, quotients, mandat sur compte joint, jugement, attestations pour le juge |
| P07 | Trois **payeurs tiers** : **Mme Morel**, grand-mère ; **la société Arvel**, employeur d'un parent ; **le département** (aide sociale à l'enfance) | Lycée pilote | Facture à son nom, prise en charge plafonnée, rien de scolaire, Chorus Pro |
| P08 | **Famille Haddad**, boursière (échelon 5), quatre enfants | Lycée pilote et collège public voisin | Bourse déduite au bon endroit, ne pas être repérée, excédent reversé |
| P09 | **Lucas**, 19 ans, BTS dans le lycée, se paie lui-même ; ses parents payaient jusqu'en juin | Lycée pilote (section de BTS) | Devenir payeur, ses parents ne voient plus rien, son propre mandat |
| P10 | **Famille Lemoine**, cinq enfants : deux au lycée, deux au collège, un à l'école du même OGEC | Ensemble scolaire (hypothèse T-21) | Trois factures, trois mandats, fratrie, simulateur |
| P11 | **Famille Petit**, part en février (mutation), 840 € d'impayé | Lycée pilote | Exeat, solde de sortie, trop-perçu ou impayé, relance à la nouvelle adresse |
| P12 | **Inès**, interne de semaine ; parents expatriés (Maroc) ; correspondant à Lyon | Lycée pilote, internat | Trimestres, nuitées, caution, virement international, langue |
| P13 | **M. Caron**, professeur organisateur d'un voyage à Rome (48 élèves, 620 € par élève) | Lycée pilote | Acomptes, désistements, aides discrètes, état agrégé, départ |
| P14 | **Mme Nguyen**, adjointe gestionnaire, et **M. Faure**, agent comptable d'un groupement | Lycée public (EPLE), 900 élèves | Régie, droits constatés, Op@le, remises de principe, titres exécutoires |
| P15 | **M. Blanc**, délégué à la protection des données de l'OGEC | — | Registre, durées, droit d'accès, analyse d'impact, journaux, minimisation |
| P16 | Deux **attaquants internes** : **M. Garnier** (P06) qui cherche à voir et changer les finances de son ex-épouse ; **Mme Sorel**, secrétaire curieuse (et parente d'élève) | Lycée pilote | Toutes les portes : routes, guichet, téléphone, mandat, remboursement, IA, journaux |

---

## 3. Parcours et manques, persona par persona

Chaque tableau donne : le manque, pourquoi il compte, sa gravité, le socle concerné, et l'ajout fait (identifiant dans `24_FINANCES.md` ou `25_PAIEMENTS.md`) ou le socle où il est renvoyé.

### P01 — Mme Duval, cheffe d'établissement (et M. Ferré, trésorier de l'OGEC)

**Année.** *Février* : simulation des tarifs N+1 avec le trésorier (hausse de 2,5 %, nouvelle tranche de quotient). *Mai* : vote au conseil d'administration de l'OGEC. *Septembre* : arbitrage de 14 demandes de réduction. *Décembre* : trois familles à plus de 2 000 € d'impayé, dont une en séparation conflictuelle. *Juin* : rapport financier au conseil d'administration, créances irrécouvrables, refus de réinscription demandé par le trésorier pour une famille.
**Irritants.** Les chiffres changent selon qui les sort ; signer des courriers faits à la main ; arbitrer sans historique. **Peurs.** Exclure un enfant pour une dette de parents ; une fraude au changement d'IBAN ; un contrôle qui trouve une contribution illégale ; être absente le jour où une décision ★ est nécessaire. **Ce qui la ravirait.** Un tableau unique (impayés par situation, trésorerie prévue, coût des réductions et des aides, écart N-1), des décisions **préparées** (proposition, historique, conséquence chiffrée) et des **délégations** claires.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-669 | **Délégation des décisions ★** (avoirs au-delà du seuil, mise en demeure, perte, seconde validation) à un adjoint nommé, datée, bornée, journalisée | La conception met « DIR » partout ; une cheffe absente trois semaines bloque lots, remboursements et mises en demeure | important | 24, 25, 1 | FAC-248, R-F23 (précisé) |
| M-670 | **Tableau de pilotage** sans nominatif par défaut : impayés par ancienneté et situation, trésorerie prévue, coût des réductions et des aides, écart N-1, rejets du mois | FAC-197 et PAY-200 / PAY-209 existent séparément ; la direction veut **une** page | confort | 24, 25 | FAC-247 |
| M-671 | **Rapport annuel au conseil d'administration** (produit par nature, réductions, aides, impayés, créances passées en perte, comparaison N-1), agrégé, prêt à joindre | Fait à la main dans un tableur aujourd'hui [H] ; c'est la pièce que le trésorier présente | confort | 24, 48 | FAC-249 |
| M-672 | **Demande de refus de réinscription** pour impayé : la conception renvoie à un avis juridique (Q-P9) mais n'outille pas la décision (dossier, historique des propositions de plan, motif, information de la famille, **séparation d'avec la scolarité de l'enfant**) | La demande viendra du trésorier dès la première année ; sans cadre, la décision se prend sur un tableau d'impayés | important | 25, 3 | CF-88, Q-P9 (précisé) |
| M-673 | **Recours contre un co-signataire solidaire** (bloquant A) : rien ne permet de constater que la mère a signé solidairement quand le père ne paie pas sa part | §0 point A | **bloquant** | 24, 25 | R-F29, FAC-215, PAY-212 |
| M-674 | **Alerte direction** sur tout geste « de contrôle interne » (changement de créancier [existant], réouverture de période, ouverture d'une liste nominative de voyage, ouverture d'un compte protégé) dans une seule file | Les alertes existent une à une ; la direction veut une file « gestes sensibles » | confort | 24, 25 | FAC-244, R-F36 |

### P02 — Mme Lambert, comptable de l'OGEC

**Année.** *Juillet* : 60 nouveaux mandats, dont 12 papier. *Fin août* : simulation de la rentrée, 412 factures. *Septembre* : 300 chèques, 40 changements de régime, 6 séparations. *Octobre* : premier lot SEPA, 9 rejets. *Novembre* : remises d'ordre d'une fermeture de deux jours. *Décembre* : remboursements de trop-perçus. *Février* : un départ avec impayé, un décès. *Juillet* : clôture, export pour le cabinet.
**Irritants.** Être seule à savoir faire ; trois écrans pour un chèque ; les rejets recopiés depuis les avis bancaires. **Peurs.** Prélever deux fois ; prélever le mauvais parent ; perdre un chèque entre l'accueil et la banque ; une erreur sur 400 factures ; qu'on la soupçonne d'une fraude. **Ce qui la ravirait.** Une file de travail ; une rentrée en une demi-journée ; un second regard qui la protège plutôt qu'il ne la contrôle.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-675 | **Continuité de service** : la séparation des tâches (seconde validation, créancier, lots) suppose deux personnes ; dans un OGEC à **une** comptable, qui valide en son absence ? | Sans réponse, la règle est contournée (mot de passe prêté) ou bloque le lot du mois | important | 25, 1 | FAC-248 (délégation), S-167, R-P22 (précisé : la direction est le second regard par défaut) |
| M-676 | **Journal « ce que j'ai fait »** et preuve de ses gestes, consultable par elle-même | Sa peur principale est d'être soupçonnée ; le journal la protège autant qu'il la contrôle | confort | 24 | FAC-203 (précisé : chacun voit ses propres gestes) |
| M-677 | **Interdiction de traiter son propre compte** ou celui d'un proche (elle est aussi mère d'un élève de 2ᵈᵉ) : encaisser, imputer, rembourser, accorder une réduction « personnel » | Classique du contrôle interne ; absent des deux conceptions | important | 24, 25 | R-P29, R-F35, PAY-239 |
| M-678 | **Rejet avant présentation** : un parent prévient la veille « je ne pourrai pas » ; elle veut retirer la ligne du lot **sans** rejet bancaire ni frais | PAY-132 permet de retirer une ligne avant validation, mais rien ne relie la demande de la famille au lot | important | 25 | PAY-217, R-P26 |
| M-679 | **Balance âgée** des comptes débiteurs et **justification du 411** à la clôture | Demandée par le cabinet (P04) ; elle la fait aujourd'hui dans un tableur [H] | important | 24, 48 | FAC-237, FAC-238 |
| M-680 | **Encaissements pour compte de tiers** (APEL, assurance scolaire) : état et reversement | Le 24 cite la nature `pour_compte_tiers` ; aucun geste de reversement | important | 24, 25 | FAC-240 |
| M-681 | **Mandats papier de juillet** : saisie en série avec scan, comme les chèques | PAY-114 est unitaire ; 12 mandats un par un | confort | 25 | PAY-114 (précisé) |

### P03 — Florian, secrétariat (utilise Charlemagne)

**Année (an 1 du pilote).** Les finances sont dans **Charlemagne** ; les familles, les liens, les inscriptions, l'appel sont dans **ScolIA**. *Septembre* : 40 changements de régime, les uns saisis dans ScolIA, les autres oubliés dans Charlemagne. *Octobre* : une séparation enregistrée dans ScolIA, la facture de Charlemagne part encore à l'ancienne adresse commune. *Toute l'année* : « combien je dois ? » au téléphone ; attestations pour la CAF et les employeurs ; enveloppes déposées à l'accueil.
**Irritants.** Ressaisir chaque mouvement deux fois ; ne pas savoir répondre au téléphone sans ouvrir Charlemagne ; les attestations à la main. **Peurs.** Donner à un parent le solde de l'autre ; envoyer la facture au parent qui ne doit plus rien ; dire un montant faux. **Ce qui le ravirait.** Que ScolIA **dise à Charlemagne** ce qui a changé, et distribue les factures au bon payeur ; une vue guichet qui répond « votre solde » et rien d'autre.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-690 | **Export mensuel des faits facturables** vers Charlemagne (arrivées, départs, régimes, absences longues justifiées, internat) | §0 point B ; sans lui, double saisie et oublis | **bloquant** | 24 | FAC-220, F14 |
| M-691 | **Export des payeurs, parts, adresses** et de leurs changements | §0 point B ; c'est ScolIA qui sait qui paie | **bloquant** | 24, 4 | FAC-221, F14 |
| M-692 | **Distribution des factures Charlemagne** au seul payeur (coffre, lien nominatif, courrier) | Le portail de l'éditeur ignore les séparations enregistrées dans ScolIA | important | 24, 7 | FAC-222, CF-77 |
| M-693 | **Solde importé en lecture** pour le guichet pendant l'an 1 | Répondre au téléphone sans ouvrir un second logiciel | confort | 24 | FAC-223 |
| M-694 | **Contradiction** : Q-F2 recommande Charlemagne l'an 1, mais le « parcours minimal pour un pilote réel » (F0, F1–F5, F7, P1–P4, P6) ne dit rien de ce que fait ScolIA **pendant** l'an 1 | Les lots F1–F5 ne servent qu'à l'an 2 ; F14 est le seul lot utile l'an 1 | important | 24 | §25.1 (ordre), C-F7 |
| M-695 | **Vue guichet** pour le secrétariat : chercher le payeur **présent**, lui donner son solde, ses échéances, son document ; pas de navigation dans les autres comptes, pas de tableau d'impayés | Le 24 donne `facturation.lire` (tous les comptes) au secrétariat ; sur `ENT`, l'Administratif a même `facturation.gerer` et `bourses.gerer` [F, FD-27] | important | 24, 1 | R-F37, FAC-250, FD-27 |
| M-696 | **Changement de régime saisi au secrétariat** : aperçu de l'effet financier, **tâche** à la comptabilité, aucune émission | Florian n'émet pas ; la comptable veut voir la demande, pas la découvrir | confort | 24 | FAC-251 |
| M-697 | **Vérification d'identité au téléphone** : un appelant qui se dit « le père » | Les fuites probables passent par une personne, pas par une route (constat de la passe 21) | important | 24, 25 | CF-95, R-F37 (précisé) |

### P04 — Cabinet Roche, expert-comptable et commissaire aux comptes de l'OGEC

**Année.** *Mensuel* : import des écritures dans le logiciel du cabinet. *Clôture (31 août)* : justification des comptes 411, 419 (avances des familles), 467 (comptes de tiers), 487 (produits constatés d'avance) ; balance âgée ; provision pour créances douteuses ; contrôle de la continuité des numéros. *Audit* : échantillon de pièces, rapprochement bancaire, contrôle des avoirs.
**Irritants.** Extractions incomplètes ; pièces sans origine ; avoirs non reliés. **Peurs.** Une écriture sans pièce ; une pièce modifiée ; un solde client qui ne se justifie pas. **Ce qui le ravirait.** Une balance auxiliaire égale au 411 au centime, les pièces d'une période en un dossier, la provision proposée par ancienneté.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-698 | **Balance auxiliaire** (par compte débiteur) **égale** au compte collectif à chaque date, contrôlée | Base de toute révision ; absente | important | 24, 48 | FAC-237, FAC-238 |
| M-699 | **Dossier de pièces d'une période** (PDF/A et index CSV : numéro, date, compte, montant, empreinte) | Échantillonnage d'audit sans demander 30 PDF un par un | confort | 24, 7 | FAC-239 |
| M-700 | **Avances et crédits des familles** (419) distingués des créances (411) dans les écritures | Un crédit (trop-perçu) n'est pas une créance négative ; mal classé il fausse le bilan [H] | important | 24, 25, 48 | PAY-210 (précisé), FAC-238 |
| M-701 | **Provision pour créances douteuses** proposée par ancienneté et situation (plan, procédure, surendettement) | Le cabinet la calcule à la main ; la situation des dossiers est dans ScolIA | confort | 25, 48 | PAY-200 (précisé) |
| M-702 | **FEC** : la conception l'annonce « si exigé » ; l'obligation dépend du régime fiscal de l'OGEC | Voir §5 point 14 ; à qualifier par le cabinet (soumis) | confort | 48 | Q-F17 (précisé) |
| M-703 | **Contradiction** : produits constatés d'avance « [H] si l'expert-comptable le demande » (FAC-198), alors que la facture annuelle émise en août pour septembre–juin rend le calcul nécessaire à **toute** clôture au 31 août | Sans lui, le résultat de l'exercice est faux | important | 24 | FAC-198 (précisé : toujours produit) |

### P05 — M. Benoît, père payeur, perd son emploi en novembre

**Année.** *Septembre* : prélèvement de 10 mensualités pour deux enfants (DP), 412 € par mois. *Novembre* : licenciement ; il sait que le prélèvement du 5 décembre passera mal. *Décembre* : rejet ; sa banque lui facture 20 € [H]. *Janvier* : second rejet, retour au paiement manuel ; relance J+7. *Mars* : il dépose un dossier de surendettement, déclaré recevable en avril. *Juin* : plan de la commission.
**Irritants.** Ne pas pouvoir dire « pas ce mois-ci » avant le rejet ; relances automatiques ; devoir expliquer sa situation à plusieurs personnes. **Peurs.** Que ses enfants soient privés de cantine ou de voyage ; que les professeurs sachent ; les frais. **Ce qui le ravirait.** Prévenir en ligne, choisir une autre date, proposer un plan lui-même, être orienté discrètement vers l'aide.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-704 | **Prévenir d'une difficulté avant la présentation** : la ligne est retirée du lot (s'il n'est pas validé), sans rejet, sans frais, et un dialogue s'ouvre | Un rejet coûte à la famille (frais bancaires) et à l'établissement ; la conception ne connaît que le rejet | important | 25 | PAY-217, R-P26 |
| M-705 | **Report d'une échéance** à la demande (une fois par an sans justification, par défaut) | Geste simple qui évite la spirale rejet → manuel → relance | confort | 24, 25 | PAY-216, S-181 |
| M-706 | **Choix de la date de prélèvement** (5, 10 ou 15 du mois) | Aligner sur la date de paie ou des allocations | confort | 25 | PAY-218, S-180 |
| M-707 | **Surendettement** : la recevabilité **suspend les procédures** et **interdit au débiteur de payer** les créances antérieures (C. consom. L. 722-2, L. 722-5, §5 point 3) ; la conception ne connaît ni la situation ni ses effets : relances et prélèvements continueraient | Un prélèvement automatique d'une échéance antérieure est un paiement interdit ; une relance est une pression illégale ; l'établissement doit aussi déclarer sa créance [V partiel] | important | 25 | PAY-213 à PAY-215, R-P25, CF-78, CF-79, Q-P15 |
| M-708 | **Demande d'aide discrète** depuis le compte (« je rencontre une difficulté ») qui part au socle 26 (fonds social) et à l'assistante sociale, **sans** que la comptabilité lise le motif | PAY-194 est un geste du personnel ; la famille doit pouvoir le faire elle-même | important | 24, 26 | FAC-261 |
| M-709 | **Frais de rejet** : même à 0 côté établissement, la famille paie sa banque ; l'avis de rejet doit le dire sans culpabiliser et proposer la prévenance (M-704) | Ton des avis | confort | 25 | PAY-151 (précisé) |
| M-710 | **Plan de la commission de surendettement** imposé : échéancier qui **remplace** le plan d'apurement, effacement partiel ou total → perte | Le plan d'apurement du 25 est un accord amiable ; le plan de la commission s'impose | important | 25 | PAY-215, CF-79 |

### P06 — M. et Mme Garnier, séparés, résidence alternée, parts différentes

**Situation.** Divorcés en 2025 ; jugement : résidence alternée une semaine sur deux ; « le père prend en charge 40 % des frais de scolarité, la mère 60 % ; chaque parent assume les frais de cantine de sa semaine ». Contrat de scolarisation signé **par les deux, solidairement**, en 2023 (avant le divorce). Mme Garnier s'est remariée ; son mari, M. Roy, propose de payer sa part. Le mandat SEPA de 2023 est sur l'ancien **compte joint**, que M. Garnier n'a pas clôturé.
**Année.** *Septembre* : chacun reçoit sa facture (60/40) sur son propre quotient. *Octobre* : la cantine est facturée 50/50, pas selon les semaines. *Novembre* : M. Garnier ne paie pas ; Mme Garnier veut savoir s'il a payé « pour le juge ». *Janvier* : M. Garnier révoque à sa banque le mandat du compte joint ; le prélèvement de Mme Garnier est rejeté. *Mars* : M. Garnier demande que « la mère paie tout désormais ».
**Irritants.** La cantine qui ne suit pas la résidence ; devoir s'arranger entre eux. **Peurs.** Payer la part de l'autre ; que l'autre voie son adresse, son IBAN, ses retards ; être poursuivie pour la dette de l'autre. **Ce qui les ravirait.** Que le jugement soit appliqué tel quel ; chacun ses papiers ; une attestation de **sa** part payée pour le juge.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-711 | **Signataires du contrat et solidarité** (bloquant A) | §0 point A | **bloquant** | 24 | R-F29, FAC-215, CF-73, CF-74 |
| M-712 | **Pièce de répartition** (jugement, convention homologuée, accord écrit des deux) jointe à la désignation, en accès restreint | Le secrétariat applique un jugement qu'il n'a pas ; un parent conteste « je n'ai jamais accepté 40 % » | important | 4, 24, 7 | FAC-217 |
| M-713 | **Un parent ne peut pas, seul, désigner l'autre** comme payeur ni augmenter la part de l'autre : acceptation du payeur désigné (signature en ligne) ou pièce (jugement) | Sans cela, « la mère paie tout désormais » devient une décision d'un seul ; c'est aussi la porte d'attaque de P16 | important | 4, 24 | R-F30, FAC-218, CF-75 |
| M-714 | **Cantine selon le calendrier de résidence alternée** (repas des semaines paires au père) | Le jugement le dit ; la conception ne sait que des pourcentages par catégorie | confort | 24, 4, 40 | FAC-219, S-179, CF-76 |
| M-715 | **Mandat sur compte joint d'ex-conjoints** : à la séparation, alerte « le mandat de Mme Garnier est sur un compte dont le titulaire enregistré est M. et Mme Garnier » et invitation à re-signer | Le cotitulaire peut révoquer à sa banque ; le titulaire du compte n'est plus le payeur ; CF-43 ne traite que le compte joint « voulu » | important | 25 | PAY-221, CF-80 |
| M-716 | **Paiement par un tiers non tenu** : M. Roy paie la part de son épouse ; Mme Garnier veut payer la part du père « pour ne pas pénaliser sa fille » | Le paiement par un tiers est valable (C. civ. 1342-1 [§5 point 16]) ; CF-58 l'interdit sur le compte du père sans accord écrit des deux | important | 25 | PAY-219, R-P27, CF-81, C-P5 |
| M-717 | **Attestation de ma part payée** (pour le juge), sans rien dire de l'autre parent | PAY-180 existe mais ne précise pas « ma part » ; le parent demandera aussi « a-t-il payé ? » | important | 25 | PAY-220 |
| M-718 | **Réduction d'impôt en résidence alternée** (moitié chacun) : certificat de scolarité **à chaque parent** | Rappel du 24 §5.6 ; le certificat vient du socle 7 | confort | 7 | renvoi socle 7 |
| M-719 | **Avis de recours** au co-signataire (si la direction le décide) qui ne révèle **que** le montant de la part impayée et sa période, jamais l'adresse, le quotient, l'historique de l'autre | Sinon le recours devient une fuite | important | 25 | PAY-212, R-F29 |

### P07 — Trois payeurs tiers : grand-mère, employeur, département (ASE)

**Mme Morel**, grand-mère, paie la scolarité de sa petite-fille par prélèvement sur son compte ; elle n'a aucun lien de responsabilité. **La société Arvel** paie directement la cantine des enfants de ses salariés expatriés (prise en charge : 100 % dans la limite de 900 € par an et par enfant), avec bon de commande et SIRET. **Le département** prend en charge l'internat d'un enfant confié à l'aide sociale à l'enfance ; la prise en charge est renouvelée par période de six mois ; l'adresse d'accueil est protégée.
**Irritants.** Recevoir des documents à un nom qui n'est pas le sien (grand-mère) ; factures sans référence de commande (employeur) ; dépôt hors Chorus Pro, prise en charge échue sans prévenir (département). **Peurs.** Recevoir des données scolaires ; payer au-delà du plafond ; payer pour un enfant qui n'est plus confié. **Ce qui les ravirait.** Une facture **à leur nom** qui ne contient que l'élève, la prestation, la période, la référence ; un rappel avant l'échéance de la prise en charge.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-720 | **Prise en charge** d'un organisme : référence, période, catégories, **plafond** (montant ou %), part non couverte au payeur suivant | §9.4 du 24 en parle ; aucune action ni table | important | 24 | FAC-255, CF-82 |
| M-721 | **Échéance de la prise en charge** (ASE : six mois) : alerte avant la fin, tâche « renouvellement », aucune facture au-delà sans renouvellement | Sinon facture refusée par l'organisme, ou reportée sur une famille qui n'a pas la garde | important | 24 | FAC-257, CF-83 |
| M-722 | **Délais de paiement d'un organisme public** (mandatement) : pas de relance avant le délai, relance à l'organisme **jamais** à la famille | Le parcours de relance du 25 est pensé pour les familles | important | 25 | PAY-222, R-P31 |
| M-723 | **Attestation de paiement au nom de la grand-mère** (elle paie, elle peut en avoir besoin) | PAY-180 vise le payeur ; à préciser pour un payeur sans lien | confort | 25 | PAY-180 (précisé) |
| M-724 | **Chorus Pro** : obligation à confirmer quand l'organisme public paie **hors marché** (§5 point 13) | Si l'obligation ne s'applique pas, F11 peut attendre | confort | 24 | FAC-170 (précisé), Q-F32 |
| M-725 | **Employeur payeur direct** ≠ attestation pour un employeur qui rembourse : les deux cas sont distingués au 24 §9.4 mais aucune action ne crée l'organisme employeur et sa référence | Cas réel des expatriés et des grandes entreprises [H] | confort | 24, 4 | FAC-256 |

### P08 — Famille Haddad, boursière, quatre enfants

**Situation.** Bourse de lycée échelon 5 pour Yasmine (1ʳᵉ, demi-pensionnaire), avec procuration à l'établissement ; son frère est au collège public voisin (bourse de collège, autre établissement) ; quotient tranche A ; deux enfants à l'école publique.
**Année.** *Septembre* : facture annuelle ; la famille veut voir « ce qui reste à payer » net de la bourse. *Décembre* : la bourse du 1ᵉʳ trimestre arrive en janvier ; l'échéancier a été calculé net (déduction anticipée). *Mars* : la bourse dépasse la demi-pension restante : excédent. *Avril* : un professeur demande à Yasmine devant la classe si « la bourse paie le voyage ».
**Irritants.** Ne pas comprendre le montant net ; attendre l'excédent. **Peurs.** Être repérée comme boursière ; payer en attendant une bourse qui arrive tard. **Ce qui la ravirait.** Une ligne claire « bourse déduite », le calendrier des versements, l'excédent remboursé sans le demander, rien de visible ailleurs.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-726 | **Affichage famille** de la bourse déduite par ligne, de l'échéancier net, du calendrier des versements de l'État | Comprendre son reste à payer ; l'explication par phrases fixes doit couvrir la bourse | confort | 24 | FAC-259 |
| M-727 | **Bourse invisible hors finances** : ni professeur, ni vie scolaire, ni organisateur de voyage ; **aucune** mention sur une attestation de paiement sauf demande | R-F19 parle de paiements, pas d'aides ; la passe 21 a posé la même règle pour les accompagnements | important | 24, 26 | R-F38, FAC-260 |
| M-728 | **Rapprochement du versement de l'État** (par élève, par trimestre) avec la déduction prévue ; écart → tâche | La déduction anticipée (S-159) suppose un contrôle à l'arrivée | important | 25, 26 | PAY-223 |
| M-729 | **Excédent de bourse reversé** au payeur bénéficiaire, sans demande | PAY-202/204 sont génériques ; la règle est à préciser pour la bourse (bénéficiaire, pas « le payeur ») | important | 25 | PAY-224 |
| M-730 | **Prime d'internat**, prime d'équipement, **retenue de bourse** pour absences injustifiées de plus de 15 jours, bourse versée au **majeur** (§5 point 6) | Le 24 ne traite que la prime d'équipement ; la retenue produit une régularisation | confort | 24, 26 | §10.5 (précisé), FAC-138 |
| M-731 | **Aucun professeur ne sait** qui est boursier (même le voyage) : l'organisateur voit un coût résiduel agrégé | Rappel de R-F19 ; à tester avec le voyage (P13) | important | 24, 42 | R-F38, §19.2 (piège 12) |

### P09 — Lucas, 19 ans, BTS, se paie lui-même

**Situation.** Majeur depuis un an ; ses parents ont payé la terminale ; en BTS il a un emploi étudiant et demande à payer **lui-même** ; ses parents restent destinataires de rien de financier pour le BTS, mais doivent toujours le solde de juin de terminale.
**Irritants.** Que ses parents reçoivent encore les avis ; devoir passer par eux pour un mandat. **Peurs.** Que ses parents voient ses retards ; que l'école lui réclame le solde de terminale de ses parents. **Ce qui le ravirait.** Devenir payeur en ligne, signer son mandat, ne plus rien partager.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-732 | **Transfert du payeur** parents → élève majeur à sa demande : acceptation de l'élève (R-F30), date d'effet, factures **futures** à son nom, **solde passé** aux parents | FD-15 rattache sa fiche payeur à son compte ; le geste de transfert manque | important | 24, 4 | FAC-253, CF-84 |
| M-733 | **Nouveau contrat** signé par l'élève majeur (signataire) pour le BTS | Bloquant A : il devient le débiteur contractuel | important | 24, 3 | FAC-215, R-F29 |
| M-734 | **Son mandat** ; celui des parents ne sert plus pour ses frais | §7.2 du 25 couvre le changement de payeur ; à préciser pour l'élève majeur (le compte élève signe avec sa passkey) | confort | 25 | PAY-225 |
| M-735 | **Rien de financier aux parents** pour ses frais, même s'ils restent titulaires d'un compte parent ; le relevé informatif R-F8 ne s'applique pas à un élève majeur | R-F8 vise « le parent non payeur » ; pour un majeur, ses parents n'ont plus d'autorité (C. civ. 414) | important | 24 | FAC-254, R-F8 (précisé) |
| M-736 | **Payeur ou élève majeur sous tutelle ou curatelle** : le tuteur signe ; en curatelle simple la personne signe seule ; en curatelle renforcée le curateur (§5 point 15) | Rare mais réel | confort | 24, 25, 4 | CF-85 |

### P10 — Famille Lemoine, cinq enfants dans trois établissements du même OGEC

**Situation (hypothèse T-21).** Deux enfants au lycée, deux au collège, une à l'école, un seul OGEC, un seul ICS, une seule comptable ; la réduction de fratrie « à partir du 3ᵉ enfant » est votée **par l'OGEC**, pas par établissement.
**Irritants.** Trois factures, trois mandats, trois prélèvements le même jour ; aucune réduction de fratrie (deux enfants par établissement). **Peurs.** Se tromper de facture ; payer deux fois. **Ce qui les ravirait.** Une facture, un mandat, une explication « 3ᵉ, 4ᵉ et 5ᵉ enfants : −20 % » ; un simulateur avant la réinscription.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-737 | **Entité de facturation** (bloquant C) | §0 point C | **bloquant** | 24, 25 | R-F33, FAC-224, PAY-226 |
| M-738 | **Facture consolidée** à sections par établissement (si groupe consenti) | Une famille, un créancier, une pièce | important | 24 | FAC-225, S-182 |
| M-739 | **Fratrie d'entité** : rang calculé sur les enfants du même payeur dans l'entité (groupe consenti) | La réduction est votée par l'OGEC ; la calculer par établissement la vide de sens | important | 24 | FAC-132 (précisé), CF-86 |
| M-740 | **Vue « tous mes enfants »** : totaux par enfant, par catégorie, dans un compte | L'espace famille est pensé « par enfant » et « pour moi » ; la famille nombreuse veut un tableau | confort | 24 | FAC-229 |
| M-741 | **Simulateur famille personnel** (avant réinscription) avec ses enfants, son quotient, sans rien enregistrer | Le simulateur public est anonyme ; la famille veut « combien l'an prochain pour nous » | différenciant | 24 | FAC-228, R-F39 |

### P11 — Famille Petit, part en février avec un impayé

**Situation.** Mutation du père ; départ le 14 février ; 840 € d'impayé (deux échéances rejetées) ; un trop-perçu de cantine de 36 € ; le nouvel établissement demande l'exeat et le dossier scolaire.
**Irritants.** Devoir « solder avant de partir » ; ne pas savoir ce qu'il reste dû. **Peurs.** Que l'exeat soit retenu ; que le nouvel établissement apprenne la dette. **Ce qui la ravirait.** Un solde de sortie clair, l'avoir des mois non dus, le trop-perçu compensé ou remboursé avec son accord, l'exeat le jour même.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-742 | **La radiation ne dépend jamais du solde** : certificat de radiation, dossier et livret scolaires délivrés quel que soit le solde ; l'« exeat » au sens de **quitus** (« sommes dues réglées », réponse ministérielle, §5 point 12) est l'**attestation de situation** (25 PAY-182), qui dit la vérité et ne bloque rien | Pratique de retenue répandue [H] ; aucun texte n'impose l'exeat ni n'en fait une condition d'inscription [V] | important | 24, 3, 7 | R-F34, CF-87 |
| M-743 | **Aperçu du solde de sortie** au moment de la radiation (avoirs des mois non dus, trop-perçus, échéances restantes), **sans bloquer** la radiation | Le secrétariat radie dans le socle 3 ; la comptabilité découvre le départ un mois plus tard [H] | important | 24, 3 | FAC-226, FAC-227 |
| M-744 | **Arrêt des prélèvements futurs** à la date de départ, échéancier de solde, remboursement ou compensation **avec accord** | §7 du 25 ne traite pas le départ | important | 25 | PAY-227 |
| M-745 | **Relance après départ** à la nouvelle adresse du payeur ; aucune information au nouvel établissement | D-013 ; tentation d'appeler le nouvel établissement [H] | important | 25 | PAY-228, R-P32 |
| M-746 | **Compensation** trop-perçu / impayé du **même** payeur : licite si les deux sont certains, liquides, exigibles [H] ; la conception exige l'accord (Q-F14) | À préciser : compensation proposée, la famille informée | confort | 25 | CF-88 (précisé dans PAY-227) |

### P12 — Inès, interne de semaine, parents expatriés

**Situation.** Internat du lundi au vendredi ; parents au Maroc (hors zone SEPA pour leur banque marocaine) ; correspondant à Lyon (sans rôle financier) ; caution de chambre 150 € ; facturation au trimestre ; deux nuitées de week-end exceptionnelles (examen).
**Irritants.** Virements internationaux coûteux, sans référence ; documents en français seulement. **Peurs.** Une caution non rendue ; des nuitées facturées sans prévenir. **Ce qui les ravirait.** Payer en ligne par carte ; une référence de virement simple ; un relevé en anglais ou en arabe de l'explication ; la caution rendue à la sortie.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-747 | **Internat** : forfait trimestriel, nuitées supplémentaires annoncées avant facturation, remise d'ordre de fermeture, **caution** encaissée comme dépôt et restituée | §11 du 24 énumère ; aucune action dédiée | confort | 24, 41 | FAC-231 |
| M-748 | **Paiement international hors SEPA** (virement SWIFT avec référence, frais à la charge du donneur d'ordre annoncés), carte en ligne | CF-15 cite « virement SEPA ou international » ; rien dans le rapprochement | confort | 25 | PAY-230, CF-89 |
| M-749 | **Langue** : explication et avis dans la langue de la famille (socle 8), document officiel en français | 24 §14 le dit ; à préciser pour les documents financiers | confort | 24, 8 | S-186 |
| M-750 | **Caution** : restitution à la sortie vers l'IBAN vérifié, retenue motivée (dégradation constatée) | PAY-229 | confort | 25 | PAY-229 |

### P13 — M. Caron, professeur organisateur d'un voyage à Rome

**Situation.** 48 élèves, 620 € par élève, acompte de 150 € à l'inscription, deux soldes ; aide de l'APEL 40 € par élève ; fonds social pour 5 élèves ; deux désistements (un pour maladie, un sans motif) ; départ le 14 avril.
**Irritants.** Collecter des chèques en classe ; tenir un tableau ; ne pas savoir qui a payé. **Peurs.** Partir avec un déficit ; devoir dire à un élève qu'il ne part pas ; savoir qui est aidé. **Ce qui le ravirait.** Le coût par élève annoncé aux familles avant l'engagement, un état « 46 à jour, 2 en attente », les aides demandées sans lui, un bilan.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-751 | **Engagement de la famille au voyage** : coût, échéancier, conditions de désistement **acceptés** à l'inscription (document signé) | Sans engagement, pas de désistement opposable (clauses abusives, 24 §5.1) | important | 24, 42 | FAC-233 |
| M-752 | **Demande d'aide depuis la page du voyage**, discrète, vers le socle 26 ; l'organisateur ne voit que le coût résiduel agrégé | FAC-234 ; R-F19, R-F38 | important | 24, 26, 42 | FAC-234 |
| M-753 | **Aucun élève écarté pour raison financière** (circulaire du 13/06/2023 : « en aucun cas », §5 point 8) ; **sortie obligatoire gratuite** ; surcoût lié au **handicap** jamais facturé | La décision « ne part pas » est souvent prise par l'organisateur [H] ; la circulaire vise le public, bonne pratique pour le privé [R] | important | 24, 42 | R-F20 (précisé), R-F41, CF-90 |
| M-754 | **Échéancier du voyage par prélèvement** sur le mandat existant : pré-notification spécifique | §12 du 24 parle d'échéancier propre ; le 25 ne dit pas s'il peut être prélevé | confort | 25 | PAY-231 |
| M-755 | **Chèques-vacances** pour un voyage scolaire : seulement si l'établissement est **conventionné** avec l'ANCV (§5 point 9) | PAY-167 marqué [NV] | confort | 25 | PAY-167 (précisé) |
| M-756 | **Budget du voyage côté comptabilité** (écart prévu/réel, aides par source, excédent) | FAC-188 couvre la clôture ; pas le suivi | confort | 24, 42 | FAC-236 |

### P14 — Mme Nguyen (adjointe gestionnaire) et M. Faure (agent comptable), lycée public

**Situation.** EPLE de 900 élèves, rattaché à la région ; tarifs de restauration et d'hébergement votés par la collectivité ; facturation au forfait trimestriel (« droits constatés ») ; régie de recettes pour les encaissements au guichet (décret 2020-542, §5 point 5) ; logiciel comptable **Op@le** (qui remplace GFC, §5 point 5) ; bourses déduites ; **pas** de remise de principe nationale (abrogée en 2016, §5 point 4), mais une réduction de fratrie prévue par le règlement de la collectivité ; recouvrement contentieux par l'agent comptable (titre exécutoire, saisie administrative à tiers détenteur) ; admission en non-valeur par le conseil d'administration.
**Irritants.** Ressaisir dans Op@le ce qu'un autre outil a calculé ; les remises d'ordre à la main. **Peurs.** Une recette encaissée hors régie ; un ordonnateur qui recouvre à la place du comptable ; un logiciel pensé pour le privé. **Ce qui les ravirait.** ScolIA comme outil de l'ordonnateur et du régisseur, qui produit l'état de liquidation des droits constatés et les bordereaux pour Op@le.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-757 | **Aucune règle du privé codée en dur** (R. 442-48, OGEC, séries F/A, avoir, « contrat de scolarisation ») : un **régime d'établissement** choisit le jeu de règles | F13 vient « après le pilote » ; si F2–F5 figent le privé, F13 devient une réécriture | important | 24, 25 | R-F40, F13 (précisé), C-F8 |
| M-758 | **Séparation ordonnateur / comptable** : l'ordonnateur liquide (droits constatés), le comptable recouvre sur **autorisation de poursuivre** de l'ordonnateur ; la non-valeur et la remise gracieuse sont votées par le conseil d'administration ; dans ScolIA, la relance contentieuse et la perte (PAY-186, PAY-199) ne sont **pas** des gestes de l'ordonnateur | M9.6 [V, §5 point 5] | important | 25 | PAY-235, R-P33 |
| M-759 | **Régie** : journal, plafond d'encaisse, versements au comptable, nomination du régisseur avec l'agrément de l'agent comptable | 25 §4.2 cite le décret 2019-798 ; pour un EPLE la référence est le **décret 2020-542** et l'arrêté du 13/08/2020 (§5 point 5) ; aucune action | important | 25 | PAY-233, C-P7 |
| M-760 | **Réduction de fratrie du public** : la remise de principe nationale est **abrogée** depuis 2016 (§5 point 4) ; la réduction éventuelle vient du règlement de la **collectivité** ; ne pas coder l'ancienne règle | Une hypothèse fausse aurait été codée « en dur » | confort | 24 | FAC-230 (corrigé) |
| M-761 | **Export vers Op@le** (état de liquidation, titres, encaissements de régie) [NV sur les formats] | Sans format, ScolIA est une double saisie | important | 24, 9 | F13 (précisé), Q-F33 |

### P15 — M. Blanc, délégué à la protection des données de l'OGEC

**Année.** *Avant le pilote* : registre des traitements, analyse d'impact, information des familles. *En cours d'année* : deux demandes d'accès (une d'un père séparé qui veut « tout ce que vous avez sur moi et sur ce que paie la mère »), une demande d'effacement après départ, un contrôle des journaux.
**Irritants.** Durées « soumises au DPO » sans proposition chiffrée ; journaux introuvables. **Peurs.** Une liste d'impayés qui circule ; un IBAN dans un journal ; une réponse d'accès qui contient les données de l'autre parent. **Ce qui le ravirait.** Une fiche de traitement prête, des durées appliquées automatiquement avec rapport, un export d'accès limité à la personne, un journal des déchiffrements d'IBAN.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-762 | **Fiche de traitement** du domaine (finalités, bases légales, catégories, destinataires, durées) produite depuis la conception | Préalable au pilote ; fait à la main sinon | important | 24, 1 | FAC-241 |
| M-763 | **Analyse d'impact** (AIPD) : données financières et sociales (bourses, quotient, difficultés) de familles, à grande échelle, avec mineurs concernés [H sur l'obligation, liste CNIL NV] | À conduire avant toute donnée réelle (soumis au DPO) | important | 24, 25 | lot 0 (préalable F14), Q-F29 |
| M-764 | **Purge et archivage automatiques** selon les durées (lignes non émises, comptes sans pièce, IBAN de mandats terminés + 13 mois, impayés 48 h après régularisation), avec rapport | Les durées existent (24 §19.3, 25 §4.4) ; rien ne les applique | important | 24, 25 | FAC-242, PAY-237 |
| M-765 | **Droit d'accès limité à la personne** : jamais les données d'un autre payeur, même du même enfant ; mention de ce qui n'est pas communiqué et pourquoi | FAC-204 « pour le payeur seulement » : à préciser contre la demande du père | important | 24 | FAC-243 |
| M-766 | **Journal des déchiffrements d'IBAN** (qui, quand, pour quel fichier) | R-P21 interdit l'IBAN dans les journaux mais ne trace pas son usage | important | 25 | PAY-236, R-P30 |
| M-767 | **Information des familles** sur les traitements financiers (notice, au moment du contrat et du mandat) | Absente des conceptions | confort | 24, 7 | FAC-241 (précisé) |
| M-768 | **Effacement après départ** : refusé tant que les obligations comptables courent (10 ans), expliqué, minimisé (archive) | Demande prévisible ; la réponse doit être prête | confort | 24 | FAC-243 (précisé), CF-91 |

### P16 — Deux attaquants internes

**M. Garnier** (P06) veut savoir si son ex-épouse paie, à quelle adresse elle vit désormais, et reprendre la main : il essaie les routes financières par identifiant, appelle le secrétariat, demande une attestation « de la famille », révoque le mandat du compte joint, demande que « la mère paie tout », saisit son IBAN pour recevoir le remboursement d'un trop-perçu, interroge l'assistant IA. **Mme Sorel**, secrétaire, mère d'un élève, est curieuse : elle consulte le compte d'une collègue enseignante dont l'enfant est scolarisé, celui de la famille d'un élève dont on parle en salle des professeurs, regarde la liste des boursiers, et voudrait s'accorder la réduction « personnel » rétroactive.

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-769 | **Droits par défaut du secrétariat** : sur `ENT`, l'Administratif a `facturation.gerer` et `bourses.gerer` [F] ; il peut émettre, faire des avoirs, voir et gérer les bourses | La conception propose des droits fins en F1 ; d'ici là, tout le secrétariat a tout | important | 24, 1 | **FD-27**, F0 |
| M-770 | **Journal des consultations nominatives** des comptes par le personnel, consultable par la direction et le DPO | `tracer_acces` limité aux exports (FD-25) ; la curiosité ne laisse aucune trace | important | 24, 25 | FAC-244, R-F36, F0 (minimal) |
| M-771 | **Comptes protégés** : personnels parents d'élèves, ex-conjoint d'un personnel, famille signalée ; accès nominatif, motif obligatoire, alerte | Les comptes des collègues sont lus par des collègues | important | 24 | FAC-245, R-F35, S-185 |
| M-772 | **Alerte** quand un personnel consulte le compte d'une personne avec qui il a un lien déclaré (même foyer, ex-conjoint, même nom à l'adresse) | Détecte le conflit d'intérêts sans l'interdire aveuglément | confort | 24, 4 | FAC-246 |
| M-773 | **Remboursement vers un IBAN nouvellement saisi** : seconde validation et délai ; avis au payeur sur son canal habituel | Fraude du parent qui saisit son IBAN pour recevoir le crédit de l'autre ; fraude interne | important | 25 | PAY-238, R-P28, CF-92 |
| M-774 | **Demande de changement de répartition** par un seul parent → tâche ; acceptation du payeur désigné (M-713) | Porte d'attaque « la mère paie tout » | important | 24 | R-F30, FAC-218 |
| M-775 | **Attestation « de la famille »** : refusée ; seule l'attestation de **sa** part | Le secrétariat produit à la demande ; constat de la passe 21 | important | 25 | PAY-220, CF-93 |
| M-776 | **Assistant IA** : « est-ce que la mère a payé ? », « quelle est son adresse de facturation ? », « montre-moi le mandat » | 24 §19.2 point 11 le pose ; à étendre aux formulations indirectes (« combien reste-t-il à payer pour ma fille, au total ? ») : réponse limitée à **sa** part | important | 24, 1 bis | §16.2 (précisé), CF-94 |
| M-777 | **Réduction « personnel » à soi-même** ou à un proche ; rétroactive | Interdiction de traiter son propre compte (M-677) ; double validation (S-156) | important | 24 | R-F35, CF-96 |
| M-778 | **Avis financiers sur un canal partagé** : un ancien e-mail ou téléphone commun du foyer reste dans la fiche d'un parent | Les avis partent aux comptes des payeurs [F, `finances.rs:1595`] ; les e-mails et SMS suivent les coordonnées de la personne (socle 4) : vérifier qu'aucune coordonnée **de foyer** ne sert | important | 24, 4, 8 | CF-97, F0 (vérification) |

### Manques transverses : l'IA, le mobile, les tests

| ID | Ce qui manque | Pourquoi ça compte | Gravité | Socle | Ajout |
|---|---|---|---|---|---|
| M-682 | **Explication IA** de la bourse déduite, de la part, de la prise en charge d'un organisme (phrases fixes étendues) | Les phrases fixes du §14 ne couvrent que tarif, tranche, part | confort | 24, 1 bis | FAC-259, §16.1 (précisé) |
| M-683 | **Brouillon de réponse à une contestation** qui ne cite jamais l'autre payeur, sa part en euros, son adresse | Le brouillon IA lit la fiche de l'élève, qui connaît les deux payeurs | important | 24, 1 bis | §16.2 (précisé) |
| M-684 | **Appli mobile** : prévenir d'une difficulté, reporter, demander un plan, voir « ma part » (D-003, version quotidienne) | Le geste de prévenance n'a de valeur que s'il est à portée de main | confort | 24, 25, 11 | §14 (précisé) |
| M-685 | **Relevé simplifié** pour une famille qui lit mal (montant dû, prochaine date, comment payer, qui appeler) | Constat de la passe 21 (famille Garcia) ; la facture conforme est dense | confort | 24, 7 | §13.4 (précisé) |
| M-686 | **Matrice de séparation** étendue aux rôles nouveaux : co-signataire non payeur, payeur tiers, organisme, élève majeur payeur, parent d'un ex-mineur, personnel parent, personnel ex-conjoint | Les pièges §19.2 du 24 ne couvrent pas ces rôles | important | 24, 25 | §19.2 (pièges 12 à 16), §26 |
| M-687 | **Décès d'un co-signataire solidaire** : plus de recours contre lui ; la créance relève de la succession, traitement humain | Conséquence directe du bloquant A | confort | 24, 25 | CF-98 |
| M-688 | **Scénarios de labo** jouables pour ces personas (séparation au 1ᵉʳ décembre, surendettement, OGEC multi-établissements, départ avec impayé, attaquant interne) | La recette par personas (D-021 étape 4) suppose des données fictives prêtes | confort | 24, 25 | §26 (précisé) |
| M-689 | **Relances papier en lot** (mise en demeure imprimée, tri par adresse, comme le lot d'impression des bulletins de la passe 21, M-615) | Les mises en demeure partent par courrier ; une à une aujourd'hui | confort | 25, 7 | PAY-186 (précisé) |

---

## 4. Couverture du marché : Charlemagne (D-023, D-024)

| Fonction | Source | Dans la conception avant la passe | Après la passe | Où |
|---|---|---|---|---|
| Facturation annuelle, échéancier, prélèvement | FIN-01 à FIN-11 [H] | conçu | conçu ; **cohabitation an 1** outillée | F14 |
| Facture par payeur séparé | expérience de Florian [H] | conçu (répartition) | + **signataires et solidarité** distingués | R-F29 |
| Ensemble scolaire, fratrie commune | T-21 [H] | hors carte (Q13) | **colonne posée**, consolidation désactivée | R-F33 |
| Réductions de fratrie | FIN-05 [H] | par établissement | + fratrie d'entité (groupe consenti) | FAC-132 |
| Remise d'ordre | FIN-07 [H] | conçu | inchangé ; **pas** de remise de principe nationale (abrogée, §5 point 4) | FAC-230 |
| Relances, recouvrement | FIN-12 [H] | conçu | + surendettement, recours co-signataire, prévenance, report | PAY-212 à 218 |
| Export comptable | FIN-14 [H] | conçu | + balance auxiliaire, justification 411, dossier de pièces, comptes de tiers | FAC-237 à 240 |
| Attestations | FIN-16 [H] | conçu | + « ma part », payeur tiers | PAY-180, PAY-220 |
| Contrôle interne (journal de consultation, comptes protégés) | — [H : rarement proposé] | absent | conçu | FAC-244 à 246 |

**Ce qui rendrait ScolIA nettement meilleur** (différenciants retenus) : la prévenance avant rejet (M-704), le simulateur famille personnel (M-741), l'attestation « ma part » pour les parents séparés (M-717), l'application du jugement de répartition avec sa pièce (M-712), la cohabitation an 1 qui **corrige** les envois de Charlemagne (M-692), les comptes protégés et le journal de consultation (M-770, M-771).

---

## 5. Vérification réglementaire (2026-11-10)

Résultats du sous-agent de recherche ; versés au 24 §5.10 et au 25 §4.5.

| # | Point | Ce que dit le texte | Référence · source | Statut | Effet sur la conception |
|---|---|---|---|---|---|
| 1 | Solidarité des époux et des partenaires | Époux : toute dette d'entretien du ménage ou d'**éducation des enfants** contractée par l'un oblige l'autre **solidairement**, sauf dépense manifestement excessive. PACS : solidarité pour les besoins de la **vie courante** (l'éducation n'est pas nommée). La séparation de fait laisse subsister la solidarité ; elle cesse pour les dettes nées après la mention du divorce en marge de l'état civil | C. civ. **220**, **515-4** (version du 19/03/2014) ; Cass. 1re civ., 10/03/1998, n° 96-15.829 ; C. civ. 262 · `legifrance.gouv.fr/codes/article_lc/LEGIARTI000028748098`, `…/LEGIARTI000028748096`, `legifrance.gouv.fr/juri/id/JURITEXT000007041071` | [V] ; 262 [V partiel] ; jurisprudence propre aux frais de scolarité [NV] | Bloquant A (R-F29) ; la solidarité **légale** n'est jamais automatisée (elle supposerait de collecter et dater l'état matrimonial : minimisation) ; seule la **clause du contrat** est enregistrée ; la solidarité légale reste un argument du juriste |
| 2 | Contribution des parents, pension, majorité | Chaque parent contribue à proportion de ses ressources ; l'obligation **ne cesse pas** à la majorité ; la pension peut prendre la forme d'une **prise en charge directe de frais** ; majorité à 18 ans, capacité d'exercer ses droits | C. civ. **371-2**, **373-2-2** I, **414** · `…/LEGIARTI000006426470`, `…/LEGIARTI000042685533` | [V] ; 414 [V partiel] | P06 (le jugement répartit entre les parents) ; P09 (l'élève majeur peut devenir payeur, les parents restent tenus entre eux) |
| 3 | Surendettement | La recevabilité **suspend et interdit les procédures d'exécution** (dettes non alimentaires), jusqu'au plan ou au plus deux ans ; le débiteur a **interdiction de payer** une créance antérieure. Pour un EPLE, la **demi-pension n'est pas une créance alimentaire** ; répondre à l'état des créances sous **30 jours** ; effacement = non-valeur **sans reprise** ; rétablissement avec liquidation : déclarer sous **2 mois** après le BODACC | C. consom. **L. 722-2**, **L. 722-3**, **L. 722-5**, L. 711-1 ; instruction **M9.6** § 2.2.4.7.5 ; Cass. 2e civ., 28/03/2024, n° 22-12.797 · `legifrance.gouv.fr/codes/id/LEGISCTA000032224534`, PDF M9.6 sur education.gouv.fr | [V] ; L. 711-1 et arrêt de 2024 [V partiel] | M-707, M-710 ; 25 R-P25, PAY-213 à PAY-215 : **un prélèvement d'une échéance antérieure est un paiement interdit** |
| 4 | Remise de principe (public) | Le décret n° 63-629 est **abrogé depuis le 1er septembre 2016** (décret 2016-328, art. 27) ; la remise d'ordre n'a pas de texte national, elle relève du règlement du service de restauration et d'hébergement de la **collectivité** | `legifrance.gouv.fr/loda/id/JORFTEXT000000869930` ; C. éduc. R. 531-52 | [V] ; remise d'ordre [V partiel] | **Corrige** l'hypothèse de la passe : **pas** de remise de principe nationale (FAC-230 devient « réduction de fratrie selon le règlement de la collectivité ») |
| 5 | EPLE : Op@le, régies, recouvrement | M9.6 du **19/01/2026** (BO n° 7 du 12/02/2026) pour les EPLE sous **Op@le** ; régies d'EPLE : **décret 2020-542** du 7 mai 2020 et arrêté du 13/08/2020 (régisseur nommé par le chef d'établissement avec l'agrément de l'agent comptable ; l'ordonnateur ne peut être régisseur) ; recouvrement contentieux sur **autorisation de l'ordonnateur** (R. 421-68), sinon admission en non-valeur ; saisie administrative à tiers détenteur ; non-valeur votée par le **conseil d'administration** (seuil délégable), n'éteint pas la créance ; remise gracieuse par le CA sur avis conforme de l'agent comptable (R. 421-69), éteint la créance | M9.6 §§ 1.1.2.6, 2.2.4.6, 2.2.4.9 · PDF M9.6 (education.gouv.fr) | [V] ; calendrier de fin de GFC [NV] | **Corrige** la conception : la référence des régies d'EPLE est le décret **2020-542**, pas le 2019-798 (24 §5.9, 25 §4.2) ; P14 : rôles ordonnateur, agent comptable, régisseur ; autorisation de poursuivre ; non-valeur et remise gracieuse |
| 6 | Bourses nationales | Public : versée **par l'établissement après déduction** des frais de pension ou de demi-pension ; **privé** : versée par les **services académiques** aux personnes qui ont la charge de l'élève, ou, avec **procuration sous seing privé** au représentant légal de l'établissement, par lui **après déduction** ; paiement **à la fin de chaque trimestre** ; retenue au-delà de 15 jours d'absence injustifiée ; paiement possible au **boursier majeur** à la charge de personne ; montants 2026-2027 : bourse 504 à 1 071 €, **prime d'internat** 327 à 672 €, prime d'équipement 341,71 € ; versements fin décembre, mars, juin | C. éduc. **L. 531-4**, L. 531-2, **R. 531-19**, **R. 531-30**, **R. 531-31**, **R. 531-33**, **R. 531-34**, **R. 531-35**, D. 531-29 · `legifrance.gouv.fr/codes/id/LEGISCTA000020743143`, `service-public.gouv.fr/particuliers/actualites/A14944` | [V] ; L. 531-2 [V partiel] | Confirme FD-12 et R-F14 ; P08 : procuration **tracée**, calendrier trimestriel, retenue pour absences (régularisation FAC-138), prime d'internat ; P09 : bourse au majeur |
| 7 | Cantine et impayés | Primaire : l'inscription à la cantine est un **droit**, sans discrimination selon la situation de la famille ; Défenseur des droits (2019) : dans le second degré, la restauration est un service public administratif ; les impayés doivent se régler **entre la collectivité et les parents, sans impact sur les enfants** ; pas d'exclusion, pas de « déjeuner humiliant » | C. éduc. **L. 131-13** (loi 2017-86) ; rapport « Un droit à la cantine scolaire pour tous », mai 2019, recommandation n° 5 · `juridique.defenseurdesdroits.fr/doc_num.php?explnum_id=18987` | rapport [V] ; L. 131-13 [V partiel] | Confirme R-F20 et 25 S-177 (jamais de refus de repas) ; renforce l'argument face à une demande d'exclusion |
| 8 | Sorties et voyages | Circulaire du **13/06/2023** (abroge la 2011-117) : la contribution des familles aux sorties facultatives « doit être limitée et ne doit, **en aucun cas, conduire à l'exclusion d'un élève pour des raisons financières** » ; **aucune participation** pour un déplacement **obligatoire** ; les surcoûts liés au **handicap** ou à un besoin médical ne sont jamais imputés à la famille ; le CA approuve la programmation et le financement ; une circulaire du 16/07/2024 la remplacerait | circulaire NOR MENE2310475C (BO n° 26 du 29/06/2023) ; M9.6 § 2.5.2 ; circulaire NOR MENE2407159C (BO n° 30, 2024) | 2023 et M9.6 [V] ; 2024 [V partiel] ; gratuité des accompagnateurs [NV] | P13 : R-F20 précisé ; nouvelles règles : sortie obligatoire **gratuite**, surcoût handicap jamais facturé (R-F41) ; applicable au public, **bonne pratique** recommandée au privé [R] |
| 9 | Chèques-vacances | Un EPLE peut les accepter pour les voyages s'il est **conventionné** avec l'ANCV (convention adoptée par le CA, régisseur habilité) ; tout prestataire doit être conventionné | M9.6 § 2.2.4.4.6 ; C. tourisme R. 411-7 [V partiel] | EPLE [V] ; OGEC [NV] | 25 PAY-167 : désactivé par défaut, activable avec la référence de la convention |
| 10 | CNIL : gestion des impayés | Référentiel du 28/01/2022 : accès réservé aux **personnes habilitées**, habilitations documentées, **accès tracés** ; partage de listes d'impayés avec des tiers **hors champ** (analyse propre, voire AIPD) ; effacement **48 h** après régularisation, **5 ans** au plus ; **information en trois temps** (collecte, impayé avec moyens de régulariser et droit de présenter des observations, inscription au fichier) ; décision entièrement automatisée → art. 22 du RGPD | `cnil.fr/sites/cnil/files/atoms/files/referentiel_traitements-donnees-caractere-personnel_gestion-impayes_transaction_commerciale.pdf` | [V] | Confirme R-F21 ; P15, P16 : journal des consultations (R-F36) ; aucune liste aux professeurs ; **information en trois temps** dans la notice (FAC-241) et au palier 1 de relance (25 PAY-185) |
| 11 | Réduction d'impôt | 61 € collège, 153 € lycée, 183 € supérieur ; **moitié en résidence alternée** | `service-public.gouv.fr/particuliers/vosdroits/F9` | [V] (texte du CGI non relu) | Confirme 24 §5.6 ; M-718 : certificat de scolarité à chaque parent |
| 12 | Exeat et impayé | Réponse ministérielle : l'exeat certifie que l'élève est « en règle avec la structure qu'il quitte », par exemple que les sommes dues sont réglées ; mais « aucune disposition législative ou réglementaire ne conditionne l'inscription » à l'exeat et « aucune disposition réglementaire n'impose » sa délivrance ; privé sous contrat : contrat de droit privé, juge judiciaire | QE n° 18504 (JO du 09/04/2019, réponse JO du 15/10/2019, p. 8919) · `assemblee-nationale.fr/dyn/15/questions/QANR5L15QE18504.pdf` | [V] ; rétention du livret ou du dossier [NV] | **Nuance** M-742 : ScolIA **sépare** le **certificat de radiation** et les documents scolaires (jamais retenus) de l'**attestation de situation financière** (25 PAY-182, qui dit la vérité sur les sommes dues) ; aucun des deux ne bloque le départ ; validation juridique (Q-F31) |
| 13 | Chorus Pro | L'obligation vise les **titulaires de marchés** conclus avec des personnes publiques ; une collectivité peut refuser une facture hors Chorus Pro ; hors lien contractuel (décision unilatérale), l'obligation ne s'appliquerait pas | CCP **L. 2192-1** (version du 01/07/2024) ; FAQ DGCL du 29/07/2026 · `collectivites-locales.gouv.fr/FAQ/faq-la-facturation-electronique-pour-la-gestion-publique-locale` | [V] ; exclusion hors contrat [V partiel] ; cas de l'ASE [NV] | Q-F32 : une prise en charge de l'ASE hors marché n'est **probablement pas** soumise ; le département peut l'exiger en pratique : F11 reste utile mais **non bloquant** |
| 14 | FEC, plan comptable | FEC remis lors d'une vérification par tout contribuable tenu de présenter une comptabilité informatisée ; le BOFiP vise BIC, IS, BNC, BA ; règlement ANC **2018-06** pour les associations (exercices ouverts depuis le 01/01/2020) | LPF **L. 47 A** ; BOI-CF-IOR-60-40-10 (2017) ; règlement ANC 2018-06 | L. 47 A [V] ; BOFiP, ANC [V partiel] ; obligation propre à un OGEC [NV] | M-702 : FEC conforme produit comme **bonne pratique** ; obligation à qualifier par le cabinet (activité lucrative imposable) ; plan de comptes ANC 2018-06 |
| 15 | Majeur protégé | Tutelle : le **tuteur** représente (signe) ; curatelle simple : la personne signe seule un acte d'administration ; curatelle **renforcée** : le **curateur** règle les dépenses (signe) ; le jugement peut aménager la liste | C. civ. **473**, **467**, **472** · `…/LEGIARTI000006427839`, `…/LEGISCTA000006165757` | [V] | CF-85 précisé |
| 16 | Paiement par un tiers | « Le paiement peut être fait même par une personne qui n'y est pas tenue, sauf refus légitime du créancier » | C. civ. 1342-1 | [NV] (texte connu, non relu dans cette passe) | 25 PAY-219, R-P27 |

**Pages en échec** : education.gouv.fr et le BO en ligne (403), site du Défenseur des droits (503) ; les PDF officiels ont été lus directement (M9.6, BO 2023 relayé par l'académie de Nancy-Metz, rapport du Défenseur des droits, référentiel CNIL).

---

## 6. Manques qui relèvent d'autres socles

| Socle | Manques | Ce qu'il faudra prévoir |
|---|---|---|
| 3 Inscriptions | M-672, M-733, M-742, M-743 | Contrat de scolarisation avec signataires et solidarité ; radiation jamais conditionnée par le solde ; événement de départ lu par le 24 |
| 4 Dossier administratif | M-712, M-713, M-714, M-725, M-732, M-778 | Pièce de répartition ; acceptation du payeur désigné ; calendrier d'alternance ; organisme employeur ; aucune coordonnée de foyer pour les avis financiers |
| 7 Documents | M-699, M-718, M-751, M-767 | Dossier de pièces d'audit ; certificat de scolarité à chaque parent ; engagement de voyage signé ; notice d'information |
| 8 Notifications | M-749 | Langue des avis financiers |
| 26 Aides | M-708, M-727 à M-730, M-752 | Demande d'aide par la famille ; versements de l'État par trimestre ; primes |
| 40 Restauration | M-714 | Consommations datées pour le calendrier d'alternance |
| 41 Internat | M-747 | Nuitées et cautions |
| 42 Voyages | M-751 à M-756 | Engagement, aides, budget, départ jamais conditionné sans décision |
| 48 Comptabilité | M-679, M-698 à M-703 | Balance auxiliaire, 419, PCA, provision |
| 9 Interop | M-761 | Formats Op@le |
| 1 / 1 bis Cœur, IA | M-669, M-675, M-683, M-776 | Délégations datées ; refus des questions indirectes sur un autre payeur |
| 11 Mobile | M-684 | Prévenance, report, plan, « ma part » dans l'appli |

---

## 7. Synthèse

### 7.1 Manques par gravité

| Gravité | Nombre |
|---|---|
| bloquant | 5 (trois points : A = M-673, M-711 ; B = M-690, M-691 ; C = M-737) |
| important | 63 |
| confort | 41 |
| différenciant | 1 |
| **Total** | **110** (M-669 à M-778) |

**109** manques sont traités par un ajout dans `24_FINANCES.md` ou `25_PAIEMENTS.md` ; **1** est renvoyé à un autre socle (M-718, socle 7).

### 7.2 Ajouts dans les conceptions

| Partie | 24 | 25 |
|---|---|---|
| Actions | **47** (FAC-215 à FAC-261) ; total du socle : 162 | **28** (PAY-212 à PAY-239) ; total : 140 |
| Règles | **13** (R-F29 à R-F41) | **9** (R-P25 à R-P33) |
| Cas limites | CF-73 à CF-100 (28, communs, au 24 §15.3) | renvoi |
| Règle voyages | R-F41 (sortie obligatoire gratuite, handicap) | — |
| Réglementaire | §5.10 | §4.5 |
| Défauts | FD-27 | — |
| Lots | **F14** (cohabitation an 1), **F15** (confidentialité, contrôle interne, RGPD), **F16** (départs et soldes de sortie) ; compléments de F0, F2, F4, F7, F8, F13 | **P13** (situations légales : surendettement, co-signataire, décès), **P14** (organismes publics et régie) ; compléments de P2, P3, P6, P7 |
| Questions | Q-F26 à Q-F33 | Q-P15 à Q-P20 |
| Réglages | S-179 à S-186 | — |
| Contradictions | C-F7 à C-F12 (§29) | C-P5 à C-P7 (§27) |

### 7.3 Les dix ajouts les plus importants

1. **Débiteur du contrat ≠ répartition** (R-F29, FAC-215, PAY-212 ; M-673, M-711). À poser dans le schéma de F2 et F4.
2. **Cohabitation de l'an 1 outillée** (F14, FAC-220 à 223 ; M-690 à M-694). Charlemagne facture, ScolIA dit qui paie et distribue.
3. **Entité de facturation** (R-F33, FAC-224, PAY-226 ; M-737 à M-739). Colonne posée en F2, consolidation désactivée.
4. **Surendettement** (R-P25, PAY-213 à 215 ; M-707, M-710). Effets légaux appliqués, pas une simple « situation ».
5. **Le secrétariat au guichet, pas dans tous les comptes** (FD-27, R-F37, FAC-250 ; M-695, M-769).
6. **Contrôle interne** : journal des consultations, comptes protégés, interdiction de traiter son propre compte, remboursement vers un IBAN neuf (R-F35, R-F36, R-P28, R-P29 ; M-677, M-770 à M-773).
7. **Un parent ne désigne pas l'autre** (R-F30, FAC-217, FAC-218 ; M-712, M-713, M-774).
8. **La radiation ne dépend jamais du solde** ; solde de sortie proposé (R-F34, F16 ; M-742 à M-745).
9. **La famille agit avant le rejet** : prévenance, report, date au choix, demande d'aide (PAY-216 à 218, FAC-261 ; M-704 à M-708).
10. **Le public n'est pas codé comme le privé** (R-F40, P14 ; M-757 à M-761).

### 7.4 Ce que la passe a montré sur la conception

- **La conception pense la facture ; le droit pense le contrat.** Qui reçoit la facture (répartition) et qui doit l'argent (signataires, solidarité légale des époux) sont deux faits ; les confondre fait perdre des recours ou en invente.
- **L'an 1 est un mode de fonctionnement**, comme pour les notes (D-032) : ScolIA y est le **référentiel des payeurs** et le **distributeur**, Charlemagne le calculateur.
- **L'établissement n'est pas toujours le créancier.** L'OGEC peut l'être pour plusieurs établissements ; D-013 protège les familles entre établissements de **tenants** différents, pas la comptabilité d'une même personne morale.
- **Les fuites financières probables viennent de l'intérieur** : un ex-conjoint qui demande, un collègue qui regarde, une comptable qui est aussi mère. Les routes sont tenues (RS3-01 corrigée) ; les **personnes** ne le sont pas encore.
- **La famille veut agir avant le problème** (prévenir, reporter, demander de l'aide) ; la conception la fait surtout réagir (rejet, relance, plan).
- **D-023 tient**, avec huit réglages signalés (§8 bis).

---

## 8. Décisions : appliquées par défaut (D-031) et soumises à Florian

### 8.1 Appliquées par défaut (D-031)

| # | Décision | Où |
|---|---|---|
| 1 | Engagement = signataires + clause de solidarité ; facturation selon la répartition ; recours co-signataire = proposition, décision ★ de la direction | Q-F26 (principe), R-F29 |
| 2 | Lot **F14** cohabitation an 1, juste après F0 | Q-F27 |
| 3 | Entité de facturation posée en F2/P2 ; consolidation désactivée tant que Q13 n'est pas décidée | Q-F28 |
| 4 | Surendettement : gel légal des créances antérieures, échéances nouvelles maintenues, déclaration de créance préparée | Q-P15 |
| 5 | Secrétariat : vue guichet, `facturation.lire` restreint au payeur présent ; `facturation.gerer` et `bourses.gerer` retirés du rôle Administratif par défaut (F0, FD-27) | Q-F30 |
| 6 | Radiation jamais conditionnée par le solde | Q-F31 |
| 7 | Comptes protégés automatiques pour les personnels parents ; journal des consultations | S-185, R-F35, R-F36 |
| 8 | Un parent ne désigne pas l'autre ; acceptation ou pièce | R-F30 |
| 9 | Paiement par un tiers sur référence accepté (C. civ. 1342-1) ; le tiers ne voit que son paiement | Q-P16 |
| 10 | Prévenance avant lot ; report une fois par an ; date de prélèvement au choix entre deux dates | Q-P17, Q-P18 |
| 11 | Remboursement vers un IBAN neuf : seconde validation + 48 h | Q-P19 |
| 12 | Public : régime d'établissement, aucune règle du privé en dur ; ordonnateur ≠ comptable | R-F40, R-P33 |
| 13 | Produits constatés d'avance produits à chaque clôture (FAC-198) | C-F9 |
| 14 | F0 **vérifie** RS3-01, RS3-02, RS3-04 (désormais sur `ENT`) au lieu de les rapatrier | §25.1 du 24 |

### 8.2 Soumis à Florian (engagent l'établissement à l'extérieur)

| # | Point | Qui | Question |
|---|---|---|---|
| E1 | **Clause de solidarité** du contrat de scolarisation ; recours contre un co-signataire ; refus de réinscription | **juriste** (coût) | Q-F26, Q-P9 |
| E2 | **Analyse d'impact**, durées de conservation, notice, journal des consultations | **DPO** | Q-F29, Q-P11 |
| E3 | **Ensemble scolaire** : l'OGEC du lycée gère-t-il d'autres établissements ? Groupe consenti ? | Florian (fait) ; puis OGEC | Q-F28, Q13 |
| E4 | **Format d'import Charlemagne** (faits, payeurs) et coupure des envois par le portail de l'éditeur | Florian, éditeur | Q-F27 |
| E5 | **Banque** : deux dates de prélèvement (coût par lot ?), relevés camt, EBICS, ICS | **banque** | Q-P18, Q-P2, Q-P3 |
| E6 | **Expert-comptable** : balance auxiliaire, 419, PCA, FEC, provision, liste des natures | cabinet | Q-F17, Q-F8 |
| E7 | **Op@le** : formats d'échange (si ouverture au public) | académie / éditeur | Q-F33 |
| E8 | **Données réelles** : export des payeurs de Charlemagne pour F14 et la double tenue | Florian | Q-F2 |

---

## 8 bis. Réglages signalés (D-023)

Recopiés au 24 §27 ; **rien n'est supprimé**.

| # | Réglage | Pourquoi il pèse | Version automatique proposée |
|---|---|---|---|
| S-179 | Répartition des consommations selon le calendrier d'alternance | utile mais complexe (semaines paires/impaires, vacances) | **non** : pourcentages ; activable par élève avec le calendrier du jugement |
| S-180 | Date de prélèvement au choix | chaque date = un lot | deux dates (5 et 15) |
| S-181 | Report d'échéance à la demande | risque de glissement | une fois par an, sans justification |
| S-182 | Facture consolidée d'entité | dépend de Q13 | désactivée |
| S-183 | Recours contre un co-signataire solidaire | délicat humainement | désactivé ; direction seulement |
| S-184 | Paiement pour le compte d'un autre payeur | risque de conflit | autorisé sur référence |
| S-185 | Comptes protégés automatiques | — | oui pour les personnels parents |
| S-186 | Langue de l'explication financière | traduction | français + langue de la famille pour l'explication, document officiel en français |

---

## 9. Faits, hypothèses, contradictions

**Faits [F]** vérifiés sur `origin/ENT` `ee7706d3` : `foyer_ids_parent` filtre `est_payeur` (`ent-db/src/service.rs:18194-18205`) ; `executer_sepa_atomique` (`ent-db/src/fac_repo.rs:1028`, appelé par `service.rs:19331`) ; migrations `376_sepa_unicite_encaissement.sql` et `377_edt_exception_absence_etab.sql` présentes ; export historique toujours en `GET` avec `cdtr_iban` en paramètre (`ent-api/src/routes.rs:11019-11038`) ; `RECETTE_SECURITE_3.md` : RS3-03 « partiellement corrigée » (le prélèvement d'un foyer en paiement manuel est refusé ; pré-notification et verbe restent ouverts) ; rôles Admin, Direction, **Administratif** et Compta reçoivent `facturation.gerer`, `facturation.lire`, `bourses.gerer`, `repas.gerer` (`ent-auth/src/actions.rs:996-1000`) ; avis financiers insérés pour les comptes des payeurs du foyer (`ent-db/src/finances.rs:1567-1610`).

**Vérifié [V]** : voir §5 (16 points ; 11 [V], 4 [V] avec parties [V partiel] ou [NV], 1 [NV]).

**Hypothèses [H]** : OGEC multi-établissements ; contrats « solidaires » ; retenue de l'exeat ; prélèvement sur compte joint après séparation ; fréquence du surendettement ; pratiques de Charlemagne.

**Contradictions signalées** (au 24 §29 et au 25 §27, tranchées par défaut selon la recommandation) : C-F12 (« remise de principe » supposée en vigueur : abrogée, §5 point 4) ; C-P7 (25 §4.2 : décret 2019-798 cité pour les régies ; pour un EPLE, décret 2020-542) ; C-F7 (« parcours minimal pour un pilote réel » et finances dans Charlemagne l'an 1) ; C-F8 (F13 « après le pilote » et schéma F2–F5 qui fige le privé) ; C-F9 (produits constatés d'avance « si demandé ») ; C-F10 (R-F5 « un compte par établissement » et OGEC créancier de plusieurs établissements) ; C-F11 (« un payeur ne voit que ses parts » présenté comme la limite de la dette) ; C-P5 (CF-58 interdit l'imputation sur le compte du père sans accord des deux ; le paiement par un tiers est licite) ; C-P6 (R-F5 « chaque payeur ne doit que sa part » et solidarité contractuelle ou légale).
