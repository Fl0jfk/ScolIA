# Socle 51 — Post-bac et apprentissage (BTS, CPGE, apprentis, UFA et CFA)

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian). Rédigée avant tout code propre au post-bac et à l'apprentissage. Aucun code écrit ni modifié. Aucun accès à une base, à un secret ni à un environnement de production. Le code Next.js a été **lu seulement**, de façon ciblée.
**Branche :** `ent-socle-50-51-conception`, partie de `origin/ent-mc0hoe`.
**Passe personas :** `docs/socles/PERSONAS_51.md` ; ajouts marqués **« ajout personas »**, surtout au §27, qui **l'emporte** en cas de contradiction.
**Socle jumeau :** `50_PREMIER_DEGRE.md` (même branche), dont ce socle reprend l'**unité scolaire** (§6 du 50) : une section de BTS, une CPGE, une unité de formation par apprentissage (UFA) sont des **unités** de l'établissement.
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` D-013, D-017, D-020, D-022, D-023, D-026, D-027, D-031, D-032, D-034, D-035, D-036, D-038, D-041 ; `docs/SOCLES.md` §51 et §39 ; `PRA_SAUVEGARDES.md` §18.4 ; conceptions `02_ELEVE.md` (statut d'apprenant, l. 340), `04_ADMINISTRATIF.md` (élève majeur), `06_PERSONNELS.md` (P48 AED étudiante), `07_DOCUMENTS.md` (l. 145, 187–189 : attestations d'assiduité, certificat de réalisation, contrat importé ; R-D13 entités émettrices), `09_INTEROP.md` (FX-42 SIFA, FX-56 Sirene), `10_EMPLOI_DU_TEMPS.md` (R-02 pondération BTS 1,25 ; CL-73 mixité des publics), `12_ASSIDUITE.md` (RA-29, RA-31, AS-188, AS-189, AS-203, AS-204, AS-242, R-AS34), `17_ANNONCES.md` (PD-07), `18_FORMULAIRES.md` (FD-04), `21_EVALUATIONS.md` (CE-20), `24_FINANCES.md` (FAC-219, S-179), `PERSONAS_02-04.md` (P18, M-099).

**Conventions.** [F] fait vérifié dans le code ; [V] lu sur une source officielle (URL au §5, consultée le 2026-11-11) ; [V-partiel] confirmé par l'extrait de la source, version consolidée non relue en entier ; [NV] non vérifié ; [H] hypothèse ; [R] recommandation.

**Numérotation propre au socle** (vérifiée libre sur toutes les branches distantes le 2026-11-11) :

| Élément | Numéros |
|---|---|
| Défauts de l'existant | **PBD-01**… |
| Actions | **PBA-100 à PBA-299** |
| Règles | **R-PBA1**… |
| Cas limites | **CPB-01**… |
| Questions | **Q-PBA1**… |
| Réglages (D-023) | **S-5100 à S-5119** |
| Lots | **PBA0 à PBA14** |
| Manques (passe personas) | **M-5100 à M-5199** |

**Plage de migrations imposée par le pilote : 3020–3039** (inscrite dans `MIGRATIONS.md`) ; aucun fichier `30xx_` n'existe sur les branches distantes au 2026-11-11.

Sommaire : §0 En bref · §1 Thèse · §2 Existant · §3 Acteurs · §4 L'année · §5 Réglementaire · §6 Deux modèles d'apprentissage dans un lycée · §7 Le statut d'apprenant, daté · §8 Ce qui change dans les autres socles · §9 BTS · §10 CPGE · §11 Bourses et assiduité étudiante · §12 Contrat d'apprentissage · §13 Alternance et assiduité de l'apprenti · §14 Entreprise, maître d'apprentissage, suivi · §15 Financement par les OPCO · §16 Obligations du CFA, qualité, indicateurs · §17 Taxe d'apprentissage · §18 Cas limites · §19 IA · §20 Actions · §21 Règles · §22 Droits et minimisation · §23 Événements et notifications · §24 Écrans · §25 Modèle de données · §26 Lots et migrations · §27 Ajout personas · §28 Décisions et questions · §29 Faits, hypothèses, contradictions.

---

## 0. En bref

1. **Trois publics, trois statuts, souvent dans la même salle** : l'**étudiant** de BTS ou de CPGE sous statut scolaire (souvent majeur, parfois boursier du CROUS) ; l'**apprenti** (salarié d'une entreprise, en formation dans l'UFA du lycée ou dans un CFA) ; le **lycéen** de bac professionnel qui passe en apprentissage en cours d'année. Le logiciel actuel porte un seul `statut_apprenant` par **année** d'inscription, réécrit sur place (`migrations/284_eleve_inscription.sql:20,35`, `crates/ent-db/src/scolarite.rs:896-904`) [F] : il ne sait ni **quand** un élève devient apprenti, ni ce que cela change.
2. **Le statut change les règles, pas l'élève** (§7) : un apprenti absent au CFA est **absent à son travail** et son employeur doit le savoir (L6222-24 [V], RA-31 du 12) ; un étudiant boursier non assidu voit sa bourse **suspendue** à la date constatée, après une procédure contradictoire (circulaires annuelles des bourses [V-partiel]) ; un étudiant majeur est **seul destinataire** de ses informations (R-AS34 du 12) ; un étudiant de BTS en lycée **n'est pas assujetti à la CVEC**, un étudiant de CPGE l'est par son inscription universitaire [V].
3. **Deux modèles d'apprentissage dans un lycée** (§6) — c'est la décision structurante :
   - **M1 « UFA rattachée à un CFA »** : le lycée enseigne ; un **CFA gestionnaire** (souvent régional ou de réseau, avec son propre logiciel de type Ypareo) porte les contrats, la relation avec les OPCO, la certification Qualiopi, le bilan pédagogique et financier. ScolIA gère la **scolarité** de l'apprenti (cours, assiduité, évaluations, livret) et **fournit au CFA** ce qu'il lui faut (assiduité mensuelle, données des certificats de réalisation) ;
   - **M2 « ScolIA, logiciel du CFA »** : l'organisme de gestion est lui-même CFA ; ScolIA porte en plus le contrat, l'OPCO, la facturation, Qualiopi, le BPF, SIFA.
   **Recommandation : M1 d'abord** (cas probable du lycée pilote [H], Q-PBA1), M2 ensuite, avec un modèle de données qui supporte les deux dès le départ.
4. **Réglementaire vérifié** (§5) : contrat de 6 mois à 3 ans, formation en centre d'**au moins 25 %** de la durée du contrat sauf règle du certificateur (L6211-2) [V] ; **période d'essai de 45 jours, consécutifs ou non, de formation pratique en entreprise** (L6222-18) [V] ; rupture anticipée par écrit notifiée au **directeur du CFA** et à l'organisme de dépôt (R6222-21) [V-partiel] ; 14 missions du CFA (L6231-2) et **conseil de perfectionnement** (L6231-3, R6231-3) [V-partiel] ; financement OPCO **au prorata des jours** depuis le 1er juillet 2025, versements **40 / 30 / 20 / 10 %**, solde après **certificat de réalisation** (décret 2025-585, R6332-25) [V-partiel] ; **déclaration de dérogation** avant d'affecter un mineur à des travaux réglementés, valable **3 ans**, par l'employeur **ou le chef d'établissement** (R4153-38 à R4153-52) [V] ; BTS = **120 crédits européens** ; CCF possible si l'établissement est **habilité** (CFA hors Éducation nationale : habilitation du recteur) ; durée minimale en CFA **1 350 h sur 2 ans** pour un BTS (D643-8, cité par le vademecum de 2025) [V-partiel] ; CPGE : formation supérieure de deux ans, **60 crédits par an, 120 au total**, **inscription parallèle obligatoire** dans un établissement public d'enseignement supérieur conventionné (L612-3, D612-19 et s.), **attestation descriptive** (D612-25) [V-partiel] ; **solde de la taxe d'apprentissage** versé par les employeurs sur **SOLTéA** aux établissements **habilités** (L6241-5), habilitation à redemander chaque année (pour 2027 : dossier avant le 11 décembre 2026) [V].
5. **Existant** : rien de propre au socle, sauf `statut_apprenant` (284), le module de stages (`082_stage.sql`, `domain-eleve/src/stage.rs`) que le **socle 39** (pas encore conçu) possède, l'engagement « apprenti » d'un **personnel** (`352_agents.sql:86`, un apprenti salarié de l'OGEC), et l'export Parcoursup (`parcoursup_export.rs`, socle 37). Next.js : niveaux « BTS » dans les offres de stage (`app/stages/candidater/page.tsx:6`, `app/components/stages/StagesOffersPanel.tsx:8`), BTS rangé au « lycée » par expression régulière (`app/lib/nomenclature-classes.ts:71`) [F]. **Lot PBA0** (correctifs urgents, §2.2) : statut réécrit sans historique, pas de valeur « étudiant », échappeur CSV de l'export Parcoursup.
6. **Le plan** :

   | Élément | Volume |
   |---|---|
   | Actions | 122 (numérotées PBA-100 à PBA-271 par blocs, trous réservés ; dont 8 de la passe personas) |
   | Règles | 46 (R-PBA1 à R-PBA46) |
   | Cas limites | 69 (CPB-01 à CPB-69, dont 7 de la passe personas) |
   | Réglages | 19 (S-5100 à S-5118, dont 6 de la passe personas) |
   | Lots | 15 (PBA0 à PBA14) |
   | Migrations | 3020–3039 |

   **Parcours minimal [R]** : PBA0, PBA1 (statut daté), PBA2 (unités post-bac), PBA3 (BTS : crédits, CCF, stages), PBA5 (bourses et assiduité étudiante), PBA6 (alternance et assiduité de l'apprenti), PBA7 (contrat en lecture, M1), PBA8 (échanges avec le CFA). Les lots M2 (OPCO, facturation, Qualiopi, BPF, SIFA) viennent après la décision Q-PBA1.
7. **Décisions appliquées par défaut (D-031)** : 26 (§28.1). **Soumis à Florian** (§28.2) : M1 ou M2 et le nom du CFA partenaire ; formations réellement ouvertes (BTS, CPGE, apprentissage) dans le groupe ; convention avec une université pour la CPGE ; habilitation SOLTéA ; démarches officielles (Parcoursup établissement d'accueil, DECA, API) ; avis du DPO sur les données d'apprentis (salariés, mineurs, santé au travail).

---

## 1. Thèse : un apprenant a un parcours et des statuts datés ; chaque statut apporte ses destinataires, ses obligations et ses comptes

Au lycée, un élève a une classe et des responsables. En post-bac et en apprentissage, il a **un contrat**, **un employeur**, **un financeur**, **un rythme d'alternance**, **une bourse**, **une convention universitaire**, **des crédits** ; il change de statut en cours d'année ; il est majeur ou va l'être ; ses parents ne sont plus forcément ses interlocuteurs. ScolIA doit :
- **garder l'élève au centre** (socle 2) : une seule fiche, du lycée au BTS, de la classe de bac professionnel au contrat d'apprentissage ;
- **dater le statut** et en **déduire** les règles : qui est prévenu d'une absence, ce qu'elle déclenche, ce qui est facturable et à qui ;
- **produire les preuves** que l'extérieur exige (assiduité mensuelle pour l'employeur et le CFA, certificat de réalisation pour l'OPCO, liste d'assiduité pour le CROUS, attestation descriptive de CPGE) sans **jamais** envoyer seul (décision humaine) ;
- **ne pas devenir Ypareo** si le CFA en a déjà un (M1) : échanger proprement plutôt que dupliquer.

---

## 2. Évaluation franche de l'existant

### 2.1 Ce qui existe [F]
| Élément | Où | Constat |
|---|---|---|
| `statut_apprenant` sur l'inscription annuelle | `migrations/284_eleve_inscription.sql:20,35` ; `crates/ent-db/src/scolarite.rs:103,188,896-926` ; `crates/ent-types/src/scolarite.rs:72-74,179` | valeurs `scolaire`, `apprenti`, `autre` ; **une** valeur par année ; **mise à jour en place** (`UPDATE … statut_apprenant = $6`) ; **aucune règle ne la lit** [F, recherche des usages : seulement lecture et écriture de la colonne] |
| Stages, entreprises, conventions | `migrations/082_stage.sql` ; `crates/domain-eleve/src/stage.rs` | `ent_entreprise` par établissement (SIRET de 14 caractères, sans contrôle de clé), `ent_convention_stage` (tuteur en **texte**, statut) ; FK simples ; socle 39 |
| Engagement « apprenti » d'un personnel | `migrations/352_agents.sql:86` | un apprenti **salarié de l'OGEC** (ex. au secrétariat) : c'est un personnel (socle 6), distinct d'un apprenti **apprenant** de l'UFA |
| Export Parcoursup | `crates/domain-eleve/src/parcoursup_export.rs` | fiche Avenir des terminales (socle 37) ; **échappeur sans neutralisation des formules** (`:40`), comme l'export ONDE (ED-03 du 50) |
| Pondération BTS | `10_EMPLOI_DU_TEMPS.md` R-02 | coefficient 1,25 des heures de BTS (service des enseignants) [V par le socle 10] |
| Next.js : niveaux BTS dans les stages | `app/stages/candidater/page.tsx:6`, `app/components/stages/StagesOffersPanel.tsx:8` | offres de stage ouvertes aux BTS |
| Next.js : BTS classé « lycée » | `app/lib/nomenclature-classes.ts:71` ; `app/lib/stage-sector-scope.ts:33` (« Classe atypique (CAP, BTS, ULIS…) : ne pas masquer ») | rangement par expression régulière, ouvert par défaut |

### 2.2 Défauts (lot PBA0 pour ceux de Rust ; Next.js signalé à Florian, intouchable)
| # | Gravité | Défaut | Preuve | Correctif |
|---|---|---|---|---|
| **PBD-01** | élevée | Le statut d'apprenant est **réécrit** : un lycéen qui signe un contrat le 15 novembre devient « apprenti » **depuis septembre** ; ses absences d'octobre changent de régime rétroactivement | `scolarite.rs:896-904` | PBA0 : écriture refusée si l'année a commencé et que des faits existent ; PBA1 : statut **daté** (§7) |
| **PBD-02** | moyenne | Pas de valeur **étudiant** (BTS, CPGE sous statut scolaire) ; « autre » sert de fourre-tout | `284:35` | PBA1 : `etudiant`, `stagiaire_formation_continue` ; reprise de « autre » proposée, jamais automatique |
| **PBD-03** | moyenne | Le statut ne commande **rien** (ni destinataires, ni assiduité) : un employeur n'est jamais informé, un étudiant boursier n'a aucun suivi | usages de la colonne | PBA5, PBA6 |
| **PBD-04** | moyenne | Export Parcoursup sans neutralisation des formules | `parcoursup_export.rs:40` | PBA0 = ECO-271 du 50 (échappeur commun) |
| **PBD-05** | faible | `ent_entreprise.siret` : longueur seulement ; pas de FK composite ; tuteur en texte libre | `082:7-11, 25` | socle 39 ; attente posée (§8) |
| **PBD-06** | info (Next.js) | BTS rangé au lycée par expression régulière ; filtre ouvert par défaut | §2.1 | Unités explicites (50 §6, R-ECO2, R-ECO3) |

### 2.3 Verdict
Rien de métier n'est à reprendre ; deux réflexes sont à corriger tout de suite (statut réécrit, échappeur). Le socle se construit **sur** les socles 2 à 29 et **avec** le socle 39 (entreprises, tuteurs, conventions), qu'il complète par le contrat d'apprentissage.

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps / ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|
| **Directeur délégué aux formations (DDFPT), coordinateur de l'UFA** | calendrier d'alternance, relations entreprises, visites, suivi des contrats, liaison avec le CFA | Ressaisir les absences dans le logiciel du CFA ; contrats rompus appris trop tard ; périodes d'essai qui expirent sans qu'on le sache | Un tableau « contrats » : essais qui finissent, ruptures, apprentis sans entreprise, absences du mois à transmettre |
| **Coordinateur de BTS, professeur principal de CPGE** | stages, CCF, colles, crédits, conseil de classe | Grilles de CCF sur tableur ; crédits attribués à la main ; colloscope en Excel | CCF planifiés et tracés, crédits calculés, colloscope généré et publié |
| **Professeurs** (dont enseignants de « domaine professionnel ») | cours en classe mixte (apprentis + scolaires), évaluations, visites en entreprise | Ne pas savoir qui est en entreprise cette semaine ; appeler des absents qui sont au travail | L'appel qui sait qui est en entreprise ; la visite planifiée avec le maître d'apprentissage |
| **Colleur (interrogateur de CPGE)**, souvent vacataire | interrogations orales par trinômes, notes | Feuilles de notes perdues ; paiement des heures | Sa liste du jour sur mobile, notes saisies sur place, heures comptées |
| **CPE / vie scolaire post-bac** | assiduité, justificatifs, liste CROUS | Signaler au CROUS un étudiant qui avait un justificatif ; parents qui appellent pour un majeur | Justificatif demandé avant tout signalement ; parents non destinataires par défaut |
| **Direction** | signe la liste CROUS, les conventions, les attestations ; préside le conseil de perfectionnement | Signer à l'aveugle | Une liste vérifiée, avec la preuve de la procédure contradictoire |
| **Secrétariat** | inscriptions Parcoursup, cartes d'étudiant, attestations (CAF, CROUS, employeur), dossiers d'examen | Demandes d'attestations hors des heures d'ouverture | Attestations en libre-service sur le compte de l'étudiant (socle 7) |
| **Gestionnaire du CFA / de l'UFA** (M2) | contrats, OPCO, factures, certificats de réalisation, BPF, SIFA, Qualiopi | Relances OPCO ; factures rejetées pour un certificat manquant ; ruptures au prorata | Factures générées depuis l'assiduité, certificats signés, échéancier OPCO suivi |
| **Comptable de l'OGEC** | encaissements OPCO, taxe d'apprentissage | Rapprocher les versements OPCO à la main | Rapprochement par contrat (socle 25, 48) |
| **Apprenti** (16 à 29 ans, parfois mineur) | alterne entreprise et formation | Ne pas savoir où il doit être ; ses absences envoyées à son patron sans qu'il le sache | Son calendrier, ses absences, ce qui part chez l'employeur, son livret |
| **Responsables légaux d'un apprenti mineur** | cosignent le contrat ; sont informés | Ne rien savoir | Être informés de ce que la loi prévoit, sans voir sa paie ni son évaluation d'entreprise |
| **Étudiant de BTS, de CPGE** (majeur) | cours, stages, colles, examens | Que ses parents reçoivent tout ; bourse suspendue sans prévenir | Être le seul destinataire ; être prévenu et pouvoir justifier avant tout signalement |
| **Employeur, maître d'apprentissage** (sans compte ScolIA, D-020) | accueille, forme, évalue, signe | Recevoir des courriels illisibles ; ne pas savoir les dates | Un lien nominatif : calendrier, absences au centre, livret à remplir, visite à confirmer |
| **OPCO** (extérieur) | finance | — | Des pièces conformes |
| **CROUS** (extérieur) | verse les bourses | — | Une liste fiable, au bon moment |
| **Université conventionnée** (CPGE) | inscription parallèle, validation des crédits | — | La liste des inscrits et les attestations |

---

## 4. L'année [H, à confirmer]

- **Janvier–mai** : Parcoursup (formations du lycée comme formations d'accueil, BTS en apprentissage compris [NV]) ; recherche d'entreprise des futurs apprentis ; journées portes ouvertes ; candidatures directes pour l'UFA.
- **Juin–septembre** : admissions ; **contrats signés** (souvent entre juillet et novembre) ; calendrier d'alternance publié ; inscription parallèle des étudiants de CPGE (avant fin décembre [V-partiel]) ; dossiers de bourse (CROUS).
- **Septembre–novembre** : **périodes d'essai** des apprentis (45 jours de présence en entreprise) ; premières visites ; apprentis sans entreprise (délai pour en trouver un [NV]).
- **Toute l'année** : assiduité mensuelle transmise au CFA et à l'employeur ; colles hebdomadaires ; CCF ; stages de BTS ; contrôles d'assiduité des boursiers ; ruptures et nouveaux contrats.
- **Fin de semestre / d'année** : crédits européens ; attestations descriptives ; conseil de perfectionnement ; livret ; examens (38) ; taxe d'apprentissage (campagne SOLTéA au printemps [V]).

---

## 5. Ce qui est réglementaire (vérifié le 2026-11-11)

| # | Règle | Texte | Statut | Conséquence |
|---|---|---|---|---|
| RP-01 | Formation en centre **≥ 25 %** de la durée totale du contrat, sous réserve des règles du certificateur ; durée adaptée au niveau et aux accords de branche ; dérogation pour un nouveau contrat après rupture (L6222-18-2) | C. trav. L6211-2 | [V] | Calcul du taux de formation en centre **par contrat** (§13.4), alerte sous 25 % |
| RP-02 | Contrat de **6 mois à 3 ans** | C. trav. L6222-7-1 (cité par D6211-x) | [V-partiel] | Contrôle des dates |
| RP-03 | **Période d'essai** : rupture libre par l'une ou l'autre partie jusqu'à l'échéance des **45 premiers jours, consécutifs ou non, de formation pratique en entreprise** ; au-delà : accord écrit des deux parties, ou force majeure, faute grave, inaptitude, décès de l'employeur maître d'apprentissage d'une entreprise unipersonnelle (forme de licenciement) | C. trav. L6222-18 | [V] | Compteur des **jours en entreprise** (pas des jours calendaires) ; alerte avant le 45e |
| RP-04 | Rupture anticipée : **document écrit notifié au directeur du CFA** et à l'organisme chargé du dépôt | C. trav. R6222-21 | [V-partiel] | La rupture est un **fait reçu** et daté ; preuve de notification (7) |
| RP-05 | Dépôt du contrat par l'**OPCO** (le contrat lui est transmis) | C. trav. L6224-1 (numéro [NV]) ; DECA [NV] | [V-partiel] | État de dépôt suivi ; pas d'appel DECA sans spécification |
| RP-06 | Temps de formation en centre = **temps de travail** ; l'employeur est informé des absences | C. trav. L6222-24, L6223-4 (repris de RA-31 du 12) | [V] par le socle 12 | Information de l'employeur par une personne habilitée, sans motif médical |
| RP-07 | **14 missions du CFA** (accompagnement, aide à la recherche d'employeur, cohérence centre-entreprise, information des droits, suivi des ruptures, mixité, handicap, mobilité…) ; certaines confiables aux chambres consulaires | C. trav. L6231-2 | [V-partiel] | Le socle trace l'**accompagnement** (référent handicap, mobilité, ruptures) en M2, et en M1 ce que l'UFA fait pour le CFA |
| RP-08 | **Conseil de perfectionnement** de tout CFA, placé auprès du directeur ; examine le projet pédagogique, l'accueil, les relations avec les entreprises | C. trav. L6231-3, R6231-3 à R6231-5 (décret 2019-1143) | [V-partiel] | Instance (§16) |
| RP-09 | **Financement OPCO** au niveau de prise en charge, **au prorata des jours de formation** depuis le **1er juillet 2025** ; versements **40 % (dans les 30 jours de la facture), 30 %, 20 %, solde de 10 %** après facture finale et **certificat de réalisation**, dans les 4 mois après la fin du contrat ; rupture : **prorata temporis**, tout mois entamé dû ; maintien des versements après rupture jusqu'à un nouveau contrat ou 6 mois ; participation de l'employeur [chiffres à relire] | Décret 2025-585 du 27 juin 2025 ; C. trav. R6332-25 ; page travail-emploi « ce qui change au 1er juillet 2025 » | [V-partiel] | Échéancier OPCO **par contrat** (§15) ; **aucun montant codé** (barèmes versionnés, R-PBA30) |
| RP-10 | **Mineurs** : travaux réglementés possibles après **déclaration de dérogation** à l'inspection du travail **avant** l'affectation, par l'employeur **ou le chef d'établissement** (ateliers du lycée) ; conditions : évaluation des risques, information et formation à la sécurité, encadrement par une personne compétente, **avis médical d'aptitude** pour chaque jeune ; **valable 3 ans** ; transmission par tout moyen donnant date certaine ; suivi individuel renforcé de santé | C. trav. R4153-38 à R4153-52 (R4153-40, R4153-41, R4153-44) ; décret 2015-443 | [V] | Registre des déclarations de l'établissement (ateliers) avec échéance ; **avis d'aptitude** = fait daté sans contenu médical (§14.4) |
| RP-11 | **BTS** : **120 crédits européens** ; épreuves en **ponctuel** ou en **CCF** si l'établissement est **habilité** (un CFA hors EPLE, GRETA ou GIP-FCIP demande l'habilitation au recteur) ; la mise en œuvre de l'évaluation certificative relève du **certificateur** ; durée minimale en CFA **1 350 h (2 ans) ou 675 h (1 an)** | ESR, page BTS ; éduscol CCF ; vademecum du contrôle pédagogique de l'apprentissage (2025) citant D643-8 | [V-partiel] | Mode d'épreuve **par candidat** et par épreuve (CCF / ponctuel) ; heures en centre comptées (§13.4) |
| RP-12 | **CPGE** : formation supérieure de **deux ans** (rattachée au grade de licence), **60 crédits** la première année, **120** au total ; **convention** obligatoire du lycée public avec un ou plusieurs établissements publics d'enseignement supérieur ; **double inscription** (lycée d'abord, université avant fin décembre) ; **attestation descriptive** du parcours | C. éduc. L612-3 ; D612-19 à D612-29-2 (D612-25 pour l'attestation) ; circulaire de 2014 | [V-partiel] | Inscription parallèle suivie ; crédits par semestre ; attestation produite (§10) |
| RP-13 | **Bourses sur critères sociaux** : versement **suspendu** (ordre de reversement possible) si l'étudiant n'est pas inscrit, **n'assiste pas aux cours**, ne fait pas ses stages ou ne se présente pas aux examens ; contrôles **sous la responsabilité du chef d'établissement** ; l'établissement **transmet au CROUS** la liste (abandon, défaut d'assiduité…) ; suspension **à la date du fait**, mois entamé dû ; **procédure contradictoire** (CRPA L121-1) avant reversement ; **dispense d'assiduité** signalée = pas de reversement | Circulaires annuelles (BO ESR 2023-29, 2024-26 ; BO de février 2026) | [V-partiel] (circulaire 2026-2027 non relue) | Liste CROUS **préparée**, justificatif demandé avant, dispenses tracées, décision humaine (§11) ; repris de RA-29, AS-188, AS-204 du 12 (moins d'un mois : pas de suspension) |
| RP-14 | **CVEC** : **non assujettis** les inscrits en **lycée** (public ou privé sous contrat) pour un **BTS**, DMA, DN MADE, formation comptable ; étudiant de CPGE : assujetti par son inscription universitaire | Site CVEC et etudiant.gouv.fr ; décret 2018-564 ; C. éduc. L841-5 | [V] (montant annuel à relire chaque année) | Attestation CVEC demandée **seulement** en CPGE (pièce de l'inscription parallèle) |
| RP-15 | **Taxe d'apprentissage** : le **solde** est réparti par l'employeur sur **SOLTéA** vers des établissements **habilités** (L6241-5 : établissements délivrant des diplômes du RNCP, d'insertion, d'orientation), listes fixées par arrêtés ; **habilitation à redemander chaque année** (pour 2027 : dossier avant le **11 décembre 2026**, listes publiées avant la fin du 2e trimestre 2027) ; plafond de 30 % pour la catégorie du 13° | soltea.education.gouv.fr ; C. trav. L6241-5 | [V] | Suivi des versements annoncés et reçus, campagne auprès des entreprises partenaires (§17) ; **démarche annuelle** (soumis à Florian) |
| RP-16 | **Qualiopi** pour accéder aux fonds publics et mutualisés ; **déclaration d'activité** et **bilan pédagogique et financier** de l'organisme de formation | C. trav. L6316-1, L6352-11 | [NV] | M2 seulement (§16) |
| RP-17 | **Enquête SIFA** (effectifs d'apprentis) | FX-42 du 9 | [NV] | M2 ; en M1, le CFA la fait |
| RP-18 | **Publication d'indicateurs** (taux d'obtention, d'insertion, InserJeunes) | C. trav. L6111-8 | [NV] | Affichage des valeurs **publiées** (pas de calcul maison présenté comme officiel) |
| RP-19 | **Mixité des publics** (apprentis et scolaires dans une même section) | [NV] | [NV] | Possible dans le modèle (§7) |
| RP-20 | **Aides à l'embauche** d'apprentis : montants variables selon les décrets annuels (extraits contradictoires lus) | travail-emploi ; alternance.emploi.gouv.fr | [NV sur les montants] | **Aucun montant** dans ScolIA (hors périmètre : c'est l'employeur qui la demande) |

**Sources (consultées le 2026-11-11)** :
- L6211-2 : https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000037386056 ; livre II apprentissage : https://www.legifrance.gouv.fr/codes/id/LEGISCTA000006145421 ; D6222 : https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006072050/LEGISCTA000018497164/
- L6222-18 : https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000037386025 ; R6222-21 et s. : https://www.legifrance.gouv.fr/codes/id/LEGIARTI000041771333/
- L6231-2 : https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000037386088 ; L6231-3 : https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000037386084 ; R6231-3 : https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000039351864 ; décret 2019-1143 : https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000039349600
- Financement : décret 2025-585 https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000051808276 ; R6332-25 https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000042907445/2025-04-25 ; https://travail-emploi.gouv.fr/apprentissage-ce-qui-change-au-1er-juillet-2025 ; France compétences, rapport 2025 https://www.francecompetences.fr/app/uploads/2026/02/RUF25_Apprentissage.pdf
- Mineurs : R4153-38 à R4153-52 https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006072050/LEGISCTA000018488561/ ; R4153-40 https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000033769318 ; formulaire https://travail-emploi.gouv.fr/sites/travail-emploi/files/files-spip/pdf/declarationderogation.pdf ; https://travail-emploi.gouv.fr/la-protection-de-la-sante-des-jeunes-travailleurs
- BTS : https://www.enseignementsup-recherche.gouv.fr/fr/brevet-de-technicien-superieur-bts-45915 ; CCF https://eduscol.education.gouv.fr/5601/controle-en-cours-de-formation ; vademecum 2025 https://eduscol.education.gouv.fr/sites/default/files/document/vademecum-mission-de-controle-pedagogique-des-formations-par-apprentissage-2025-94149.pdf
- CPGE : L612-3 https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000042815027/ ; D612-19 et s. https://www.legifrance.gouv.fr/codes/section_lc/LEGITEXT000006071191/LEGISCTA000027864438/ ; inscription https://www.education.gouv.fr/bo/14/Hebdo37/MENS1416489D.htm ; conventions https://cache.media.enseignementsup-recherche.gouv.fr/file/2014/93/8/conventions_lycees_390938.pdf
- Bourses : https://www.education.gouv.fr/bo/2024/Hebdo26/ESRS2413977C ; https://www.enseignementsup-recherche.gouv.fr/fr/bo/2023/Hebdo29/ESRS2315208C
- CVEC : https://cvec.etudiant.gouv.fr/ ; https://www.etudiant.gouv.fr/en/do-you-have-pay-cvec-contribution-955 ; L841-5 https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000042919730 ; décret 2018-564 https://legifrance.gouv.fr/eli/decret/2018/6/30/ESRS1813895D/jo/texte
- Taxe d'apprentissage : https://www.soltea.education.gouv.fr/espace-public/aide/qui-sont-les-etablissements-beneficiaires-de-la-taxe-dapprentissage ; https://www.soltea.education.gouv.fr/espace-public/

**À vérifier avant les lots** (Q-PBA20 à Q-PBA27) : L6224-1 et DECA ; L6316-1 et L6352-11 ; SIFA ; L6111-8 ; mixité des publics ; circulaire des bourses 2026-2027 ; livret scolaire du BTS et son export ; délai pour trouver un employeur après l'entrée en formation ; formats Parcoursup « établissement d'accueil ».

---

## 6. Deux modèles d'apprentissage dans un lycée

| Sujet | **M1 — UFA rattachée à un CFA** | **M2 — ScolIA, logiciel du CFA** |
|---|---|---|
| Qui porte le contrat, l'OPCO, Qualiopi, BPF, SIFA | le CFA (son logiciel) | l'organisme de gestion, dans ScolIA |
| Ce que fait ScolIA | scolarité de l'apprenti (cours, appel, évaluations, livret), calendrier d'alternance, **copie de lecture** du contrat (dates, employeur, maître d'apprentissage, essai, rupture), **exports vers le CFA** (assiduité mensuelle, heures réalisées, évaluations), information de l'employeur si la convention CFA-UFA la confie à l'UFA | tout M1 + contrat source, dépôt OPCO, échéancier, factures, certificats de réalisation, accompagnement, conseil de perfectionnement, indicateurs, BPF, SIFA |
| Source de vérité du contrat | le CFA (ScolIA reçoit ou saisit une **copie** marquée « source : CFA ») | ScolIA |
| Unité ScolIA | unité `ufa` rattachée à un **CFA partenaire** (tiers) | unité `cfa` (organisme de formation de l'établissement) |
| Convention | convention CFA-UFA (pièce du 7) : qui informe l'employeur, qui émet quoi | — |

**Règle** (R-PBA1) : le modèle est porté **par l'unité** ; une même école peut avoir une UFA en M1 pour un BTS et être CFA en M2 pour un autre diplôme. Tous les objets de contrat portent leur **source** (`cfa_externe` ou `scolia`) ; une copie M1 n'est **jamais** modifiée à la main sans trace, elle se resynchronise.

---

## 7. Le statut d'apprenant, daté

### 7.1 Les statuts [R]
`scolaire` (lycéen ou élève de section post-bac sous statut scolaire, mineur) · `etudiant` (BTS, CPGE, DN MADE sous statut scolaire) · `apprenti` (contrat en cours) · `apprenti_sans_contrat` (en formation dans l'attente d'un employeur, ou après rupture, délai de recherche [NV]) · `stagiaire_formation_continue` · `autre`.

### 7.2 Une période de statut
Un statut est une **période** `[du, au)` sur une **scolarisation** (socle 2), avec sa **source** (inscription, contrat, rupture, décision) ; les périodes ne se chevauchent pas (contrainte d'exclusion) ; changer de statut **ferme** la période précédente à la date du fait et en **ouvre** une nouvelle. La colonne annuelle `statut_apprenant` devient une **projection** (statut au jour de rentrée, puis statut courant) pendant la transition, puis est retirée (lot de retrait).

### 7.3 Ce que le statut commande (règles déduites, jamais saisies)
| Domaine | `scolaire` mineur | `etudiant` | `apprenti` | `apprenti` mineur |
|---|---|---|---|---|
| Destinataires des avis (8) | titulaires | **l'étudiant seul** (R-AS34) sauf désignation | l'apprenti ; employeur pour l'assiduité ; CFA | l'apprenti, **ses titulaires** pour ce que la loi prévoit, employeur, CFA |
| Absence (12) | régime lycée | régime étudiant ; **liste CROUS** si boursier | **absence au travail** : information de l'employeur ; heures non réalisées (financement) | idem + titulaires |
| Facturation (24) | contribution des familles | contribution éventuelle | **aucune** facture à la famille pour la formation [NV : gratuité de la formation pour l'apprenti] ; OPCO (M2) | idem |
| Évaluations (21, 22, 38) | — | crédits, CCF | CCF si habilité ; livret d'apprentissage | idem |
| Calendrier (10) | classe | classe | **calendrier d'alternance** | idem |
| Restauration, internat (40, 41) | régime | régime | selon présence en centre (FAC-219 du 24) | idem |

---

## 8. Ce qui change dans les socles déjà conçus

| Socle | Ce qui change | Lot ici |
|---|---|---|
| **2 Élève** | statut daté (§7) ; INE des apprentis (famille d'INE « apprentissage », `02_ELEVE.md` l. 310) ; élève majeur courant ; **coordonnées propres** de l'étudiant (l. 84) | PBA1 |
| **3 Inscriptions** | Parcoursup comme source d'admission des BTS et CPGE (import des admis, Q-PBA22) ; candidature directe à l'UFA ; **admission sous condition de contrat** (apprenti) ; inscription parallèle en CPGE | PBA2, PBA4 |
| **4 Dossier administratif** | l'**étudiant majeur** devient le premier interlocuteur (déjà au 4) ; **employeur** et **maître d'apprentissage** comme tiers liés à l'apprenant (M-099 de `PERSONAS_02-04.md`) ; payeur = étudiant lui-même possible | PBA7 |
| **5 Structure** | unités `section_bts`, `cpge`, `ufa`, `cfa` (50 §6) ; **semestres** (périodisation) ; **sous-publics** d'une classe (apprentis et scolaires, CL-73 du 10, `05_STRUCTURE.md` l. 215) | PBA2 |
| **6 Personnels** | **colleurs** (vacataires, heures d'interrogation) ; **référent handicap** du CFA ; **référent mobilité** ; enseignants de BTS (pondération 1,25, R-02 du 10) ; **AED étudiante du même lycée** (P48 du 6, R-P5) ; formateurs de CFA | PBA3, PBA10 |
| **7 Documents** | **attestation d'assiduité** par période (l. 145), **certificat de réalisation** (l. 187), **contrat et avenant importés** (l. 189) ; **attestation descriptive** de CPGE ; relevé de crédits ; certificat de scolarité **étudiant** (CAF, CROUS) en libre-service ; **entités émettrices** (R-D13 : l'organisme de formation pour l'apprentissage) | PBA3, PBA7, PBA9 |
| **8 Notifications** | destinataires selon le statut (§7.3) ; **tiers sans compte** (employeur, maître d'apprentissage) par lien nominatif (D-027 G) | PBA6 |
| **9 Interopérabilité** | Parcoursup (accueil), DECA et API apprentissage [NV], SIFA (FX-42), Sirene (FX-56), échanges **CFA ↔ UFA** (fichiers d'assiduité et d'heures), CROUS (liste), université (inscriptions parallèles), SOLTéA (consultation) | PBA8, PBA12 |
| **10 Emploi du temps** | **calendrier d'alternance** par sous-public (CL-73) ; séances « en entreprise » sans appel ; colles (créneaux, salles, trinômes) ; pondération BTS | PBA3, PBA6 |
| **12 Assiduité** | profil post-bac (R-AS34) ; liste CROUS (AS-188, AS-204) ; information de l'employeur (AS-189, AS-242) ; **heures réalisées** (financement OPCO) ; **jours en entreprise** (période d'essai) | PBA5, PBA6 |
| **13 Discipline** | **apprenti = salarié** : une faute en centre peut intéresser l'employeur ; ScolIA **n'informe** l'employeur d'une sanction **que** si la convention CFA-UFA ou le règlement le prévoit et par décision de la direction [R, NV] | PBA6 |
| **14 Accueil, sûreté** | majeurs : sortie libre ; apprentis présents en ateliers : liste des présents en cas d'alerte | — |
| **16 Messagerie** | l'étudiant majeur est seul titulaire de son fil ; employeur sans compte par lien | PBA7 |
| **17, 18 Annonces, formulaires** | PD-07 et FD-04 (majeur ignoré) corrigés par la règle des destinataires ; signature du contrat par les **titulaires d'un apprenti mineur** | PBA7 |
| **19 Agenda** | visites en entreprise ; entretiens tripartites ; conseil de perfectionnement | PBA9 |
| **20 Cahier de textes** | travail à faire pendant la période en entreprise (« à faire en entreprise » : non, sauf livret) ; continuité pour l'apprenti absent pour son travail | PBA6 |
| **21 Évaluations** | **notes de colles** ; **CCF** (situations, grilles) ; moyenne par **semestre** ; apprentis dans une classe mixte (CE-20) | PBA3 |
| **22 Compétences** | **blocs de compétences** des référentiels de BTS (RNCP) [NV] ; **livret d'apprentissage** (compétences acquises en entreprise, saisies par le maître d'apprentissage) | PBA9 |
| **23 Bulletins, conseils** | conseil de classe post-bac ; **attribution des crédits** ; attestation descriptive ; LSL BTS [NV] | PBA3, PBA4 |
| **24, 25 Finances** | apprentis : pas de contribution de la formation ; **factures OPCO** (M2) ; restauration et internat des apprentis selon leur présence (FAC-219) ; encaissements OPCO rapprochés par contrat | PBA11 |
| **26 Bourses** | bourses **CROUS** (enseignement supérieur), distinctes des bourses de lycée ; aucun calcul de bourse par ScolIA (le CROUS décide) | PBA5 |
| **34 Santé** | **avis d'aptitude** du médecin du travail (mineur, travaux réglementés) = fait daté sans diagnostic | PBA10 |
| **37 Orientation** | Parcoursup côté **accueil** (candidatures reçues) vs côté **lycée d'origine** (fiche Avenir) | PBA2 |
| **38 Examens** | mode **CCF / ponctuel** par candidat et par épreuve ; habilitation au CCF | PBA3 |
| **39 Stages** | **stages de BTS** (durées par référentiel) ; **entreprises** (`ent_entreprise`) et **tuteurs** réutilisés pour les employeurs et maîtres d'apprentissage ; le contrat d'apprentissage **n'est pas** une convention de stage | PBA7 |
| **41 Internat** | internat des CPGE (dont week-ends) et des apprentis aux semaines de centre | — |
| **47 Pilotage** | effectifs par statut ; ruptures ; taux de présence ; indicateurs **publiés** (InserJeunes) affichés | PBA13 |
| **48 Comptabilité** | produits OPCO ; taxe d'apprentissage ; analytique par formation | PBA11, PBA12 |

---

## 9. BTS

- **Unité** `section_bts` (UAI de rattachement, souvent celui du lycée) ; **classes** de 1re et 2e année ; **sous-publics** scolaires et apprentis.
- **Périodisation par semestre** (S1–S4) [R, organisation fréquente, texte non relu] ; **crédits européens** (120) attribués par **unité d'enseignement** ou par semestre selon le référentiel, validés au conseil de classe de fin de semestre ; **relevé de crédits** remis à l'étudiant (document du 7).
- **Stages** (39) : durées par référentiel, convention, tuteur ; pour l'apprenti, la période en entreprise **remplace** le stage [H].
- **CCF** : **situations d'évaluation** planifiées par épreuve (date, sujet, jury), grilles d'évaluation, **notes non communiquées** avant le jury de délibération quand le règlement le prévoit [NV : confidentialité des notes de CCF], **procès-verbaux** ; **mode par candidat** (CCF si l'unité est habilitée et le candidat éligible ; sinon ponctuel) ; transmission au **certificateur** (38).
- **Parcoursup** côté accueil : liste des admis importée (format [NV]) ; vœux en apprentissage.
- **Livret scolaire du BTS** : export seulement avec convention éditeur (D-032).

---

## 10. CPGE

- **Unité** `cpge` ; classes de 1re et 2e année (MPSI, PCSI, ECG…), éventuellement « 5/2 » (redoublants de 2e année).
- **Convention avec l'université** (pièce du 7, dates) ; **inscription parallèle** de chaque étudiant (date, établissement, numéro étudiant **facultatif** [R], attestation CVEC jointe) ; alerte « non inscrit au 31 décembre » (RP-12).
- **Crédits** : 60 par année, attribués par le conseil de classe sur proposition des enseignants ; **attestation descriptive** du parcours (D612-25) en fin d'année ou au départ.
- **Colles** : **colloscope** (groupes de 2 ou 3 étudiants, rotation hebdomadaire par matière, colleur, salle, créneau) **généré** à partir des groupes et des disponibilités des colleurs (moteur du 10 ou algorithme simple de rotation), publié aux étudiants ; **notes de colle** saisies par le colleur sur mobile (socle 21 : évaluation de type « interrogation orale ») ; **heures de colle** comptées par colleur (paie, 49 ; heures supplémentaires) ; colleur absent : remplacement ou report.
- **Concours** : inscriptions par le service national (SCEI [NV]) hors ScolIA ; ScolIA garde seulement les **résultats déclarés** par l'étudiant (facultatif) pour le pilotage.
- **Internat** et **bourses** (41, §11).

---

## 11. Bourses et assiduité étudiante (RP-13)

1. **Profil d'assiduité étudiant** (socle 12) : mêmes appels ; **seul destinataire** l'étudiant (R-AS34).
2. **Signalement d'un défaut d'assiduité** : la vie scolaire voit les étudiants boursiers dont l'assiduité passe sous un seuil ([R] défini par l'établissement : S-5100) ; **aucun signalement** pour une absence de moins d'un mois (RA-29 du 12).
3. **Procédure contradictoire** : l'étudiant reçoit une **demande de justificatif** datée (AS-204) avec un délai (S-5101 : 8 jours [R]) ; ses réponses et pièces sont tracées ; une **dispense d'assiduité** (stage à l'étranger, situation particulière) est enregistrée et **exclut** le signalement.
4. **Liste CROUS** : préparée par la vie scolaire, **validée et signée** par la direction (R-PBA20), transmise par la direction (hors ScolIA ou par l'échange déclaré du 9) ; la date du fait (première absence non justifiée retenue) figure sur la liste.
5. **Abandon, réorientation, passage en alternance** : signalés au CROUS (la circulaire de 2026 les liste [V-partiel]) ; le passage en **apprentissage** d'un boursier met fin à la bourse [NV] : alerte au changement de statut.
6. **Traçabilité** : chaque liste est figée (instantané du 7), avec ses justifications.

---

## 12. Le contrat d'apprentissage

### 12.1 Objet [R]
Un **contrat** relie un **apprenant**, un **employeur** (entreprise du 39, établissement d'exécution avec SIRET), un ou des **maîtres d'apprentissage** (personnes du 39, sans compte), une **formation** (diplôme visé, RNCP, unité), un **organisme de formation** (CFA, UFA), des **dates** (début, fin, début de formation en centre), un **numéro de dépôt** (OPCO), un **OPCO** ; il a des **avenants** (changement de maître, de durée, d'employeur dans un même groupe) et peut être **rompu**.
- **Source** : `cfa_externe` (M1, copie de lecture, resynchronisée) ou `scolia` (M2).
- **Pièce** : le CERFA signé **importé** (7, DCT-138) ; ScolIA ne génère pas le formulaire officiel en M1 [R] ; en M2, il pré-remplit le formulaire depuis ses données (format [NV]).
- **Mineur** : signature des **titulaires** (socle 18 : formulaire à deux signatures si les deux sont connus) [NV sur l'exigence exacte de signature].

### 12.2 Cycle de vie
`projet` → `signe` → `transmis_opco` → `depose` (numéro reçu) → `en_cours` → (`avenant`)* → `rompu_essai` | `rompu` | `termine` ; `refuse_depot` possible. Chaque passage est un **fait daté** (journal en ajout seul) avec sa pièce.

### 12.3 Période d'essai (RP-03)
- Compteur des **jours de formation pratique en entreprise** (jours calendaires du calendrier d'alternance où l'apprenti est attendu en entreprise, moins les absences en entreprise déclarées par l'employeur [R]) ; **alerte** au coordinateur à J-10 et à l'échéance ; affichage « période d'essai jusqu'au [date estimée] ».
- Une rupture **pendant** l'essai est enregistrée comme `rompu_essai` (pas de motif exigé).

### 12.4 Rupture (RP-03, RP-04)
- Saisie par le coordinateur à réception de l'écrit (pièce) ; **date d'effet** ; **motif** dans la liste légale (accord, force majeure, faute grave, inaptitude, décès, exclusion du CFA [NV], démission de l'apprenti via médiateur [NV]) ;
- **Effets** (par événement) : statut → `apprenti_sans_contrat` ou sortie (décision humaine) ; information du CFA (M1 : export ; M2 : interne) ; **arrêt** des avis à l'employeur ; recalcul **prorata** du financement (M2, §15) ; tâche « accompagnement de l'apprenti en rupture » (L6231-2, mission de suivi [V-partiel]) ; **aucune** information de la famille d'un majeur.

---

## 13. Alternance et assiduité de l'apprenti

### 13.1 Calendrier d'alternance [R]
- Un **calendrier** par **sous-public** de classe (apprentis de BTS 1re année) : semaines ou jours **en centre** / **en entreprise** ; rythme régulier (2 semaines / 2 semaines, 1 jour / 4 jours) ou irrégulier ; **exceptions individuelles** (un apprenti dont l'entreprise a un autre rythme) ; publié à l'apprenti, à l'employeur (lien), aux enseignants.
- **Emploi du temps** (10) : les séances de la classe n'attendent l'apprenti que les jours « en centre ».

### 13.2 Appel et absences
- L'appel ne liste l'apprenti que s'il est **attendu en centre** ce jour-là ; sinon, « en entreprise » (ni présent ni absent).
- **Absence en centre** = absence au travail (RP-06) : l'**employeur** est informé (R-PBA12), **sans motif médical**, par une personne habilitée ou par l'avis automatique **si** la convention et le réglage le prévoient (S-5103 : défaut « récapitulatif hebdomadaire validé par le coordinateur ») ; l'apprenti voit ce qui part.
- **Justification** : l'employeur peut **attester** (« présent en entreprise ce jour, à notre demande ») : une absence en centre **autorisée par l'employeur** reste une absence au centre (heures non réalisées) mais pas une faute [R, NV].

### 13.3 Heures réalisées
- Chaque séance **attendue** et **réalisée** (présence) compte en **heures de formation réalisées** ; les absences (justifiées ou non) sont des heures **non réalisées** ; le cumul par contrat alimente l'export CFA (M1) et les certificats de réalisation (M2).
- **Formation à distance** (heures en ligne) : déclarée par l'enseignant avec preuve de réalisation [NV : exigences de traçabilité des OPCO].

### 13.4 Taux de formation en centre (RP-01, RP-11)
Pour chaque contrat : heures **prévues** en centre (calendrier) / durée totale du contrat ; **alerte** si < 25 % ou sous le minimum du diplôme (1 350 h pour un BTS en 2 ans) ; **heures réalisées** suivies.

---

## 14. Entreprise, maître d'apprentissage, suivi

### 14.1 Employeur et maître d'apprentissage
- **Employeur** = `ent_entreprise` du 39 (raison sociale, SIRET avec clé de contrôle Luhn [R], adresse, OPCO, convention collective [NV, facultatif]) ; **établissement d'exécution** (SIRET de l'établissement) ; **contacts** (RH, signataire).
- **Maître d'apprentissage** : personne **sans compte** (D-020) ; accès par **lien nominatif** (D-027 G) : calendrier de l'apprenti, absences au centre, **livret** à compléter, **visites** à confirmer ; jamais les notes de l'apprenti hors de ce que le livret prévoit, jamais ses autres données.

### 14.2 Livret d'apprentissage et suivi
- **Livret** : compétences du référentiel (22) observées **en entreprise**, appréciations du maître d'apprentissage, du formateur, de l'apprenti ; **entretiens tripartites** (dates, compte rendu) ; signé par les trois.
- **Visites** en entreprise (19) : planifiées par le formateur référent, compte rendu, alertes (difficulté, risque de rupture).
- **Accompagnement** (M2, RP-07) : référent handicap, aides à la mobilité ou au permis [NV], suivi des ruptures, médiation (coordonnées du médiateur consulaire [NV]).

### 14.3 Apprentis mineurs : travaux réglementés (RP-10)
- **Registre des déclarations de dérogation** de l'établissement pour **ses ateliers** (le chef d'établissement déclare) : date de déclaration, travaux concernés, lieu, échéance **à 3 ans**, preuve de date certaine (pièce) ; alerte avant échéance.
- Pour **chaque jeune** affecté : **avis médical d'aptitude** (fait daté, **sans contenu médical** : « apte aux travaux réglementés de l'atelier X jusqu'au … »), **information et formation à la sécurité** (date, formateur), **encadrant compétent** désigné ; **blocage** : un mineur sans avis valide ni formation **n'est pas inscrit** à une séance d'atelier à travaux réglementés (R-PBA25).
- Côté **employeur** : déclaration de l'employeur **hors** ScolIA ; ScolIA peut recueillir l'attestation de l'employeur (pièce) si le CFA l'exige [R].

---

## 15. Financement par les OPCO (M2 ; en M1, le CFA le fait)

- **Niveaux de prise en charge** (NPEC) **versionnés** par diplôme et branche (table de barème, **aucune valeur codée**, R-PBA30), saisis ou importés depuis la publication officielle [NV : format].
- **Prise en charge** par contrat : montant annuel, **prorata des jours** (décret 2025-585), échéancier **40 / 30 / 20 / 10 %** (RP-09), dates d'exigibilité.
- **Factures à l'OPCO** (socle 24, entité émettrice « organisme de formation ») : générées depuis l'échéancier ; **facture finale** accompagnée du **certificat de réalisation** (heures réalisées, §13.3, document du 7 signé par le directeur du CFA) ; **rupture** : recalcul au prorata, mois entamé dû (R6332-25) ; **maintien** après rupture jusqu'à un nouveau contrat ou 6 mois [V-partiel] ;
- **Encaissements** (25) rapprochés par contrat ; **relances** ; **rejets** de l'OPCO (motif, pièce manquante) en file de travail.
- **Participation de l'employeur** en cas de rupture pendant l'essai ou de nouveau contrat [V-partiel : chiffres à relire] : calculée par barème versionné, facturée à l'employeur sur décision.

---

## 16. Obligations du CFA, qualité, indicateurs (M2 ; partiellement M1)

- **Conseil de perfectionnement** (RP-08) : instance (composition, réunions, ordre du jour, PV) ; en M1, l'UFA y participe (personnes sans compte du CFA).
- **Qualiopi** [NV] : ScolIA **trace les preuves** utiles aux indicateurs (satisfaction, réclamations, accompagnement, information du public) sans prétendre certifier ; **réclamations** (registre), **enquêtes de satisfaction** (socle 18, anonymes).
- **BPF** et **SIFA** [NV] : **exports de travail** (effectifs, heures, produits) préparés, vérifiés et déclarés par une personne (aucun envoi automatique).
- **Indicateurs publiés** (InserJeunes) [NV] : affichés tels que publiés, avec leur source et leur date ; **aucun** taux maison présenté comme officiel (R-PBA34).

---

## 17. Taxe d'apprentissage (RP-15)

- **Habilitation** : rappel annuel de la campagne SOLTéA (dépôt avant le 11 décembre pour l'année suivante [V]) ; pièce de l'arrêté d'habilitation ; **démarche faite par l'établissement** (soumis à Florian).
- **Suivi** : versements **annoncés** par les entreprises (déclaratif), **reçus** (relevé de la Caisse des dépôts importé ou saisi), affectation analytique (48) ; **remerciements** aux entreprises (17, lettre type D-023).
- **Campagne** auprès des entreprises partenaires (employeurs d'apprentis, lieux de stage) : liste **minimale** (raison sociale, contact professionnel), message, **jamais** de donnée d'apprenant dans la campagne (R-PBA38).

---

## 18. Cas limites (catalogue)

| # | Cas | Réponse attendue |
|---|---|---|
| CPB-01 | Lycéen de bac pro qui signe un contrat le 15 novembre | période `scolaire` close le 14, `apprenti` ouverte le 15 ; absences d'octobre restent au régime scolaire (PBD-01) |
| CPB-02 | Apprenti dont le contrat est **rompu** en février, reste en formation | `apprenti_sans_contrat` ; avis à l'employeur arrêtés ; accompagnement ; nouveau contrat possible avec dérogation de durée (L6211-2) |
| CPB-03 | Rupture pendant la période d'essai au 30e jour en entreprise | `rompu_essai` ; pas de motif |
| CPB-04 | Apprenti absent en entreprise une semaine (maladie) : compte pour l'essai ? | jours **non** en entreprise ne comptent pas (« consécutifs ou non », RP-03) : l'employeur le déclare ; date estimée recalculée |
| CPB-05 | Apprenti **mineur** qui devient majeur pendant le contrat | les titulaires ne reçoivent plus les avis à partir de ses 18 ans (sauf désignation par lui) ; historique conservé |
| CPB-06 | Apprenti de **16 ans** en atelier de soudage | blocage sans avis d'aptitude et formation sécurité (R-PBA25) |
| CPB-07 | Déclaration de dérogation de l'atelier **expirée** | tous les mineurs bloqués sur les séances concernées ; alerte 60 jours avant (S-5110) |
| CPB-08 | Employeur qui demande les **notes** de l'apprenti | seulement ce que prévoit le livret ; le reste sur décision de l'apprenti majeur (partage explicite) |
| CPB-09 | Employeur qui demande le **motif** d'une absence | jamais un motif médical (RP-06) ; « absence justifiée » ou « non justifiée » |
| CPB-10 | Apprenti qui change d'**employeur** (rupture + nouveau contrat) | deux contrats successifs ; calendrier possiblement différent ; financement au prorata de chacun (M2) |
| CPB-11 | Deux employeurs (saisonnier, multi-employeurs) [NV] | contrats parallèles refusés par défaut ; exception à étudier (Q-PBA9) |
| CPB-12 | Classe **mixte** : 14 scolaires, 10 apprentis, rythmes différents | sous-publics ; appel selon le calendrier de chacun |
| CPB-13 | Apprenti attendu en centre mais **retenu par l'employeur** | absence au centre « à la demande de l'employeur » (attestation) ; heures non réalisées ; pas de faute (§13.2) |
| CPB-14 | Apprenti en centre un jour **férié** travaillé dans l'entreprise | jour férié = pas de séance (5) ; aucun appel |
| CPB-15 | Contrat avec **moins de 25 %** de formation en centre | alerte bloquante à la validation du calendrier (M2) ; information (M1) |
| CPB-16 | BTS apprenti avec 1 200 h prévues sur 2 ans | alerte « sous le minimum du diplôme » (1 350 h) |
| CPB-17 | Étudiant boursier absent **3 semaines** (hospitalisation) | moins d'un mois : aucun signalement (RA-29) ; justificatif facultatif |
| CPB-18 | Étudiant boursier qui ne répond pas à la demande de justificatif | après le délai, proposé sur la liste ; décision de la direction |
| CPB-19 | Étudiant avec **dispense d'assiduité** (stage à l'étranger) | exclu des listes |
| CPB-20 | Boursier qui passe en **apprentissage** | alerte « informer le CROUS » (§11.5) |
| CPB-21 | Étudiant de CPGE **non inscrit** à l'université au 31 décembre | alerte à la direction et à l'étudiant |
| CPB-22 | Étudiant de BTS à qui l'on réclame une attestation CVEC | non : BTS en lycée non assujetti (RP-14) |
| CPB-23 | Étudiant de CPGE qui « cube » (3e année, « 5/2 ») | nouvelle année en 2e année ; crédits déjà acquis conservés |
| CPB-24 | Étudiant qui quitte la CPGE en février | attestation descriptive des crédits acquis (S1) |
| CPB-25 | Colleur absent | remplacement ou report ; trinôme prévenu |
| CPB-26 | Colleur vacataire **extérieur** | personnel (6) à engagement court ; voit ses trinômes du jour seulement |
| CPB-27 | Note de colle contestée | historique de la note (21) |
| CPB-28 | AED qui est aussi étudiante de BTS du lycée | portée vie scolaire **sans** sa classe (R-P5 du 6) |
| CPB-29 | Étudiant majeur qui **désigne** ses parents comme destinataires des absences | désignation datée, révocable (4) |
| CPB-30 | Parents d'un étudiant majeur qui appellent pour ses notes | refus poli ; renvoi vers l'étudiant (R-AS34) |
| CPB-31 | Étudiant **mineur** en BTS (17 ans, bac précoce) | statut `etudiant` mais mineur : titulaires destinataires jusqu'à 18 ans |
| CPB-32 | CCF : candidat **non éligible** au CCF (UFA non habilitée) | mode ponctuel ; le planning de CCF ne le convoque pas |
| CPB-33 | CCF : étudiant absent à une situation d'évaluation | absence justifiée : situation de remplacement ; non justifiée : selon le règlement d'examen [NV] |
| CPB-34 | Notes de CCF demandées par l'étudiant avant le jury | non communiquées si le règlement le prévoit (S-5106) |
| CPB-35 | Crédits d'un semestre non validés | relevé partiel ; compensation selon le règlement [NV] |
| CPB-36 | Stage de BTS d'un apprenti | sans objet : la période en entreprise en tient lieu [H] |
| CPB-37 | Employeur dont l'entreprise est **radiée** (Sirene) | alerte au coordinateur ; aucun effet automatique |
| CPB-38 | SIRET erroné (clé invalide) | refus à la saisie |
| CPB-39 | Maître d'apprentissage qui **change** | avenant ; ancien lien révoqué |
| CPB-40 | Lien du maître d'apprentissage transféré à un collègue | lien nominatif, révocable, durée limitée, journal des accès |
| CPB-41 | OPCO qui **rejette** une facture (certificat manquant) | file de travail ; pièce ajoutée ; nouvelle facture (pas de modification de l'émise, D-027 E) |
| CPB-42 | Rupture au 15 du mois : financement | prorata, mois entamé dû (R6332-25) |
| CPB-43 | NPEC modifié en cours d'année | version datée ; contrats signés avant gardent leur version [NV : règle de l'OPCO] |
| CPB-44 | CFA externe (M1) qui envoie un fichier d'assiduité **contradictoire** | ScolIA est la source de l'assiduité en centre ; l'écart est signalé, pas écrasé |
| CPB-45 | CFA externe qui envoie un contrat **modifié** | resynchronisation ; avenant reconstitué ; trace |
| CPB-46 | UFA en M1 pour un BTS et CFA en M2 pour un CAP | modèle par unité (R-PBA1) |
| CPB-47 | Apprenti **handicapé** | référent handicap (M2) ; aménagements (35) ; durée de contrat adaptée [NV] |
| CPB-48 | Apprenti qui demande **toutes** ses données (RGPD) | export de son dossier (27), contrat compris |
| CPB-49 | Ancien apprenti, 5 ans après : attestation de formation | archive selon la durée du DPO ; attestation rééditée depuis l'archive |
| CPB-50 | Contrôle de l'OPCO 3 ans après | pièces conservées (Q-PBA14, durée à fixer avec le DPO) |
| CPB-51 | Entreprise partenaire qui demande la liste des **apprentis des autres entreprises** | refus |
| CPB-52 | Taxe d'apprentissage : entreprise qui annonce un versement non reçu | état « annoncé », relance, jamais de comptabilisation |
| CPB-53 | Habilitation SOLTéA non renouvelée | alerte en octobre (S-5112) |
| CPB-54 | Apprenti interne (41) aux semaines de centre seulement | réservation selon le calendrier |
| CPB-55 | Restauration de l'apprenti | selon les jours en centre (FAC-219 du 24) |
| CPB-56 | Apprenti exclu temporairement du centre (13) | l'employeur est informé de l'absence (pas forcément du motif, S-5104) |
| CPB-57 | Étudiant de BTS salarié à côté (job) qui demande un aménagement | régime d'étudiant salarié [NV] ; décision de la direction tracée |
| CPB-58 | Apprenti dont l'employeur est **le lycée lui-même** (OGEC) | deux casquettes : apprenant et personnel (6, `352`), portées séparées |
| CPB-59 | Apprenti de **30 ans** (exception de l'âge, handicap, création d'entreprise) [NV] | contrôle d'âge avec exception motivée |
| CPB-60 | Parcoursup : admis qui ne confirme pas | pas d'inscription créée (3) |
| CPB-61 | Étudiant étranger hors UE | titre de séjour : pièce facultative, accès restreint (secrétariat) [R] |
| CPB-62 | Fermeture exceptionnelle du centre (neige) | séances annulées ; heures non réalisées **non imputables** à l'apprenti ; information de l'employeur (l'apprenti doit-il aller en entreprise ? [NV]) |

---

## 19. Intelligence artificielle

**Fait** [R] : prépare la **liste CROUS** avec les justifications manquantes ; résume un **contrat** importé (dates, essai, échéances) ; rédige la **demande de justificatif**, le **compte rendu de visite** à partir des notes, le **message à l'employeur** (relu, envoyé par une personne) ; génère une **proposition de colloscope** ; explique un **écart** CFA ↔ ScolIA ; prépare les **exports** BPF / SIFA (M2) avec les contrôles.
**Ne fait pas** : aucune décision sur une **bourse**, un **signalement**, une **rupture**, une **habilitation** ; aucune **évaluation** (crédits, CCF, notes de colle : règlement IA, annexe III point 3) ; aucun accès aux **avis d'aptitude** (même sans contenu médical, R-PBA26) ; aucun envoi à un tiers.
**Capacités** : `postbac.preparer_liste_crous` (N1), `apprentissage.resumer_contrat` (N0), `apprentissage.rediger_message_employeur` (N2 avant envoi), `cpge.proposer_colloscope` (N1), `apprentissage.expliquer_ecart_cfa` (N0).

---

## 20. Actions métier (PBA-100 à PBA-299)

Acteurs : COO coordinateur (DDFPT, coordinateur de BTS ou d'UFA), DIR direction, VS vie scolaire, SEC secrétariat, ENS enseignant ou formateur, COL colleur, GES gestionnaire du CFA (M2), CPT comptabilité, APP apprenant (étudiant ou apprenti), TIT titulaire d'un mineur, EMP employeur ou maître d'apprentissage (sans compte), SYS.

### 20.1 Statut et unités
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PBA-100 | Créer une unité post-bac (BTS, CPGE, UFA, CFA) et choisir M1/M2 | DIR · `structure.unites` | R-PBA1 | `structure.unite_creee` | PBA2 |
| PBA-101 | Déclarer le CFA partenaire d'une UFA (M1) et la convention | DIR | pièce (7) | — | PBA2 |
| PBA-102 | Ouvrir une période de statut (inscription) | SEC, SYS | pas de chevauchement | `eleve.statut_ouvert` | PBA1 |
| PBA-103 | Changer de statut à une date (contrat signé, rupture) | COO, SYS | ferme l'ancienne période | `eleve.statut_change` | PBA1 |
| PBA-104 | Voir l'historique des statuts | COO, VS, APP | — | — | PBA1 |
| PBA-105 | Reprendre `statut_apprenant` en périodes (proposition) | DIR | « autre » jamais converti seul | — | PBA1 |
| PBA-106 | Créer les sous-publics d'une classe (apprentis, scolaires) | COO | 5 | `structure.groupe_cree` | PBA2 |
| PBA-107 | Choisir la périodisation par semestres | DIR | 5 | — | PBA2 |
| PBA-108 | Voir les effectifs par statut et par unité | DIR, COO | — | — | PBA2 |

### 20.2 BTS : crédits, CCF, stages
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PBA-110 | Déclarer les unités d'enseignement et leurs crédits (référentiel) | COO | total 120 | — | PBA3 |
| PBA-111 | Attribuer les crédits d'un semestre en conseil | DIR | décision humaine | `postbac.credits_attribues` | PBA3 |
| PBA-112 | Éditer le relevé de crédits | SEC | 7 | `documents.genere` | PBA3 |
| PBA-113 | Déclarer l'habilitation au CCF de l'unité (dates, pièce) | DIR | RP-11 | — | PBA3 |
| PBA-114 | Fixer le mode d'épreuve par candidat (CCF / ponctuel) | COO | éligibilité | `examens.mode_fixe` | PBA3 |
| PBA-115 | Planifier une situation d'évaluation de CCF (date, jury, sujet) | ENS, COO | 38 | `examens.ccf_planifie` | PBA3 |
| PBA-116 | Saisir l'évaluation de CCF (grille) | ENS | confidentialité (S-5106) | `examens.ccf_evalue` | PBA3 |
| PBA-117 | Organiser une situation de remplacement (absence justifiée) | COO | — | — | PBA3 |
| PBA-118 | Arrêter le procès-verbal de CCF et le transmettre au certificateur | DIR | — | `examens.ccf_transmis` | PBA3 |
| PBA-119 | Planifier les stages de BTS (durées du référentiel) | COO | 39 | `stages.periode` | PBA3 |
| PBA-120 | Importer les admis Parcoursup | SEC | format [NV] | `inscriptions.import` | PBA4 |
| PBA-121 | Contrôler la pondération horaire des services de BTS | DIR | 1,25 (10) | — | PBA3 |

### 20.3 CPGE
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PBA-130 | Enregistrer la convention avec l'université | DIR | pièce | — | PBA4 |
| PBA-131 | Enregistrer l'inscription parallèle d'un étudiant (+ attestation CVEC) | APP, SEC | avant fin décembre | `postbac.inscription_parallele` | PBA4 |
| PBA-132 | Alerte « non inscrit à l'université » | SYS → SEC, APP | RP-12 | — | PBA4 |
| PBA-133 | Transmettre la liste des inscrits à l'université | SEC | échange déclaré | `interop.envoi` | PBA4 |
| PBA-134 | Constituer les trinômes de colle | ENS | — | — | PBA4 |
| PBA-135 | Générer le colloscope (rotation, colleurs, salles) | COO | proposition (IA ou algorithme) | `cpge.colloscope_propose` | PBA4 |
| PBA-136 | Publier le colloscope | COO | — | `cpge.colloscope_publie` | PBA4 |
| PBA-137 | Voir ses colles du jour (mobile) | COL | trinômes du jour | — | PBA4 |
| PBA-138 | Saisir la note et l'appréciation de colle | COL | 21 | `evaluations.resultat` | PBA4 |
| PBA-139 | Déclarer une absence de colleur, un report | COL, COO | — | `cpge.colle_reportee` | PBA4 |
| PBA-140 | Compter les heures de colle par colleur (mois) | COO, CPT | paie (49) | — | PBA4 |
| PBA-141 | Attribuer les crédits annuels (60) | DIR | — | `postbac.credits_attribues` | PBA4 |
| PBA-142 | Éditer l'attestation descriptive du parcours | SEC | D612-25 | `documents.genere` | PBA4 |
| PBA-143 | Déclarer un résultat de concours (facultatif) | APP | — | — | PBA4 |

### 20.4 Assiduité étudiante et bourses
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PBA-150 | Appliquer le profil d'assiduité étudiant | SYS | R-AS34 | — | PBA5 |
| PBA-151 | Déclarer un étudiant boursier (échelon facultatif) | SEC, APP | minimisation | — | PBA5 |
| PBA-152 | Voir les boursiers sous le seuil d'assiduité | VS | S-5100 ; < 1 mois exclu | — | PBA5 |
| PBA-153 | Demander un justificatif (délai) | VS | contradictoire | `assiduite.justificatif_demande` | PBA5 |
| PBA-154 | Répondre, déposer une pièce | APP | — | `assiduite.justificatif_depose` | PBA5 |
| PBA-155 | Enregistrer une dispense d'assiduité | DIR | exclut les listes | `postbac.dispense` | PBA5 |
| PBA-156 | Préparer la liste CROUS (IA possible) | VS | justifications jointes | — | PBA5 |
| PBA-157 | Valider et signer la liste CROUS | DIR | R-PBA20 | `postbac.liste_crous_signee` | PBA5 |
| PBA-158 | Enregistrer la transmission au CROUS | DIR | — | `postbac.liste_crous_transmise` | PBA5 |
| PBA-159 | Alerte « boursier passé en apprentissage / abandon » | SYS → VS | §11.5 | — | PBA5 |
| PBA-160 | Désigner des destinataires (étudiant majeur) | APP | révocable | `personnes.designation` | PBA5 |
| PBA-161 | Attestations en libre-service (scolarité, assiduité) | APP | 7 | `documents.genere` | PBA5 |

### 20.5 Alternance et assiduité de l'apprenti
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PBA-170 | Créer le calendrier d'alternance d'un sous-public | COO | — | `apprentissage.calendrier_cree` | PBA6 |
| PBA-171 | Ajouter une exception individuelle | COO | — | — | PBA6 |
| PBA-172 | Publier le calendrier (apprenti, employeur, enseignants) | COO | lien employeur | `apprentissage.calendrier_publie` | PBA6 |
| PBA-173 | Contrôler 25 % et minimum du diplôme | SYS | RP-01, RP-11 | — | PBA6 |
| PBA-174 | Appel : n'attendre que les apprentis « en centre » | SYS | 12 | — | PBA6 |
| PBA-175 | Informer l'employeur des absences (récapitulatif validé) | COO, SYS | sans motif médical ; S-5103 | `apprentissage.employeur_informe` | PBA6 |
| PBA-176 | Recevoir l'attestation de l'employeur (absence à sa demande) | EMP | lien | `apprentissage.attestation_employeur` | PBA6 |
| PBA-177 | Voir ce qui a été transmis à son employeur | APP | transparence | — | PBA6 |
| PBA-178 | Compter les heures réalisées par contrat | SYS | §13.3 | — | PBA6 |
| PBA-179 | Déclarer des heures de formation à distance | ENS | preuve | — | PBA6 |
| PBA-180 | Informer les titulaires d'un apprenti mineur | SYS | §7.3 | `notifications.avis` | PBA6 |
| PBA-181 | Informer l'employeur d'une exclusion temporaire (absence) | DIR | S-5104 | — | PBA6 |
| PBA-182 | Fermeture du centre : heures non imputables, information employeur | SYS | CPB-62 | — | PBA6 |

### 20.6 Contrats (M1 lecture, M2 source)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PBA-190 | Enregistrer un contrat (source CFA ou ScolIA) | COO, GES · `apprentissage.contrats` | dates 6 mois–3 ans | `apprentissage.contrat_enregistre` | PBA7 |
| PBA-191 | Importer le CERFA signé | COO | 7 | `documents.depose` | PBA7 |
| PBA-192 | Recueillir la signature des titulaires d'un mineur | TIT | 18 | `formulaires.signe` | PBA7 |
| PBA-193 | Suivre le dépôt (transmis, déposé, refusé, numéro) | GES, COO | — | `apprentissage.contrat_depose` | PBA7 |
| PBA-194 | Enregistrer un avenant | COO, GES | — | `apprentissage.avenant` | PBA7 |
| PBA-195 | Suivre la période d'essai (jours en entreprise) | SYS → COO | alerte J-10 | — | PBA7 |
| PBA-196 | Enregistrer une rupture (date, motif légal, pièce) | COO | RP-03, RP-04 | `apprentissage.contrat_rompu` | PBA7 |
| PBA-197 | Accompagner l'apprenti en rupture (tâche, suivi) | COO | L6231-2 | — | PBA7 |
| PBA-198 | Resynchroniser un contrat M1 depuis le CFA | SYS | écarts signalés | `interop.reception` | PBA8 |
| PBA-199 | Voir son contrat, son calendrier, son livret | APP | — | — | PBA7 |
| PBA-200 | Créer ou rattacher un employeur (SIRET contrôlé) | COO | 39 ; Sirene | — | PBA7 |
| PBA-201 | Déclarer un maître d'apprentissage et lui ouvrir un lien | COO | sans compte ; révocable | `apprentissage.lien_ouvert` | PBA7 |
| PBA-202 | Révoquer un lien | COO | — | `apprentissage.lien_revoque` | PBA7 |

### 20.7 Échanges avec le CFA (M1)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PBA-210 | Déclarer l'échange avec le CFA (format, fréquence, champs) | DIR | 9 | `interop.echange_declare` | PBA8 |
| PBA-211 | Exporter l'assiduité mensuelle et les heures réalisées | COO | tracé ; figé | `interop.envoi` | PBA8 |
| PBA-212 | Exporter les évaluations du semestre | COO | — | `interop.envoi` | PBA8 |
| PBA-213 | Importer les contrats et leurs états | COO | proposition | `interop.reception` | PBA8 |
| PBA-214 | Voir les écarts CFA ↔ ScolIA | COO | CPB-44 | — | PBA8 |

### 20.8 Suivi en entreprise, livret, mineurs
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PBA-220 | Planifier une visite en entreprise | ENS | 19 | `agenda.rdv_cree` | PBA9 |
| PBA-221 | Confirmer la visite (maître d'apprentissage) | EMP | lien | — | PBA9 |
| PBA-222 | Rédiger le compte rendu de visite | ENS | IA proposée | — | PBA9 |
| PBA-223 | Compléter le livret (compétences en entreprise) | EMP | 22 | `apprentissage.livret` | PBA9 |
| PBA-224 | Compléter le livret (formateur, apprenti) | ENS, APP | — | `apprentissage.livret` | PBA9 |
| PBA-225 | Tenir un entretien tripartite (compte rendu signé par trois) | ENS | — | — | PBA9 |
| PBA-226 | Signaler un risque de rupture | ENS, COO | tâche | `apprentissage.alerte` | PBA9 |
| PBA-227 | Tenir le registre des dérogations de l'établissement | DIR | 3 ans | `apprentissage.derogation` | PBA10 |
| PBA-228 | Alerte d'échéance d'une dérogation | SYS → DIR | S-5110 | — | PBA10 |
| PBA-229 | Enregistrer l'avis d'aptitude (date, validité, sans contenu médical) | COO | R-PBA26 | — | PBA10 |
| PBA-230 | Enregistrer la formation sécurité d'un jeune | ENS | — | — | PBA10 |
| PBA-231 | Bloquer l'inscription d'un mineur non conforme en atelier | SYS | R-PBA25 | — | PBA10 |
| PBA-232 | Désigner le référent handicap et le référent mobilité | DIR | 6 | — | PBA10 |

### 20.9 Financement OPCO (M2)
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PBA-240 | Enregistrer une version de barème NPEC | GES | versionné | — | PBA11 |
| PBA-241 | Calculer la prise en charge d'un contrat (prorata jours) | SYS | RP-09 | — | PBA11 |
| PBA-242 | Générer l'échéancier 40/30/20/10 | SYS | — | `apprentissage.echeancier` | PBA11 |
| PBA-243 | Émettre une facture OPCO | GES, CPT | 24 ; entité émettrice | `facturation.emise` | PBA11 |
| PBA-244 | Générer et signer le certificat de réalisation | GES, DIR | heures réalisées | `documents.genere` | PBA11 |
| PBA-245 | Recalculer au prorata après rupture | SYS | R6332-25 | — | PBA11 |
| PBA-246 | Traiter un rejet de l'OPCO | GES | file | — | PBA11 |
| PBA-247 | Rapprocher un encaissement OPCO | CPT | 25 | `paiements.rapproche` | PBA11 |
| PBA-248 | Facturer la participation de l'employeur (décision) | GES | barème | `facturation.emise` | PBA11 |

### 20.10 Qualité, instances, taxe, indicateurs
| N° | Action | Acteur · droit | Règles | Événement | Lot |
|---|---|---|---|---|---|
| PBA-250 | Tenir le conseil de perfectionnement | DIR | RP-08 | — | PBA12 |
| PBA-251 | Tenir le registre des réclamations | DIR | Qualiopi [NV] | — | PBA12 |
| PBA-252 | Lancer une enquête de satisfaction anonyme | COO | 18 | — | PBA12 |
| PBA-253 | Préparer l'export BPF | GES | personne déclare | — | PBA12 |
| PBA-254 | Préparer l'export SIFA | GES | idem | — | PBA12 |
| PBA-255 | Afficher les indicateurs publiés (source, date) | DIR | R-PBA34 | — | PBA13 |
| PBA-256 | Rappel de campagne SOLTéA | SYS → DIR | S-5112 | — | PBA12 |
| PBA-257 | Enregistrer les versements de taxe annoncés et reçus | CPT | 48 | — | PBA12 |
| PBA-258 | Campagne auprès des entreprises partenaires | DIR | sans donnée d'apprenant | `annonces.publiee` | PBA12 |
| PBA-259 | Remercier une entreprise (lettre type) | DIR | 7 | `documents.genere` | PBA12 |

**Correctifs et reprise** : PBA-260 (PBA0) refus de la réécriture du statut quand l'année a commencé ; PBA-261 (PBA0) échappeur commun pour Parcoursup (= ECO-271) ; PBA-262 (PBA14) reprise des contrats d'un logiciel de CFA (fichier, zone transitoire) ; PBA-263 (PBA14) reprise des entreprises ; PBA-264–PBA-271 : ajout personas (§27) ; PBA-272–PBA-299 réservées.

---

## 21. Règles transverses (R-PBA1 à R-PBA40) [R]

| # | Règle |
|---|---|
| R-PBA1 | Le modèle d'apprentissage (M1 / M2) est porté **par l'unité**. |
| R-PBA2 | Le statut d'apprenant est une **période datée** sans chevauchement ; il n'est jamais réécrit. |
| R-PBA3 | Les règles (destinataires, assiduité, facturation) sont **déduites** du statut à la date du fait. |
| R-PBA4 | Un étudiant ou apprenti **majeur** est seul destinataire, sauf désignation datée et révocable. |
| R-PBA5 | Les titulaires d'un **mineur** restent destinataires de ce que la loi prévoit jusqu'à ses 18 ans. |
| R-PBA6 | Un contrat M1 est une **copie de lecture** : resynchronisée, jamais modifiée sans trace. |
| R-PBA7 | Toute pièce officielle (CERFA, rupture, dérogation) est **importée** et datée. |
| R-PBA8 | La période d'essai se compte en **jours de formation pratique en entreprise**. |
| R-PBA9 | La rupture est un fait daté avec un **motif légal** ; elle arrête les avis à l'employeur. |
| R-PBA10 | L'appel n'attend l'apprenti que les jours **en centre**. |
| R-PBA11 | Une absence en centre d'un apprenti est une **heure non réalisée**. |
| R-PBA12 | L'employeur reçoit l'assiduité **sans motif médical**, selon la convention et le réglage. |
| R-PBA13 | L'apprenti voit **tout ce qui est transmis** à son employeur. |
| R-PBA14 | Le taux de formation en centre est contrôlé à **25 %** et au minimum du diplôme. |
| R-PBA15 | Une fermeture du centre ne rend pas l'apprenti absent. |
| R-PBA16 | Le maître d'apprentissage accède par **lien nominatif**, limité, révocable, journalisé. |
| R-PBA17 | Le livret est la **seule** évaluation visible de l'employeur. |
| R-PBA18 | Une entreprise ne voit **que** ses apprentis. |
| R-PBA19 | Aucun signalement CROUS pour une absence de **moins d'un mois**. |
| R-PBA20 | La liste CROUS est **signée par la direction** après la procédure contradictoire. |
| R-PBA21 | Une dispense d'assiduité **exclut** le signalement. |
| R-PBA22 | L'attestation CVEC n'est demandée qu'en **CPGE**. |
| R-PBA23 | L'inscription parallèle est contrôlée avant la **fin décembre**. |
| R-PBA24 | Les crédits européens sont **attribués par une instance**, jamais calculés seuls. |
| R-PBA25 | Un **mineur** n'est inscrit à une séance de travaux réglementés qu'avec dérogation valide, avis d'aptitude valide, formation sécurité et encadrant désigné. |
| R-PBA26 | L'avis d'aptitude est un fait **daté sans contenu médical**, inaccessible à l'IA. |
| R-PBA27 | Le mode d'épreuve (CCF / ponctuel) est fixé **par candidat** ; CCF seulement si l'unité est habilitée. |
| R-PBA28 | Les notes de CCF suivent la confidentialité du règlement d'examen. |
| R-PBA29 | Une note de colle est une évaluation du socle 21 avec son historique. |
| R-PBA30 | **Aucun montant** réglementaire (NPEC, aides, participation) n'est codé : barèmes **versionnés**. |
| R-PBA31 | Le financement se calcule **au prorata des jours** ; une rupture recalcule ; mois entamé dû. |
| R-PBA32 | Une facture OPCO émise ne se modifie pas ; elle se compense (D-027 E). |
| R-PBA33 | Le certificat de réalisation est **signé** par la direction de l'organisme de formation. |
| R-PBA34 | Les indicateurs officiels sont **affichés tels que publiés** ; aucun taux maison présenté comme officiel. |
| R-PBA35 | BPF et SIFA sont **préparés** ; une personne les déclare. |
| R-PBA36 | Le SIRET est contrôlé (format et clé). |
| R-PBA37 | Une entreprise radiée déclenche une **alerte**, jamais un effet automatique. |
| R-PBA38 | Une campagne de taxe d'apprentissage ne contient **aucune** donnée d'apprenant. |
| R-PBA39 | ScolIA est la source de l'**assiduité en centre** ; un écart avec le CFA est signalé, pas écrasé. |
| R-PBA40 | L'IA ne décide **ni** bourse, **ni** signalement, **ni** rupture, **ni** évaluation. |

---

## 22. Droits et minimisation

### 22.1 Catalogue (D-017) [R]
`postbac.credits` · `postbac.crous` · `cpge.colles` · `apprentissage.contrats` · `apprentissage.calendrier` · `apprentissage.employeur_informer` · `apprentissage.livret` · `apprentissage.derogations` · `apprentissage.financement` (M2) · `apprentissage.qualite` (M2) · `interop.cfa` · modèles de rôles : **Coordinateur d'UFA / DDFPT** (portée `mon_unite`), **Gestionnaire du CFA** (M2), **Colleur** (trinômes du jour), **Formateur référent** (apprentis suivis).

### 22.2 Ce que voit chacun
| Donnée | ENS | COL | COO | VS | EMP (lien) | APP | TIT (mineur) | Parents d'un majeur |
|---|---|---|---|---|---|---|---|---|
| Identité | ses cours | trinômes | unité | unité | son apprenti | soi | oui | non |
| Absences | ses cours | non | unité | unité | **sans motif**, son apprenti | oui | oui | non (sauf désignation) |
| Notes, crédits | ses cours | ses colles | unité | non | **livret seulement** | oui | oui (+ livret, ajout personas) | non |
| Contrat | non | non | unité | non | le sien | oui | oui | non |
| Bourse | non | non | non | boursiers (sans échelon) | non | oui | non | non |
| Avis d'aptitude | non | non | unité (date) | non | non | oui | oui | non |
| Financement OPCO | non | non | non (M2 : GES) | non | non | non | non | non |

### 22.3 Conservation [R, à valider avec le DPO, Q-PBA14]
Contrats et pièces de financement : durée des contrôles OPCO et comptables (à fixer : 5 à 10 ans [NV]) ; listes CROUS : 3 ans [R] ; liens employeurs : révoqués à la fin du contrat, journaux 1 an ; avis d'aptitude : fin du contrat + 1 an [R] ; colloscopes : fin d'année.

### 22.4 Pièges à tester
Employeur A qui ouvre l'apprenti de l'employeur B ; maître d'apprentissage qui lit les notes ; parents d'un étudiant de 20 ans ; IA à qui l'on demande la liste des boursiers « à risque » ; colleur qui ouvre un autre trinôme ; ancien maître d'apprentissage après avenant ; liste CROUS sans signature ; mineur inscrit en atelier sans avis.

---

## 23. Événements et notifications

**Émis** : `eleve.statut_ouvert|change`, `postbac.credits_attribues`, `postbac.inscription_parallele`, `postbac.dispense`, `postbac.liste_crous_*`, `cpge.colloscope_*`, `cpge.colle_reportee`, `apprentissage.calendrier_*`, `apprentissage.contrat_*`, `apprentissage.avenant`, `apprentissage.employeur_informe`, `apprentissage.attestation_employeur`, `apprentissage.lien_*`, `apprentissage.livret`, `apprentissage.alerte`, `apprentissage.derogation`, `apprentissage.echeancier`, `examens.mode_fixe|ccf_*`.
**Consommés** : `assiduite.*` (12), `structure.*` (5 : fermetures), `eleve.majeur` (2), `stages.*` (39), `evaluations.*` (21), `personnels.*` (6 : colleurs).
**Notifications** : demande de justificatif (étudiant), récapitulatif d'absences (employeur, lien), fin de période d'essai proche (coordinateur), rupture enregistrée (formateur référent), inscription universitaire manquante (étudiant), échéance de dérogation (direction), colloscope publié (étudiants, colleurs), visite à confirmer (maître d'apprentissage).

---

## 24. Écrans

- **Tableau « Apprentissage »** (coordinateur, tuiles) : contrats en essai, ruptures du mois, apprentis sans employeur, absences à transmettre, dérogations à échéance, écarts CFA.
- **Fiche apprenant** (onglet du socle 2) : statuts datés, contrat, calendrier, heures réalisées, employeur et maître d'apprentissage, livret.
- **Calendrier d'alternance** : vue annuelle par sous-public et exceptions.
- **Colloscope** : grille semaines × trinômes × matières ; vue mobile du colleur.
- **Assiduité étudiante** : boursiers sous le seuil, justificatifs, liste CROUS à signer.
- **Espace employeur** (lien) : calendrier, absences, livret, visites.
- **Mobile** (11) : appel ; colles ; l'apprenti voit son calendrier et ce qui part à son employeur.

---

## 25. Modèle de données cible (brique `postbac` et `apprentissage`) [R]

Mêmes exigences que le 50 (E1–E14 : `etablissement_id NOT NULL`, FK composites, UUID applicatifs, horodatages, RLS, aucun JSON ni tableau, fichiers par `ent_objet`, journaux en ajout seul, registre).

```sql
-- PBA1 (3020–3021) : statut daté
ent_eleve_statut (id, etablissement_id, scolarisation_id, eleve_id,
  statut CHECK (scolaire|etudiant|apprenti|apprenti_sans_contrat|stagiaire_formation_continue|autre),
  du DATE, au DATE NULL, source CHECK (inscription|contrat|rupture|decision|reprise), source_ref UUID NULL,
  EXCLUDE gist (eleve_id WITH =, daterange(du, au) WITH &&))
ent_eleve_statut_journal (id, etablissement_id, statut_id, a, acteur, action, valeur_avant TEXT, valeur_apres TEXT, motif) -- ajout seul

-- PBA2–PBA3 (3022–3023) : unités post-bac, crédits, CCF
ent_unite_apprentissage (etablissement_id, unite_id, modele CHECK (m1_ufa|m2_cfa), cfa_partenaire_id NULL,
  convention_objet_id NULL, numero_declaration_activite TEXT NULL, du, au NULL)
ent_cfa_partenaire (id, etablissement_id, raison_sociale, siret, uai NULL, contact_nom, contact_courriel)
ent_postbac_ue (id, etablissement_id, unite_id, formation_code, code, libelle, credits SMALLINT, semestre SMALLINT)
ent_postbac_credit (id, etablissement_id, eleve_id, ue_id, annee_id, acquis BOOL, decide_le, instance_seance_id NULL) -- historique
ent_examen_habilitation_ccf (id, etablissement_id, unite_id, diplome_code, du, au NULL, objet_id)
ent_examen_mode_candidat (id, etablissement_id, eleve_id, annee_id, epreuve_code, mode CHECK (ccf|ponctuel), motif TEXT NULL)
ent_ccf_situation (id, etablissement_id, cours_id, epreuve_code, date_prevue, sujet_objet_id NULL, etat CHECK (planifiee|realisee|remplacee|annulee))
ent_ccf_jury (etablissement_id, situation_id, personne_id, qualite)
ent_ccf_evaluation (id, etablissement_id, situation_id, eleve_id, resultat_numerique NUMERIC(5,2) NULL, absence CHECK (non|justifiee|injustifiee), saisi_par, saisi_le)
ent_ccf_pv (id, etablissement_id, situation_id, arrete_le, objet_id, transmis_le NULL)

-- PBA4 (3024–3025) : CPGE
ent_cpge_convention_universite (id, etablissement_id, unite_id, universite_libelle, universite_uai NULL, du, au NULL, objet_id)
ent_cpge_inscription_parallele (id, etablissement_id, eleve_id, annee_id, convention_id, inscrit_le DATE NULL,
  numero_etudiant TEXT NULL, attestation_cvec_objet_id NULL)
ent_cpge_trinome (id, etablissement_id, classe_id, annee_id, numero SMALLINT)
ent_cpge_trinome_membre (etablissement_id, trinome_id, eleve_id, du, au NULL)
ent_cpge_colloscope (id, etablissement_id, classe_id, periode_du, periode_au, version INT, publie_le NULL)
ent_cpge_colle (id, etablissement_id, colloscope_id, semaine DATE, matiere_id, trinome_id, colleur_agent_id,
  creneau_debut TIMESTAMPTZ, salle_id NULL, etat CHECK (prevue|faite|reportee|annulee), evaluation_id NULL)
ent_cpge_colle_heure (id, etablissement_id, colle_id, colleur_agent_id, minutes INT, mois DATE) -- vers la paie (49)

-- PBA5 (3026) : bourses et assiduité étudiante
ent_postbac_boursier (id, etablissement_id, eleve_id, annee_id, declare_le, source CHECK (etudiant|crous|secretariat))
ent_postbac_dispense_assiduite (id, etablissement_id, eleve_id, du, au, motif_code, objet_id NULL, decide_par)
ent_postbac_justificatif_demande (id, etablissement_id, eleve_id, demande_le, echeance DATE, repondu_le NULL, objet_id NULL, issue CHECK (justifie|non_justifie|sans_reponse) NULL)
ent_postbac_liste_crous (id, etablissement_id, annee_id, prepare_le, signe_le NULL, signe_par NULL, transmis_le NULL, objet_id NULL)
ent_postbac_liste_crous_ligne (id, etablissement_id, liste_id, eleve_id, nature CHECK (defaut_assiduite|abandon|reorientation|alternance|absence_examen), date_fait DATE, demande_id NULL)

-- PBA6 (3027–3028) : alternance, heures réalisées, information de l'employeur
ent_alternance_calendrier (id, etablissement_id, groupe_id, annee_id, libelle, version INT, publie_le NULL)
ent_alternance_jour (etablissement_id, calendrier_id, jour DATE, lieu CHECK (centre|entreprise|conge), PRIMARY KEY (calendrier_id, jour))
ent_alternance_exception (id, etablissement_id, eleve_id, jour DATE, lieu CHECK (centre|entreprise|conge), motif TEXT)
ent_apprentissage_heures (id, etablissement_id, contrat_id, mois DATE, heures_prevues NUMERIC(7,2), heures_realisees NUMERIC(7,2), arrete_le NULL) -- figé à l'arrêt
ent_apprentissage_info_employeur (id, etablissement_id, contrat_id, periode_du, periode_au, valide_par, envoye_le, lien_id) -- ajout seul
ent_apprentissage_info_employeur_ligne (etablissement_id, info_id, jour DATE, demi_journee CHECK (matin|apres_midi), etat CHECK (absent_justifie|absent_non_justifie|retard))
ent_apprentissage_attestation_employeur (id, etablissement_id, contrat_id, jour DATE, motif CHECK (demande_employeur|autre), recue_le, lien_id)

-- PBA7 (3029–3030) : contrats, employeurs, maîtres d'apprentissage, liens
ent_apprentissage_contrat (id, etablissement_id, eleve_id, unite_id, source CHECK (cfa_externe|scolia),
  reference_source TEXT NULL, employeur_entreprise_id, siret_execution TEXT CHECK (siret_execution ~ '^[0-9]{14}$'),
  diplome_code, rncp_code TEXT NULL, opco_code TEXT NULL, debut DATE, fin DATE, debut_formation DATE,
  numero_depot TEXT NULL, etat CHECK (projet|signe|transmis_opco|depose|refuse_depot|en_cours|rompu_essai|rompu|termine),
  CHECK (fin > debut))
ent_apprentissage_contrat_fait (id, etablissement_id, contrat_id, a, nature CHECK (signature|transmission|depot|refus|avenant|rupture|fin|resynchronisation),
  date_effet DATE, motif_code TEXT NULL, objet_id NULL, acteur, detail TEXT NULL) -- ajout seul
ent_apprentissage_maitre (id, etablissement_id, contrat_id, personne_id, du, au NULL)
ent_apprentissage_lien (id, etablissement_id, contrat_id, personne_id, empreinte_jeton TEXT CHECK (empreinte_jeton ~ '^[0-9a-f]{64}$'), expire_le, revoque_le NULL) -- jeton haché (SHA-256 en hexadécimal), jamais en clair
ent_apprentissage_lien_acces (id, etablissement_id, lien_id, a, objet_consulte) -- ajout seul

-- PBA9–PBA10 (3031–3032) : livret, visites, mineurs
ent_apprentissage_livret_ligne (id, etablissement_id, contrat_id, element_id, observe_le, auteur_personne_id, role CHECK (maitre|formateur|apprenti), appreciation TEXT, niveau SMALLINT NULL)
ent_apprentissage_entretien (id, etablissement_id, contrat_id, a, compte_rendu TEXT, signe_apprenti_le NULL, signe_maitre_le NULL, signe_formateur_le NULL)
ent_apprentissage_alerte (id, etablissement_id, contrat_id, a, nature CHECK (risque_rupture|difficulte|autre), description, traitee_le NULL)
ent_derogation_travaux (id, etablissement_id, lieu_libelle, travaux_codes_texte TEXT, declaree_le DATE, preuve_objet_id, echeance DATE)
ent_derogation_travaux_code (etablissement_id, derogation_id, code)   -- liste des travaux réglementés en table, pas en tableau
ent_jeune_aptitude (id, etablissement_id, eleve_id, derogation_id, avis_le DATE, valide_jusqu_au DATE, formation_securite_le DATE NULL, encadrant_agent_id NULL)

-- PBA11 (3033–3034) : financement OPCO (M2)
ent_npec_bareme (id, diplome_code, branche_code, montant_centimes BIGINT, du, au NULL, source_objet_id NULL) -- **référence globale** (E4 : `globale_reference`, sans donnée d'établissement)
ent_opco_prise_en_charge (id, etablissement_id, contrat_id, bareme_id, montant_annuel_centimes BIGINT, jours_prevus INT)
ent_opco_echeance (id, etablissement_id, prise_en_charge_id, rang SMALLINT CHECK (rang BETWEEN 1 AND 4), pourcentage SMALLINT, montant_centimes BIGINT, exigible_le, facture_id NULL)
ent_opco_rejet (id, etablissement_id, facture_id, recu_le, motif TEXT, traite_le NULL)
ent_apprentissage_certificat (id, etablissement_id, contrat_id, periode_du, periode_au, heures_realisees NUMERIC(7,2), signe_le, signe_par, objet_id)

-- PBA12 (3035) : qualité, instances, taxe
ent_cfa_reclamation (id, etablissement_id, unite_id, recue_le, auteur_personne_id NULL, objet TEXT, traitement TEXT NULL, close_le NULL)
ent_taxe_apprentissage_habilitation (id, etablissement_id, campagne_annee SMALLINT, depose_le NULL, arrete_objet_id NULL, habilite BOOL NULL)
ent_taxe_apprentissage_versement (id, etablissement_id, campagne_annee, entreprise_id NULL, annonce_centimes BIGINT NULL, recu_centimes BIGINT NULL, recu_le NULL)
-- conseil de perfectionnement : instance du socle 46 / `ent_ecole_instance` généralisée (nature `conseil_perfectionnement`)

-- PBA13 (3036) : indicateurs publiés
ent_indicateur_publie (id, etablissement_id, unite_id, code, valeur NUMERIC, millesime TEXT, source_url TEXT, releve_le DATE)

-- PBA14 (3036) : reprise d'un logiciel de CFA (zone transitoire TEXT, détruite à 60 jours)
ent_apprentissage_reprise_ligne (id, etablissement_id, lot_id, numero INT, colonne TEXT, valeur TEXT, statut CHECK (attente|repris|rejete))
```

**Registre (E12)** : toutes `etablissement` sauf `ent_npec_bareme` (`globale_reference`) ; `ent_apprentissage_lien` et `ent_apprentissage_info_employeur` **neutralisés à la restauration** (E9 : pas de renvoi aux employeurs, jetons invalidés) ; export `particuliere` pour `ent_jeune_aptitude` et la bourse. **D-036** : aucune colonne JSON ni tableau ; **aucun `BYTEA`** (l'empreinte du jeton est en hexadécimal) ; les jetons sont classés comme ceux du socle 1.

---

## 26. Lots et migrations

| Lot | Contenu | Dépend de | Critères de fin | Migration |
|---|---|---|---|---|
| **PBA0** correctifs | PBD-01 (refus de réécrire le statut d'une année commencée avec des faits), PBD-04 (échappeur, = ECO-271) | — | test : statut d'une année commencée non réécrit | aucune |
| **PBA1** statut daté | §7, PBA-102 à PBA-105 ; projection vers `statut_apprenant` | 2 E4 | aucune période chevauchante ; reprise sans conversion de « autre » | 3020–3021 |
| **PBA2** unités post-bac | PBA-100, 101, 106–108 ; semestres ; sous-publics | 50 EC1 ; 5 | M1/M2 par unité | 3022 |
| **PBA3** BTS | crédits, CCF, stages, pondération | PBA2 ; 21 ; 38 ; 39 | mode par candidat ; habilitation | 3023 |
| **PBA4** CPGE | convention, inscriptions parallèles, colles, crédits, attestation | PBA2 ; 21 ; 10 | colloscope ; heures de colle | 3024–3025 |
| **PBA5** bourses et assiduité étudiante | §11 | 12 AS5, AS11 | < 1 mois exclu ; liste signée | 3026 |
| **PBA6** alternance et assiduité apprenti | §13 | PBA1 ; 10 ; 12 | appel selon le calendrier ; info employeur sans motif | 3027–3028 |
| **PBA7** contrats (M1 lecture) | §12, PBA-190 à PBA-202 | PBA6 ; 39 ; 7 | essai compté en jours d'entreprise ; rupture ; liens | 3029–3030 |
| **PBA8** échanges CFA (M1) | PBA-198, PBA-210 à PBA-214 | PBA7 ; 9 X7 | écarts signalés, jamais écrasés | (tables du 9) |
| **PBA9** suivi et livret | PBA-220 à PBA-226 | PBA7 ; 19 ; 22 | livret seule évaluation employeur | 3031 |
| **PBA10** mineurs et référents | PBA-227 à PBA-232 | PBA7 ; 6 | blocage atelier testé | 3032 |
| **PBA11** OPCO (M2) | §15 | PBA7 ; 24 ; 25 | prorata ; rupture ; aucun montant codé | 3033–3034 |
| **PBA12** qualité, instances, taxe | §16, §17 | PBA11 (M2) ; 46 ; 48 | exports préparés, pas envoyés | 3035 |
| **PBA13** indicateurs | PBA-255 ; 47 | — | source affichée | 3036 |
| **PBA14** reprise, RLS stricte, retraits | PBA-262, PBA-263 ; retrait de `statut_apprenant` | tous | registre complet | 3036 (reprise), 3037 (personas), 3038 (RLS stricte), 3039 (retraits, sur décision, E8) |

---

## 27. Ajout personas (`PERSONAS_51.md`)

> Écrit après la passe personas ; l'emporte en cas de contradiction.

### 27.1 Bloquants tranchés (D-031)
- **A. Le lycée pilote est probablement en M1 et le CFA a déjà son logiciel** (P01, P08) : le **premier livrable utile** n'est pas le contrat mais l'**export d'assiduité et d'heures réalisées vers le CFA** et l'**information de l'employeur** ; PBA8 passe **avant** PBA7 dans le parcours minimal, avec une saisie **minimale** du contrat (dates, employeur, maître, essai) suffisante pour l'appel et l'essai (PBA-264).
- **B. L'employeur ne doit recevoir que ce que l'apprenti a pu voir avant** (P05, P13) : le récapitulatif hebdomadaire est **visible par l'apprenti 24 h avant** son envoi (S-5113, défaut 24 h) ; l'apprenti peut **signaler une erreur** (justificatif déposé) avant l'envoi (PBA-265, R-PBA41).
- **C. La liste CROUS doit pouvoir être contestée** (P06, P15) : chaque étudiant porté sur la liste reçoit, **avant** la signature, l'information qu'il y figure et le motif, avec le délai restant (PBA-266, R-PBA42) ; cela fait partie de la procédure contradictoire.
- **D. Le lien employeur est la surface d'attaque principale** (P14) : lien nominatif **lié à une adresse vérifiée**, code à usage unique à chaque ouverture (courriel), expiration à 30 jours renouvelée par le coordinateur, **aucune** liste d'apprentis visible, journal des accès consultable par l'apprenti (R-PBA43, PBA-267).

### 27.2 Actions ajoutées (PBA-264 à PBA-271)
| N° | Action | Lot |
|---|---|---|
| PBA-264 | Saisie minimale d'un contrat M1 (dates, employeur, maître, début de formation) pour l'appel et l'essai | PBA6 |
| PBA-265 | Aperçu par l'apprenti du récapitulatif à l'employeur, signalement d'erreur avant envoi | PBA6 |
| PBA-266 | Information préalable de l'étudiant porté sur la liste CROUS (motif, délai) | PBA5 |
| PBA-267 | Ouverture du lien employeur par code à usage unique ; journal des accès visible de l'apprenti | PBA7 |
| PBA-268 | Planning des colles imprimable par colleur, pour les colleurs sans compte actif | PBA4 |
| PBA-269 | « Apprentis sans employeur » : liste, délai restant [NV], actions d'accompagnement | PBA7 |
| PBA-270 | Attestation employeur de **présence en entreprise** (pour la période d'essai) en un geste | PBA7 |
| PBA-271 | Bascule de statut en masse au 1er septembre (classe entière de BTS en apprentissage) avec aperçu | PBA1 |

### 27.3 Règles ajoutées
| # | Règle |
|---|---|
| R-PBA41 | Un récapitulatif à l'employeur est visible de l'apprenti **avant** l'envoi (S-5113). |
| R-PBA42 | Un étudiant porté sur une liste CROUS en est **informé avant** la signature. |
| R-PBA43 | Le lien employeur exige un **code à usage unique** par ouverture ; il ne montre jamais d'autre apprenti. |
| R-PBA44 | Un changement de statut en masse passe par un **aperçu** et une confirmation. |
| R-PBA45 | Les heures de colle d'un colleur ne sont visibles que de lui, du coordinateur et de la paie. |
| R-PBA46 | L'échelon de bourse n'est **jamais** saisi par défaut (minimisation) ; seul le fait « boursier » compte. |

### 27.4 Cas limites ajoutés
CPB-63 apprenti qui conteste une absence la veille de l'envoi (R-PBA41) · CPB-64 étudiant qui découvre sa présence sur la liste CROUS (R-PBA42) · CPB-65 lien employeur ouvert depuis une autre adresse (code exigé) · CPB-66 classe de BTS passée en apprentissage au 1er septembre (PBA-271) · CPB-67 colleur extérieur sans compte (PBA-268) · CPB-68 employeur qui n'atteste jamais la présence (essai estimé, alerte) · CPB-69 CFA qui exige un format de fichier propriétaire (adaptateur d'échange du 9, pas de format maison).

### 27.5 Lots révisés
Parcours minimal : PBA0 → PBA1 → PBA2 → **PBA6 (avec PBA-264)** → **PBA8** → PBA5 → PBA3 → PBA7 complet → PBA9. Réglages ajoutés : **S-5113** (délai d'aperçu par l'apprenti avant envoi à l'employeur, 24 h), **S-5114** (durée du lien employeur, 30 jours), **S-5115** (information des titulaires d'un apprenti mineur : récapitulatif identique à l'employeur, oui).

### 27.6 Précisions issues de `PERSONAS_51.md` §6.2
- **Récapitulatif à l'employeur** : il inclut les **dates d'évaluation à venir** (CCF, examens), jamais les notes (M-5105, M-5130).
- **Export mensuel vers le CFA** : **figé** à l'envoi (instantané) ; une correction part en **rectificatif** (M-5120). Une **rupture** apprise par le lycée part au CFA **le jour même** (M-5121).
- **R-PBA33 précisée** : en **M1**, ScolIA **ne signe aucun certificat de réalisation** ; il fournit les heures au CFA (M-5122). La règle ne vaut qu'en M2.
- **Titulaires d'un apprenti mineur** : ils lisent le **livret** (autorité parentale) en plus du récapitulatif (M-5124) ; le tableau du §22.2 est corrigé en ce sens. L'apprenti est prévenu un mois avant ses 18 ans de la fin de leur accès et peut les désigner (M-5125).
- **Attestation descriptive** (CPGE) déclenchée automatiquement à la **sortie** en cours d'année (M-5118).
- **« Moins d'un mois »** (R-PBA19) : compté en **absences consécutives** jusqu'à lecture de la circulaire 2026-2027 (M-5115, Q-PBA43).
- **Format du CFA** : fichier produit par un **adaptateur d'échange** au format du partenaire (socle 9, X7), jamais un format maison (M-5102, CPB-69).
- **Réglages ajoutés** : **S-5116** rappel d'inscription parallèle (15 octobre) ; **S-5117** code d'accès du maître d'apprentissage par SMS (défaut **non**, coût, D-028 Q-N22) ; **S-5118** récapitulatif groupé par employeur (défaut **non** : un message par apprenti).
- **Préalable de mise en service** de PBA6 à PBA10 : **AIPD** et avis du DPO (M-5136, M-5141, Q-PBA41).
- **Urgent (soumis à Florian)** : date limite **SOLTéA du 11 décembre 2026** pour être habilité en 2027 (M-5133, Q-PBA38).

---

## 28. Décisions appliquées par défaut (D-031) et questions

### 28.1 Appliquées par défaut
| # | Question | Recommandation appliquée |
|---|---|---|
| Q-PBA2 | Statut annuel ou daté ? | **Daté** (R-PBA2) |
| Q-PBA3 | Valeur « étudiant » ? | **Oui** |
| Q-PBA4 | Modèle M1/M2 au tenant ou à l'unité ? | **À l'unité** (R-PBA1) |
| Q-PBA5 | Contrat M1 : copie de lecture ? | **Oui** (R-PBA6) |
| Q-PBA6 | Information de l'employeur : automatique ou validée ? | **Récapitulatif hebdomadaire validé** (S-5103) |
| Q-PBA7 | Motif de l'absence à l'employeur ? | **Jamais médical** ; justifiée / non justifiée (R-PBA12) |
| Q-PBA8 | Parents d'un majeur ? | **Non destinataires** par défaut (R-PBA4) |
| Q-PBA9 | Contrats parallèles (deux employeurs) ? | **Refusés** par défaut, exception à étudier |
| Q-PBA10 | Seuil CROUS ? | Paramétrable (S-5100) ; **< 1 mois exclu** (R-PBA19) |
| Q-PBA11 | Échelon de bourse ? | **Non saisi** (R-PBA46) |
| Q-PBA12 | Crédits calculés ou attribués ? | **Attribués** par l'instance (R-PBA24) |
| Q-PBA13 | Colloscope : moteur du 10 ou algorithme simple ? | **Algorithme de rotation simple** d'abord, moteur du 10 si besoin |
| Q-PBA15 | Montants réglementaires dans le code ? | **Jamais** (R-PBA30) |
| Q-PBA16 | ScolIA génère-t-il le CERFA en M1 ? | **Non** ; importé |
| Q-PBA17 | Avis d'aptitude : contenu ? | **Date et validité seulement** (R-PBA26) |
| Q-PBA18 | Blocage atelier des mineurs non conformes ? | **Oui** (R-PBA25) |
| Q-PBA19 | Indicateurs : calcul maison ? | **Non**, valeurs publiées (R-PBA34) |
| Q-PBA28 | Source de l'assiduité en centre ? | **ScolIA** (R-PBA39) |
| Q-PBA29 | Lien employeur : code à usage unique ? | **Oui** (R-PBA43) |
| Q-PBA30 | Aperçu par l'apprenti avant envoi ? | **Oui, 24 h** (R-PBA41) |
| Q-PBA31 | Information préalable avant la liste CROUS ? | **Oui** (R-PBA42) |
| Q-PBA32 | Stage de BTS pour un apprenti ? | **Sans objet** [H] |
| Q-PBA33 | CVEC demandée en BTS ? | **Non** (R-PBA22) |
| Q-PBA34 | Exclusion temporaire : motif à l'employeur ? | **Absence seulement**, motif sur décision (S-5104) |
| Q-PBA35 | Notes de CCF communiquées avant le jury ? | Selon le règlement (S-5106, défaut **non**) |
| Q-PBA36 | Contrat BYTEA ou pièce ? | **Pièce** (`ent_objet`, E5) |

### 28.2 Soumis à Florian
| # | Point | Pourquoi |
|---|---|---|
| **Q-PBA1** | Le groupe pilote a-t-il des BTS, une CPGE, une UFA ? **M1 ou M2 ?** Quel CFA partenaire et quel logiciel (Ypareo ou autre) ? | Données réelles ; décide l'ordre des lots |
| **Q-PBA14** | Durées de conservation (contrats, pièces OPCO, listes CROUS) | DPO |
| **Q-PBA37** | Convention CFA-UFA : qui informe l'employeur, quels fichiers, quel format | Contrat avec un tiers |
| **Q-PBA38** | **Habilitation SOLTéA** (dossier avant le 11 décembre 2026 pour 2027) | Démarche officielle à date fixe : **urgent** si l'établissement veut percevoir le solde en 2027 et ne l'a pas encore déposé |
| **Q-PBA39** | Démarches éditeur : Parcoursup (établissement d'accueil), DECA, API apprentissage, LSL BTS | Démarches officielles |
| **Q-PBA40** | Convention avec l'université (CPGE) et transmission des listes | Contrat avec un tiers |
| **Q-PBA41** | Avis du DPO sur les **données d'apprentis** (salariés, mineurs, aptitude) et sur l'**accès des employeurs** | Analyse d'impact probable |
| **Q-PBA42** | Paiement des **colleurs** : par l'État (public) ou par l'OGEC (privé) ? | Paie (49), données réelles |

### 28.3 Réglages signalés (D-023)
| # | Réglage | Défaut |
|---|---|---|
| S-5100 | Seuil d'assiduité des boursiers | à fixer par l'établissement (proposition : 80 % sur le mois) |
| S-5101 | Délai de réponse au justificatif | 8 jours |
| S-5102 | Périodisation post-bac | semestres |
| S-5103 | Information de l'employeur | récapitulatif hebdomadaire validé |
| S-5104 | Motif de l'exclusion communiqué à l'employeur | non |
| S-5105 | Alerte de fin de période d'essai | J-10 |
| S-5106 | Notes de CCF visibles avant le jury | non |
| S-5107 | Colloscope : rotation automatique | oui |
| S-5108 | Heures de formation à distance déclarables | non |
| S-5109 | Attestations en libre-service | oui |
| S-5110 | Alerte d'échéance des dérogations | 60 jours |
| S-5111 | Contrôle 25 % bloquant (M2) | oui |
| S-5112 | Rappel de campagne SOLTéA | 1er octobre |
| S-5113 | (personas) Aperçu par l'apprenti avant envoi | 24 h |
| S-5114 | (personas) Durée du lien employeur | 30 jours |
| S-5115 | (personas) Récapitulatif aux titulaires d'un apprenti mineur | oui |
| S-5116 | (personas) Rappel d'inscription parallèle (CPGE) | 15 octobre |
| S-5117 | (personas) Code d'accès du maître d'apprentissage par SMS | non |
| S-5118 | (personas) Récapitulatif groupé par employeur | non |

---

## 29. Faits, hypothèses, contradictions

- **Faits [F]** : fichiers cités, relevés le 2026-11-11 sur `origin/ent-mc0hoe` ; rien compilé ni exécuté.
- **Hypothèses [H]** : le lycée pilote a une UFA rattachée à un CFA (M1) ; pratiques des colles, des visites, du livret ; stage de BTS sans objet pour l'apprenti.
- **Contradictions signalées**
  - `02_ELEVE.md` l. 340 place `statut_apprenant` sur l'inscription annuelle « [H] à préciser » ; ce socle le **date** (§7) ; la colonne devient une projection puis est retirée.
  - `07_DOCUMENTS.md` l. 187 rattache le certificat de réalisation au socle **39** ; ce socle le produit (§15) : il appartient au **51** (apprentissage), le 39 garde les stages. À corriger dans la carte des documents.
  - `09_INTEROP.md` FX-42 range SIFA au socle 44 (« 44 / X7 ») : probable coquille pour **51** (le 44 est « ressources matérielles »).
  - `SOCLES.md` §51 cite « licences professionnelles en lycée (semestres, crédits) » : non traitées ici au-delà du modèle de crédits (rares, partenariat universitaire) ; à rouvrir si Florian en a.
  - Le rapport de recherche initial (sous-agent) donnait des articles de mémoire ; seuls ceux vérifiés au §5 sont retenus.
