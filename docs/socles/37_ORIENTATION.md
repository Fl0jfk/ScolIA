# Socle 37 — Orientation et parcours

**Date :** 2026-11-11 — **Statut :** conception de fond, **non décidée** (à relire par Florian) ; recommandations appliquées par défaut (D-031), listées au §25. Aucun code écrit ni modifié ; aucun accès à une base, à un secret ni à un service de production. Code Next.js **lu seulement**, de façon ciblée (CLAUDE.md).
**Branche :** `ent-socle-37-38-conception`, partie de `origin/ent-mc0hoe`.
**Passe personas :** `docs/socles/PERSONAS_37.md` ; ajouts faits **ici**, marqués **« ajout personas »** (§28) ; en cas de contradiction, le §28 l'emporte ; rien n'est supprimé.
**À lire avec :** `CLAUDE.md` ; `DECISIONS.md` D-017, D-021 à D-036, D-037 à D-046 (E1–E14) ; `SOCLES.md` §37 ; **`23_BULLETINS.md`** (§4.1, §5 R-B07 à R-B17, §9, §21 « orientation », Q-BU1) ; `21_EVALUATIONS.md` (R-17, R-31, R-V37) ; `22_COMPETENCES.md` ; `09_INTEROP.md` (FX-05, FX-35, FX-36, X0, X7, D-2) ; `19_AGENDA_RDV.md` (entretiens, AGD-341) ; `13_DISCIPLINE.md` (M-1344, CD-49) ; `03_INSCRIPTIONS.md` (vœux internes, l. 516) ; `TRANSITION_T0_INVENTAIRE.md` (M10, M17) ; `IA_COEUR.md` §5.
**Conventions** : **[F]** fait vérifié dans le dépôt (chemin:ligne) ; **[V]** règle lue sur source officielle ; **[V-r]** règle lue **en résumé de moteur de recherche** pointant vers une page officielle (les sites education.gouv.fr, eduscol, parcoursup.gouv.fr ont renvoyé **403** à la lecture directe le 2026-11-11) : sens fiable, lettre à relire avant de coder ; **[V*]** repris d'une autre conception qui l'a vérifié (référence citée) ; **[NV]** non vérifié, **interdit de coder dessus** ; **[H]** hypothèse de terrain ; **[R]** recommandation.
**Numérotation (préfixes vérifiés libres le 2026-11-11 sur `docs/`) :** actions **ORI-100 à ORI-399** (ORI-01 à 06 de `SOCLE2_DOMAINES.md` repris par renvoi) ; règles **R-OR1…** ; réglementaire **RG-O01…** ; cas limites **COR-01…** ; défauts Rust **OD1…** ; questions **Q-OR1…** ; réglages **S37-01…** ; points pour Florian **F37-1…** ; lots **OR0…OR13** ; migrations **2740–2759** (plage imposée par le pilote, vérifiée libre sur les 134 branches distantes).

---

## 0. En bref

1. **La frontière est déjà tracée par le socle 23** (Q-BU1, tranchée et appliquée) : le **circuit de décision** (fiche de dialogue reprise de Next.js, demandes, propositions du conseil, décision du chef d'établissement, appel, maintien) appartient au **23**. Le **37 possède tout le reste du chemin** : ce qui prépare la décision (parcours Avenir, exploration, entretiens, carnet de l'élève), ce qui la **prolonge vers l'extérieur** (Affelnet, Parcoursup, fiche Avenir, inscriptions dans le lycée ou la formation d'accueil), et ce qui **suit l'élève après** (résultats d'affectation et d'admission, élèves sans solution, devenir, réorientation). Le 37 **lit** les vœux et décisions du 23 ; il ne les écrit jamais (§4). La consigne de lancement plaçait les fiches de dialogue au 37 : contradiction signalée (C1, §27), tranchée en faveur du 23 pour ne pas dédoubler un circuit déjà conçu.
2. **Thèse** : l'orientation n'est pas « une saisie de vœux en mars » ; c'est **un parcours de la 5ᵉ à l'après-bac**, fait d'activités, d'échanges et de choix datés, qui se joue **à trois** (élève, famille, équipe) et **avec l'extérieur** (applications nationales, lycées, supérieur). Le logiciel doit faire gagner du temps au professeur principal et au secrétariat (listes prêtes, relances ciblées, rien de ressaisi), **ne laisser aucun élève sans réponse** en juillet, et ne jamais décider à la place des personnes (§1).
3. **Le réglementaire** [§5] : procédure d'orientation et privé sous contrat (D. 331-23 et suivantes, D. 331-46 et suivantes) **vérifiée par le 23** [V*] ; **plan Avenir** à partir de 2025-2026 (BO n° 27 de 2025) : plan pluriannuel d'orientation par établissement, **quatre demi-journées annuelles**, plateforme **Avenir(s)** de l'Onisep de la 5ᵉ à la terminale, **rencontres parents-élève-professeurs systématiques en 3ᵉ et en 2ᵈᵉ** [V-r] ; calendrier **Parcoursup 2027** (vœux du 18 janvier au 11 mars, confirmation jusqu'au 1ᵉʳ avril, **fiche Avenir jusqu'au 2 avril**, phase principale du 1ᵉʳ juin au 10 juillet, complémentaire jusqu'en septembre) [V-r] ; remontée des bulletins Parcoursup **réservée aux éditeurs certifiés** [V-r] ; **Affelnet lycée** (arrêté du 24 novembre 2023 [V-r]) : saisie dans l'application nationale, **aucune interface éditeur trouvée** [NV], vœux vers le privé sous contrat doublés d'une démarche directe [V-r] ; règlement européen sur l'IA : orientation = **haut risque** [V* R-B16].
4. **Existant Rust** [F] : rien qui porte le parcours ; des **vœux génériques** (`ent_voeu_inscription`, 077) que l'avis du conseil (181) réutilise comme vœux d'orientation ; un **export « Parcoursup » en CSV maison** ; une mission « référent orientation » (355). **Lot OR0** (2 défauts propres au 37, les autres sont déjà au lot B0 du 23 et au lot X0 du 9) : les familles et l'élève **écrivent des vœux à tout moment**, hors de toute campagne, sans le second titulaire ni journal (OD1) ; les événements de vœux sont **publiés hors de la transaction** (OD2) (§2).
5. **Existant Next.js** : la fiche de dialogue (M10) est reprise par le **23** (NXF-01 à 16). Le 37 reprend du module **« invitations » (M17)** la **situation déclarée des anciens élèves** (études, emploi, établissement fréquenté) pour le **devenir** — et relève un **défaut de production grave** à signaler à Florian (ND-01, §2.3) : la page publique de la cérémonie renvoie, à qui connaît **le nom et le prénom** d'un lauréat, l'e-mail du parent et la situation actuelle de l'élève.
6. **Ce que fait le 37** (§7 à §14) : plan d'orientation de l'établissement et activités (forums, interventions, visites, immersions, mini-stages, salons, semaines de l'orientation) avec participation et traces ; **entretiens** (professeur principal, PsyEN, référent, rencontres obligatoires de 3ᵉ et 2ᵈᵉ) ; **carnet de parcours** tenu par l'élève, partagé à son choix ; **Affelnet** : liste de saisie prête, contrôle de cohérence avec la décision du 23, suivi des saisies (famille ou établissement), résultats par tour, **élèves non affectés** suivis jusqu'à une solution, **inscription effective** au lycée d'accueil vérifiée ; **Parcoursup** : accompagnement des vœux, **fiche Avenir préparée dans ScolIA** (brouillons du PP, avis du chef d'établissement par vœu, contrôle de contenu) puis **reportée** dans Parcoursup tant que la certification manque, suivi des propositions de juin et des **élèves sans proposition** (CAES), élève majeur ; **devenir N+1** ; **réorientation** en cours d'année ; pilotage.
7. **IA** (§16) : explique une formation, résume les activités d'un élève **pour lui**, prépare une liste de relances, rédige un brouillon de compte rendu d'entretien relu ; **jamais** une recommandation de formation pour un élève, un avis de fiche Avenir ni un classement (haut risque, R-B16).
8. **Sécurité et minimisation** (§17) : la PsyEN garde son dossier (socle 36) ; le 37 ne montre qu'**un fait d'entretien** que la psychologue choisit de partager ; un professeur voit l'orientation des **seuls élèves de ses groupes** (D-022) ; la fiche Avenir ne cite **jamais** sanction effacée, santé, dispositif ou accompagnement (M-1344, R-V37) ; aucun export national sans administratif (D-022).
9. **Chiffres** : **166 actions** (130 dans la conception, ORI-100 à ORI-362, et 36 ajoutées par la passe personas, ORI-364 à ORI-399), **52 cas limites**, **32 règles** (dont 4 de la passe personas), **14 lots** (OR0 à OR13), **16 migrations** planifiées dans 2740–2759 (2740–2754, 2757), 3 en réserve (2755, 2756, 2758), 2759 pour les retraits.
10. **Pour Florian** (§25.2) : **ND-01 en production** (Next.js, à corriger par lui) ; certification Parcoursup et inscription éditeur (F-06) ; calendriers académiques Affelnet ; PsyEN ou CIO pour le privé ; DPO (durées, devenir des anciens) ; juriste (IA haut risque).

---

## 1. Thèse : un parcours de six ans, des choix datés, aucun élève sans réponse

**Ce que vivent les équipes aujourd'hui [H, expérience de terrain, `PERSONAS_23.md`]** :
- le professeur principal de 3ᵉ tient un **tableau Excel** des vœux, recopie les vœux de la fiche de dialogue dans Affelnet ou vérifie que les familles l'ont fait, relance au téléphone ;
- le secrétariat découvre **fin juin** qu'un élève n'est affecté nulle part, ou qu'il est affecté mais **ne s'est jamais inscrit** au lycée d'accueil ;
- en terminale, deux professeurs principaux se partagent 35 fiches Avenir, recopient des appréciations déjà écrites au bulletin, et le chef d'établissement doit **donner un avis par vœu** pour des centaines de vœux en une semaine ;
- les activités d'orientation (forum, intervention d'un ancien, visite d'un salon) **ne laissent aucune trace** exploitable : impossible de dire en juin quel élève n'a rien vu ;
- l'élève **perd son fil** entre la 4ᵉ et la terminale : chaque année, on repart de zéro.

**Ce que redoutent les acteurs [H]** : un élève **sans affectation à la rentrée** ; une fiche Avenir qui **cite une sanction** ou une difficulté de santé ; un avis du chef d'établissement **incohérent** avec la décision du conseil ; un parent séparé qui **apprend l'orientation après coup** ; un élève majeur dont les parents exigent de voir les vœux ; une IA qui « conseille » une filière.

**Ce qui les ravirait [H]** : la **liste Affelnet prête**, triée, avec ce qui manque ; un **tableau de juin** « qui n'a pas encore de solution » ; la **fiche Avenir pré-remplie** depuis les appréciations validées, à relire et non à écrire ; un **fil de parcours** que l'élève retrouve en terminale ; un **compte rendu d'entretien** en deux clics ; savoir **ce que sont devenus** les élèves l'année suivante sans enquête papier.

**Principes [R]** :
1. **Le 23 décide, le 37 accompagne et relie à l'extérieur** : aucune décision d'orientation n'est prise ni modifiée par le 37.
2. **Rien ne se ressaisit** : les vœux définitifs de la fiche de dialogue (23) préparent la liste Affelnet ; les appréciations validées (21, 23) préparent la fiche Avenir ; les affectations reçues préparent l'inscription (3).
3. **Aucun élève sans réponse** : chaque élève d'un palier a, à chaque date clé, un **état** (« vœux saisis », « affecté », « inscrit », « sans solution — suivi par … ») ; personne n'est perdu en silence.
4. **Le national reste national** : ScolIA ne remplace pas Affelnet, Parcoursup ni Avenir(s) ; il prépare, contrôle, trace et suit ; il n'échange directement avec eux **qu'après** l'inscription et la validation éditeur (D-029, D-2 du 9).
5. **L'élève est auteur de son parcours** : son carnet lui appartient ; il choisit ce qu'il partage ; majeur, il décide seul.
6. **Minimisation** : on trace des **faits** (« a participé au forum », « entretien tenu le … »), pas des jugements ; la psychologie et la santé restent dans leurs socles.

---

## 2. Évaluation franche de l'existant

### 2.1 Rust : ce qu'il y a [F]

| Élément | Où | Constat | Sort |
|---|---|---|---|
| Filières | `migrations/077_filiere_voeu.sql` (`ent_filiere` : code, libellé, MEF) ; `ent-db/src/filiere_repo.rs` ; route `/filieres` (`ent-api/src/routes.rs:578`) | catalogue interne libre | rattaché au **socle 5** (offre) et au **23** (`orn_destination`) ; le 37 ne l'utilise pas comme catalogue de formations externes |
| Vœux | `ent_voeu_inscription` (077 : rang 1 à 5, `annee_scolaire TEXT`, FK simples, `ON DELETE CASCADE`) ; `poser_voeu_for_actor`, `retirer_voeu_for_actor` (`service.rs:14780-14910`) ; routes `/eleves/{id}/voeux` (`routes.rs:580-583`) | **vœu générique** : sert aux vœux internes d'inscription (socle 3, l. 516) **et** aux vœux d'orientation lus par le conseil (`evaluations_conseil.rs:582-620`) | **scindé** : 077 reste aux vœux internes (3) ; l'orientation passe aux `orn_voeu` du 23 (lot B8) ; le 37 **lit** le port du 23 |
| Avis du conseil | `ent_orientation_avis` (181) ; `avis_voeu_for_actor`, `repondre_avis_for_actor` (`evaluations_conseil.rs:721-920`) | propriété du **23** (BD5, BD7) | lot B0 puis B8 du 23 |
| Export « Parcoursup » | `domain-eleve/src/parcoursup_export.rs` (en-tête « Parcoursup / APB — CSV local ») ; `service.rs:15211-15270` ; route `/exports/parcoursup` (`routes.rs:596`) | CSV maison `ine;nom;prenom;classe;formation;rang_voeu;moyenne`, moyenne **toujours vide**, une ligne par élève même sans vœu ; droit `exports.officiels` (professeur refusé, `tests_d022.rs:368-383`) | lot **X0 du 9** : renommé « export de travail (non officiel) » ; format officiel au lot OR12 (bloqué D-2) |
| Mission | `355_missions.sql:99` : `referent_orientation` (portée établissement ou niveau) | bonne base | **consommée** (§3, droits §17) |
| Psychologue | `domain-eleve/src/psy.rs` (motif « orientation »), `psy_pdf.rs` (courrier d'orientation de la psychologue) | dossier du **socle 36** | lien seulement (§9.3) |
| Écrans | Leptos : `web/crates/ent-web-app/src/modules/evaluations/{conseil,famille}.rs` (avis et réponse, 23) ; `ui.rs` : 159 occurrences « orientation/voeu » (ancienne UI figée, F-001) | aucun écran de parcours | refaits en Leptos (lots OR2 et suivants) |

### 2.2 Rust : défauts propres au 37 (lot OR0)
Les défauts du circuit (cascade vœu → avis BD5, décision sans destination BD7, droits du conseil BD6) sont **au lot B0 et B8 du 23** ; les exports maison au **lot X0 du 9**. Restent :

| # | Défaut | Conséquence | Où [F] | Gravité | Correction (OR0) |
|---|---|---|---|---|---|
| **OD1** | Un **parent ou l'élève écrit des vœux à tout moment** (`poser_voeu_for_actor` : rôle parent avec lien, ou élève lié), **remplace** un vœu existant (`upsert_voeu`), le **supprime** (`retirer_voeu_for_actor`), sans campagne, sans étape ouverte, sans le second titulaire, sans journal | un vœu modifié la veille du conseil sans que l'équipe ni l'autre parent ne le sachent ; un élève mineur modifie seul les vœux de la famille | `service.rs:14780-14835`, `14876-14910` ; `filiere_repo.rs:100-160` | **important** | écriture des vœux par la famille et l'élève **fermée** hors du circuit du 23 (refus explicite « la saisie des vœux passe par la fiche de dialogue ») ; le personnel garde l'écriture des **vœux internes** (socle 3) ; journal `ent_journal_modif` pour chaque écriture restante (E7) |
| **OD2** | Événements `voeu`, `filiere` publiés par `self.events.publish` **après** l'écriture, hors de la transaction | un vœu écrit sans événement (ou l'inverse) si le processus tombe entre les deux ; règle de la toile (`SOCLES.md` §5) non tenue | `service.rs:14749-14765`, `14818-14833` | important | écriture et publication dans **une seule** transaction (`tx.publier`, comme `evaluations_conseil.rs:796`) |

**Non retenu comme défaut** : la portée professeur sur la lecture des vœux (`exiger_perimetre_prof`, `service.rs:14845`) est déjà appliquée [F] ; elle sera recalée sur la portée au groupe (D-022) par le 23 et le 37 (R-OR4).

### 2.3 Next.js : ce qui touche l'orientation [F, lecture seule et ciblée]
- **Fiches de dialogue (M10)** : inventaire et reprise au **23** (§2.3 du 23 : NXF-01 à NXF-16, NXFD-1 à NXFD-12). Le 37 n'en reprend **aucune action** ; il consomme le résultat (vœux définitifs, décision) par port (§4.3).
- **Invitations à la cérémonie de remise des diplômes (M17)** : `drizzle/0054_invitation_ceremonie.sql`, `0057_invitation_eligible_situation.sql` ; `app/lib/invitation-db.ts`, `app/api/invitation/**`. Une page publique (`slug`) par cérémonie ; liste d'« éligibles » (prénom, nom, date de naissance, diplôme bac ou brevet) **importée par copier-coller ou Excel** ; réponse des familles (présence, nombre de personnes, e-mail du parent) ; **situation de l'ancien élève** demandée selon le réglage `ask_situation` (`off`, `always`, `bac_only`) : statut `etudes` / `emploi` / `autre`, détail libre, **établissement fréquenté** (`invitation-types.ts:15-19`, `0057:11-17`). La **cérémonie** relève du **38** (§20.11 du 38) ; la **situation** est la seule source actuelle du **devenir** des élèves : reprise ici (ORI-330 à 336).

**Défauts Next.js relevés (à ne pas reproduire ; Next.js est intouchable, signalés à Florian)** :

| # | Constat | Où [F] | Gravité | Dans ScolIA |
|---|---|---|---|---|
| **ND-01** | **Fuite par la page publique** : `POST /api/invitation/lookup` (sans compte) accepte prénom, nom et date de naissance et retient la meilleure correspondance avec **au moins 2 critères exacts sur 3** (`invitation-db.ts:688-698`, `pickBestIdentityMatch(…, 2)`) ; **prénom et nom suffisent donc**, la date pouvant être fausse. La réponse contient, si une réponse existe déjà, **l'e-mail du parent**, la **situation** et l'**établissement fréquenté** de l'ancien élève, et l'**orthographe exacte** du nom enregistré (`lookup/route.ts:21-37`, `invitation-db.ts:713-722`). Avec la liste des éligibles, elle **confirme qu'un élève est lauréat** (bac ou brevet) | `app/api/invitation/lookup/route.ts` ; `app/lib/invitation-db.ts:645-750` | **grave** (données personnelles d'élèves et de parents exposées à quiconque connaît un nom ; résultat d'examen révélé sans consentement) | **F37-1** : à corriger **par Florian** dans Next.js (exiger les trois critères, ne jamais renvoyer l'e-mail ni la situation, limiter le débit) ; dans ScolIA, aucune page publique ne renvoie de donnée personnelle : la réponse se fait **dans le compte** ou par **lien nominatif** du socle 7 (R-OR22) |
| ND-02 | Identité par **nom normalisé** (`eleve_name_norm`) et groupes de doublons à résoudre à la main | `0054:38-41` | moyen | identité ScolIA de l'élève (socle 2) |
| ND-03 | Situation en **texte libre** sans date ni source ; mélange « établissement fréquenté » et « détail » | `0057:13-17` | mineur | devenir **codé** (§13), daté, déclaré par qui, établissement par UAI quand connu |
| ND-04 | Aucune durée de conservation ; données de l'ancien élève gardées sans limite | T0 l. 121 (« perdable après l'événement ») | moyen | conservation déclarée (§17.4) |

### 2.4 Ce qu'il ne faut pas perdre (D-035, point 5)
- Le recueil de la **situation des anciens élèves** à l'occasion d'un événement convivial (taux de réponse élevé [H]) : repris, avec consentement et minimisation (ORI-332).
- Le choix **« demander seulement aux bacheliers »** (`bac_only`) : repris comme réglage (S37-05).

### 2.5 Charlemagne et Pronote [H, `MATRICE_CHARLEMAGNE.md` §3.7]
Les logiciels du marché couvrent les **vœux de la fiche de dialogue** et les **exports** (Parcoursup pour les éditeurs certifiés) ; ils ne suivent ni le **parcours** (activités, entretiens), ni les **résultats d'affectation**, ni le **devenir** [H, à confirmer par Florian]. Le marché laisse les « sans solution » de juillet au téléphone et au tableur.

### 2.6 Verdict
**Presque tout est à construire** ; deux correctifs immédiats (OR0) ; une page de production à corriger par Florian (ND-01).

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps | Ce qu'il redoute | Ce qui le ravirait | Quand |
|---|---|---|---|---|---|
| **Chef d'établissement** (et adjoint) | arrête le plan d'orientation ; **avis par vœu** sur la fiche Avenir ; signe ; répond des « sans solution » | des centaines d'avis en une semaine ; incohérences entre avis et décision | un avis mal relu ; un élève sans affectation | avis préparés depuis la décision du conseil, **cohérence vérifiée** ; tableau « sans solution » | toute l'année ; mars–avril ; juin–septembre |
| **Professeur principal** (3ᵉ, 2ᵈᵉ, 1ʳᵉ, terminale ; deux en terminale [H]) | entretiens, suivi des vœux, appréciation de la fiche Avenir, relances | tableaux recopiés ; relances une à une ; doublons avec le bulletin | oublier un élève ; écrire une appréciation contradictoire avec le bulletin | listes prêtes, relance en un geste, fiche Avenir pré-remplie | mi-novembre à juillet |
| **Référent orientation** (mission 355) | construit le plan, organise forums et interventions, coordonne | invitations, listes de présence, salles | une activité où personne ne vient ; un niveau oublié | calendrier, inscriptions, présences, bilan | toute l'année |
| **Psychologue de l'Éducation nationale** (public) ; **conseiller d'orientation** ou CIO (privé [H]) | entretiens individuels, bilans, participation au conseil | demandes d'entretien par papier ; aucune vue du parcours | voir son dossier exposé | demandes structurées, faits partagés à son choix | toute l'année |
| **Secrétariat de scolarité** | listes Affelnet, saisies pour les familles qui ne le font pas, résultats, inscriptions des affectés, liste des non-inscrits | ressaisie ; relances ; listes de juillet | une saisie oubliée ; une inscription non faite | **liste de saisie** et **état par élève** | mai–septembre |
| **CPE, vie scolaire** | repère les décrocheurs, alerte | aucun lien entre absentéisme et orientation | un élève qui disparaît en juin | alertes croisées (12, 36) | toute l'année |
| **Professeurs** | interventions, appréciations, découverte des métiers | — | — | voir le parcours de **ses** élèves (D-022) | — |
| **Professeur documentaliste (CDI)** | espace orientation, ressources | — | — | ressources reliées aux activités (45) | — |
| **Famille** (titulaires de l'autorité, séparés ou non) | accompagne, saisit (téléservices), assiste aux rencontres | ne pas savoir quoi faire ni quand | rater une date ; découvrir une décision ; dossier Parcoursup illisible | un **calendrier personnel**, « ce qui reste à faire », une seule fois pour les deux parents | 3ᵉ, 2ᵈᵉ, terminale |
| **Parent sans exercice** de l'autorité | informé des choix importants [V* R-B15] | — | être écarté | information, pas d'écriture | — |
| **Élève mineur** | explore, participe, tient son carnet, donne son avis (23), formule ses vœux Parcoursup | refaire chaque année ; tout est papier | être orienté sans être entendu | son **fil de parcours**, ses traces, son calendrier | 5ᵉ–terminale |
| **Élève majeur** | décide seul (Parcoursup, réorientation) | parents qui exigent de voir | — | maîtrise du partage | terminale, post-bac |
| **Élève en situation de handicap, AESH** | aménagements, besoins de l'élève dans le choix (35) | formations « accessibles » mal connues | — | informations du référent handicap (35), sans exposer le diagnostic | — |
| **Élève allophone, SEGPA, ULIS, UPE2A** | parcours adaptés, voies spécifiques | — | être cantonné | destinations adaptées listées (23) | — |
| **Lycée ou formation d'accueil** | reçoit l'élève affecté | dossier incomplet | — | inscription via le socle 3 | juillet |
| **Intervenants extérieurs** (professionnels, anciens élèves, entreprises, associations) | interviennent | — | — | inscription simple, **sans compte**, sans voir les élèves | — |
| **Inspection, DASEN, CIO** (public) | commissions, suivi des sans-solution | listes incomplètes | — | liste à jour, transmise par un administratif | juin–septembre |
| **Auditeur RGPD / DPO** | contrôle | — | profilage, fuite | registre, durées, journal | — |

---

## 4. Frontières

### 4.1 Tableau

| Sujet | 37 | Autres |
|---|---|---|
| Procédure d'orientation et de passage, fiche de dialogue, destinations, choix des spécialités, appel, maintien | **lit** | **23 possède** (Q-BU1) |
| Plan d'orientation de l'établissement (PPO), activités, participation | **possède** | 19 (salles, créneaux), 10 (séances touchées), 12 (présence), 17 (annonce), 45 (ressources) |
| Entretiens d'orientation (PP, référent, rencontres de 3ᵉ et 2ᵈᵉ) | **possède** le motif, le compte rendu minimal, le suivi | **19** porte le créneau et le rendez-vous ; **36** possède les entretiens et le dossier de la **psychologue** |
| Carnet de parcours de l'élève | **possède** | 22 (compétences du parcours Avenir si l'établissement les évalue) |
| Stages d'observation (3ᵉ, 2ᵈᵉ), mini-stages, immersions | **affiche** comme activités du parcours | **39 possède** convention et période ; 12 (absence attendue) |
| Catalogue des formations **externes** (formation × établissement d'accueil) | **possède** (référence) | 9 charge l'annuaire de l'éducation (UAI) et la BCN ; 23 garde ses **destinations** (voie, série) |
| Vœux d'affectation Affelnet, résultats, inscription effective | **possède** (préparation, suivi) | application nationale (saisie, barème) ; **3** inscrit les affectés entrants ; 9 transporte (si interface un jour) |
| Moyennes annuelles de 3ᵉ pour Affelnet | — | **21** calcule, **22**/LSU transmet (FX-36) |
| Parcoursup : accompagnement, vœux déclarés, fiche Avenir, suivi des propositions | **possède** | **21, 23** fournissent les appréciations ; **9** transporte la remontée (bloquée D-2) |
| Résultats aux examens transmis à Parcoursup (EAM, Pix) | lit | **38** |
| Décrochage, groupe de prévention, signalements | lit et **alerte** | **36** possède (GPDS, cellule de veille) |
| Devenir des élèves N+1 | **possède** | 38 (cérémonie : moment de recueil), 47 (indicateurs) |
| Réorientation en cours d'année, passerelles | **possède** l'accompagnement | **23** ouvre la procédure hors calendrier ; 3 et 5 changent la scolarité |
| Bourses au changement de cycle | — | **26** |
| Notifications | déclare ses natures | **8** |

### 4.2 Ports exposés par le socle 37 [R]
- `parcours::fil(tx, eleve) -> FilParcours` (activités, entretiens tenus, carnet partagé) — 2 (fiche 360°), 23 (dossier du conseil, ligne « parcours »), 29.
- `affectation::etat(tx, eleve, annee) -> EtatAffectation` (`sans_voeu`, `voeux_prets`, `saisi`, `affecte`, `liste_supplementaire`, `non_affecte`, `inscrit`, `sans_solution`) — 23, 3, 36, 47.
- `admission::etat(tx, eleve, annee) -> EtatAdmission` (Parcoursup) — 23, 36, 47.
- `orientation::sans_solution(tx, annee) -> [EleveSuivi]` — 36, 47, direction.
- `devenir::situation(tx, eleve, annee) -> Option<Situation>` — 47.
- `fiche_avenir::preparee(tx, eleve) -> FicheAvenirPreparee` — 9 (remontée, lot OR12).

### 4.3 Ports consommés [R]
- **23** : `orientation::voeux_definitifs(eleve, annee)` (port demandé par `PERSONAS_23.md` M23-021, même rang, même destination) ; `passage::decision(eleve, annee)` ; `bulletins::publie(eleve, periode)` ; avis du conseil sur les vœux Parcoursup du 2ᵉ trimestre [V-r] (lecture de l'examen du conseil).
- **21** : appréciations **validées** par cours et moyennes annuelles (fiche Avenir, Affelnet).
- **22** : compétences du parcours Avenir, attestations (Pix lu au 38).
- **5** : niveaux, MEF, classes, groupes, offre interne, **jours ouvrables** et calendrier.
- **6** : professeurs principaux, référent orientation (mission 355), PsyEN.
- **4** : titulaires, droits du lien (`recevoir_*`), parent sans exercice, interdictions, majorité.
- **2** : élève, situation (handicap : drapeau seulement via 35), sortie.
- **19** : créneaux, rendez-vous, rencontres collectives.
- **39** : conventions de stage d'observation et périodes.
- **36** : fait « entretien PsyEN tenu » **partagé** par la psychologue ; drapeau « suivi GPDS » (sans contenu).
- **38** : résultats officiels (DNB, EAM, bac), certification Pix.
- **9** : annuaire de l'éducation (UAI, statut public/privé), BCN (formations), fichiers d'affectés (FX-05).
- **7, 8** : documents émis (comptes rendus, attestations de participation), avis.

---

## 5. Ce qui est réglementaire (vérifié le 2026-11-11)

**Méthode et limites** : deux recherches sur sources officielles le 2026-11-11. education.gouv.fr, eduscol et parcoursup.gouv.fr **refusent la lecture directe** (403) ; Légifrance renvoie des résumés. D'où le statut **[V-r]** (résumé pointant vers la page officielle citée) : le **sens** est fiable, la **lettre** (dates, champs, libellés) est à relire avant de coder. La procédure d'orientation, déjà vérifiée par le 23 sur Légifrance, est reprise **[V*]**.

| # | Règle | Source (consultée le 2026-11-11) | Statut | Conséquence pour ScolIA |
|---|---|---|---|---|
| RG-O01 | **Procédure d'orientation (public)** : demande de la famille, propositions du conseil, avis de l'élève mineur recueilli, décision du chef d'établissement, entretien en cas de désaccord, décision motivée, 3 jours ouvrables, commission d'appel, maintien de droit | C. éduc. D. 331-31 à D. 331-37 — 23 R-B07 | [V*] | **propriété du 23** ; le 37 lit la décision et les vœux définitifs |
| RG-O02 | **Privé sous contrat** : procédure miroir, commission d'appel propre | D. 331-46 à D. 331-61 — 23 R-B08 | [V*] | idem ; le 37 traite public et privé ensemble (D-019) |
| RG-O03 | **Parent sans exercice** de l'autorité : informé des choix importants | C. civ. 373-2-1 — 23 R-B15 | [V*] | reçoit les **informations** du 37 (calendrier, résultats d'affectation), n'écrit rien |
| RG-O04 | **Plan Avenir** (à partir de 2025-2026) : **plan pluriannuel d'éducation à l'orientation** par établissement ; **quatre demi-journées annuelles** dédiées ; plateforme **Avenir(s)** (Onisep) dans tous les collèges et lycées dès septembre 2025, de la 5ᵉ à la terminale, intégrant « Mon projet sup » dès la 2ᵈᵉ ; **rencontres parents-élève-professeurs systématiques pour tous les élèves de 3ᵉ et de 2ᵈᵉ** | BO n° 27 du 3 juillet 2025 (MENE2519127N) — https://www.education.gouv.fr/bo/2025/Hebdo27/MENE2519127N | [V-r] | plan annuel, compteur de demi-journées par niveau (R-OR8), rencontre obligatoire suivie élève par élève (R-OR9) ; **aucune** interface avec Avenir(s) supposée (RG-O16) |
| RG-O05 | Volumes horaires antérieurs (« 54 h » d'accompagnement à l'orientation au lycée, découverte des métiers au collège), arrêté du 1ᵉʳ juillet 2015 (parcours Avenir) | — | [NV] (probablement remplacés par RG-O04) | **ne rien coder** ; le compteur est paramétrable (S37-01) |
| RG-O06 | **Calendrier national des procédures d'orientation et d'affectation** fixé chaque année par note de service (2026 : note du 5 décembre 2025, BO n° 48 du 18 décembre 2025) ; **calendrier 2027 non trouvé** | https://www.education.gouv.fr/bo/2025/Hebdo48/MENE2532981N | [V-r] (existence) ; contenu [NV] | toutes les dates sont des **paramètres de campagne**, par académie, jamais des constantes (R-OR2) |
| RG-O07 | **Téléservices Orientation et Affectation** (Scolarité services, ÉduConnect) : saisie des intentions et des demandes par les familles, accusé des avis ; **fusionnés dans le portail depuis 2025**, mais SIECLE Orientation et Affelnet lycée restent deux traitements distincts | eduscol « La poursuite d'études après la troisième et la seconde GT » — https://eduscol.education.gouv.fr/5394/la-poursuite-d-etudes-apres-la-troisieme-et-la-seconde-generale-et-technologique | [V-r] | dans le **public**, la fiche de dialogue officielle est le téléservice : le 23 doit gérer « procédure tenue dans le téléservice » (report, Q-OR11) ; le 37 suit les dates |
| RG-O08 | **Affelnet lycée** : traitement créé par l'**arrêté du 24 novembre 2023** (remplace celui du 17 juillet 2017, abrogé au 1ᵉʳ janvier 2024) | https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000048736103 | [V-r] (existence) ; barème, bonus, données, avis du chef d'établissement [NV] | aucun barème codé ; le 37 ne calcule **jamais** de rang d'affectation (R-OR13) |
| RG-O09 | **Calendrier Affelnet 2026** : offres consultables dès le 3 avril ; **saisie des demandes par les familles du 4 au 26 mai** ; résultats fin juin ; **second tour** facultatif du 6 au 10 juillet ; saisie **en établissement** selon l'académie (ex. du 4 mai au 9 juin, public et privé sous contrat) | BO n° 48 (ci-dessus) ; Onisep — https://www.onisep.fr/orientation/le-college/apres-la-3-preparer-ses-demandes-d-orientation-et-d-affectation-en-lycee ; pages Affelmap académiques | [V-r] ; 2027 [NV] | paramètres de campagne ; **plusieurs tours** (R-OR14) ; double fenêtre famille / établissement |
| RG-O10 | **Vœu vers un lycée privé sous contrat** : saisi dans la demande **et** démarche directe auprès de l'établissement privé, qui fixe ses modalités d'inscription | Onisep — https://www.onisep.fr/orientation/le-college/affectation-en-lycee-vos-questions-nos-reponses | [V-r] (exemple académique) | un vœu « privé » porte une **démarche** à suivre (R-OR15) ; un lycée privé utilisateur reçoit la candidature par le **socle 3** |
| RG-O11 | **Interface éditeur Affelnet** : aucune trouvée (déjà relevé par le 9, FX-05) | — | [NV] | **pas d'échange** ; liste de saisie et report manuel (R-OR12) |
| RG-O12 | **Parcoursup 2027** : saisie des vœux du **18 janvier au 11 mars 2027** ; confirmation jusqu'au **1ᵉʳ avril 2027** ; phase principale d'admission du **1ᵉʳ juin au 10 juillet 2027** ; phase complémentaire jusqu'en septembre (9 ou 7 septembre selon la source : écart) | https://www.education.gouv.fr/reussir-au-lycee/baccalaureat-brevet-cap-parcoursup-le-calendrier-2027-341384 ; https://www.parcoursup.gouv.fr/calendrier | [V-r] | paramètres ; écart signalé (C3) |
| RG-O13 | **Fiche Avenir** : appréciations du **professeur principal** et **avis du chef d'établissement** (par vœu) renseignés **jusqu'au 2 avril 2027** ; consultables par l'élève **à partir du 1ᵉʳ juin** ; non obligatoire pour confirmer ; le **conseil de classe du 2ᵉ trimestre examine les vœux** ; champs : méthode de travail, autonomie, capacité à s'investir, engagement [V* 21 R-31] ; libellés exacts de l'avis du chef [NV] | https://www.parcoursup.gouv.fr/faq/thematiques/autres-sujets/fiche-avenir ; PDF 2026 — https://www.parcoursup.gouv.fr/sites/default/files/database/documents/2026-03/la-fiche-avenir-dans-la-proc-dure-parcoursup-1146.pdf | [V-r] / [V*] / [NV] | fiche **préparée** dans ScolIA, **figée** au 2 avril (paramètre) ; **masquée** à l'élève et à la famille dans ScolIA **avant** la date de consultation nationale (R-OR18) ; échelle de l'avis = **référentiel chargé** depuis le texte de l'année, jamais en dur |
| RG-O14 | **Remontée des bulletins Parcoursup** : le chef d'établissement déclenche depuis le **logiciel de notes** la remontée automatique des moyennes et appréciations ; éditeurs **certifiés** depuis 2022 ; remontent moyennes par discipline, rang, appréciations, **pas les absences** ; concaténation automatique des appréciations T1/T2 supprimée | parcoursup.gouv.fr (FAQ, PDF fiche Avenir) ; mémento eduscol 2025 — https://eduscol.education.gouv.fr/sites/default/files/document/2025-memento-cadrereglementaire-evaluationsbacpdf-113592.pdf | [V-r] ; procédure de certification et cahier des charges [NV] | **bloqué** (D-2 du 9, D-032 : Charlemagne l'an 1) ; ScolIA **pré-contrôle** (R-OR19) ; pas d'absences dans la préparation |
| RG-O15 | **Parcoursup, données** : candidats conservés 2 ans en base active puis 4 ans en archive intermédiaire ; traces d'accès 1 an ; délibérations CNIL 2018-011 et 2018-119 | https://www.parcoursup.gouv.fr/faq/thematiques/autres-sujets/utilisation-et-protection-des-donnees-individuelles ; https://www.legifrance.gouv.fr/cnil/id/CNILTEXT000036773197/ | [V-r] | ce sont les durées **du ministère** ; celles de ScolIA sont propres (§17.4) |
| RG-O16 | **Avenir(s)** : plateforme nationale ; aucune interface éditeur trouvée ; remplacement de Folios **non confirmé** | — | [NV] | lien sortant seulement ; aucune donnée échangée |
| RG-O17 | **Règlement européen sur l'IA** : systèmes destinés à **déterminer le niveau d'éducation approprié** ou à **orienter** = haut risque (annexe III, point 3), applicable le 2 décembre 2027 | 23 R-B16 | [V*] | l'IA du 37 ne propose **aucune formation pour un élève**, aucun avis, aucun classement (R-OR24) |
| RG-O18 | **Décision fondée exclusivement sur un traitement automatisé** interdite sauf exceptions (RGPD art. 22) | règlement (UE) 2016/679, art. 22 | [V] (texte connu, non relu ce jour) | aucune règle automatique ne change un état d'orientation d'un élève ; les alertes sont des **signaux** (R-OR24) |
| RG-O19 | **Obligation de formation de 16 à 18 ans** (loi de 2019), missions de lutte contre le décrochage | C. éduc. L. 114-1 [NV, non relu] | [NV] | suivi des « sans solution » de 16–18 ans **signalé**, jamais une transmission automatique à un tiers (R-OR28) |
| RG-O20 | **Rôle de la PsyEN, du CIO, secret professionnel** | — | [NV] | dossier de la psychologue au 36 ; le 37 ne voit que ce qu'elle **choisit** de partager (R-OR21) |
| RG-O21 | **Stage d'observation de fin de 2ᵈᵉ** (obligatoire depuis 2024 [H]) et stage de 3ᵉ | — | [NV] | conventions au **39** ; le 37 les montre au fil du parcours |
| RG-O22 | Réformes annoncées (classe « prépa-seconde », brevet condition d'accès en 2ᵈᵉ, voie professionnelle) | — | [NV] : **non confirmées** sur source officielle le 2026-11-11 | **aucune règle codée** ; destinations ajoutables au catalogue du 23 par l'établissement |

**Non vérifié et interdit de coder dessus** : barème et bonus Affelnet ; libellés de l'avis du chef d'établissement sur la fiche Avenir ; champs exacts de la fiche Avenir 2027 ; calendrier Affelnet 2027 ; certification Parcoursup ; obligations de la PsyEN ; durée de conservation recommandée des dossiers d'orientation par l'établissement (le 23 retient la DUA des « dossiers scolaires », R-B14).

---

## 6. Concepts et vocabulaire [R]

| Terme | Sens | Remarque |
|---|---|---|
| **Plan d'orientation** | plan de l'établissement pour une année (et pluriannuel) : objectifs par niveau, demi-journées, activités | « PPO » [V-r] |
| **Activité** | forum, intervention, visite, salon, immersion, mini-stage, atelier, rencontre, demi-journée dédiée ; datée ; public (niveaux, classes, groupes, volontaires) | type = donnée de l'établissement (préréglage) |
| **Participation** | fait qu'un élève a participé (présent, absent, excusé, inscrit) | pas de note |
| **Entretien d'orientation** | rendez-vous individuel ou familial avec un objet d'orientation | créneau au 19 ; compte rendu minimal ici |
| **Rencontre obligatoire** | rencontre parents-élève-professeurs de 3ᵉ et 2ᵈᵉ [V-r] | suivi élève par élève |
| **Carnet de parcours** | notes et réflexions de l'élève, traces (documents), ses intérêts | l'élève en est l'auteur ; partage à son choix |
| **Formation cible** | formation × établissement d'accueil (UAI) × statut (public, privé sous contrat, hors contrat, apprentissage) | catalogue de référence ; ≠ **destination** du 23 (voie, série) |
| **Campagne d'affectation** | Affelnet pour une année : fenêtres (famille, établissement), tours | dates paramètres |
| **Vœu d'affectation** | rang, formation cible, origine (fiche de dialogue, famille, équipe), état | ≠ vœu d'orientation (23) |
| **Résultat d'affectation** | par tour : affecté, liste supplémentaire (rang), refusé, sans suite | saisi depuis la notification ou un fichier |
| **Inscription effective** | l'élève affecté s'est inscrit dans l'établissement d'accueil | déclarée (famille, accueil) ou vérifiée |
| **Sans solution** | élève sans affectation, sans admission ou non inscrit à la date paramétrée | suivi jusqu'à clôture motivée |
| **Fiche Avenir préparée** | appréciations du PP et avis du chef d'établissement **dans ScolIA**, avant report dans Parcoursup | état « reportée le … » |
| **Vœu Parcoursup déclaré** | vœu que l'élève choisit de déclarer à son équipe (facultatif) | jamais obligatoire |
| **Devenir** | situation de l'ancien élève à N+1 (études, emploi, autre), source, date | codé |
| **Réorientation** | changement de voie en cours ou en fin d'année hors palier | procédure au 23, accompagnement ici |

---

## 7. Le plan d'orientation et les activités

### 7.1 Construire le plan [R]
- Le référent orientation (ou la direction) crée le **plan de l'année** : objectifs par niveau (texte court), **demi-journées** prévues par niveau (préréglage : 4, RG-O04), activités planifiées ; reprise du plan de l'année précédente en un geste (ORI-101).
- Le plan est **pluriannuel** : vue de la 5ᵉ à la terminale « ce que voit un élève sur son parcours » ; trous signalés (« la 4ᵉ n'a aucune activité métiers ») (ORI-104).
- Présenté au conseil pédagogique ou au conseil d'administration [H] : export PDF de présentation (socle 7) (ORI-106).

### 7.2 Une activité [R]
Une activité a : type, titre, description pour les familles, date et créneau (ou période), lieu (salle du 19, ou extérieur), public (niveaux, classes, groupes, ou volontaires avec inscription), encadrants (personnels du 6), intervenants (externes, §7.4), capacité, demi-journée banalisée ou non, documents (7), **séances touchées** (10 : neutralisées ou déplacées, comme une sortie, en lien avec le 42 si sortie de l'établissement).
États : `brouillon` → `publiee` → `tenue` → `bilan` (ou `annulee`, `reportee`).

### 7.3 Participation [R]
- Public imposé : participants **calculés à la date** (élèves de la classe ou du groupe ce jour-là) ; volontaires : inscription par l'élève, validée ou non (capacité, liste d'attente).
- Présence : par l'appel de la séance si elle a lieu sur une séance (12 « en activité ») ; sinon pointage par l'encadrant (mobile) ; absent → le motif d'absence reste au 12.
- Trace dans le **fil de parcours** de l'élève (« a participé au forum des métiers du 14 février »).

### 7.4 Intervenants extérieurs [R]
Annuaire minimal (nom, structure, métier ou domaine, contact professionnel), **sans compte** ; invitation et confirmation par **lien nominatif** du socle 7 ; ils ne voient **aucune donnée d'élève** (seulement le nombre d'élèves et la classe) ; anciens élèves intervenants : seulement s'ils l'ont **consenti** (§13.3).

### 7.5 Salons, portes ouvertes extérieures, immersions [R]
Agenda des **événements extérieurs** (portes ouvertes des lycées et du supérieur, salons) publié aux familles d'un niveau ; **immersion** ou **mini-stage** dans un lycée : demande de l'élève ou du PP, accord de la famille (socle 18 : autorisation), absence attendue (12), convention éventuelle (39) ; retour d'expérience au carnet.

### 7.6 Bilan [R]
Par activité : participants, absents, retours (questionnaire facultatif, socle 18, anonyme par défaut) ; par niveau : demi-journées tenues / prévues, élèves qui n'ont **rien suivi** (liste au PP, ORI-128).

---

## 8. Entretiens et rencontres

### 8.1 Entretien d'orientation [R]
- Demandé par l'élève, la famille, le PP, le référent, la psychologue ; motif d'orientation ; créneau par le **19** ; participants.
- **Compte rendu minimal** (à la saisie : « points abordés », « suites convenues », 1 500 caractères) ; visible : auteur, PP, direction ; famille et élève : **les suites convenues** seulement, si l'auteur les publie (R-OR20).
- Lien avec la procédure du 23 : un entretien **de désaccord** (D. 331-34) est un entretien du **23** (orn_entretien) ; le 37 l'affiche au fil sans le dupliquer.

### 8.2 Rencontres obligatoires de 3ᵉ et de 2ᵈᵉ [V-r, R]
- Pour chaque élève de 3ᵉ et de 2ᵈᵉ : une **rencontre parents-élève-professeurs** attendue dans la période paramétrée ; tableau du PP « fait / proposé / refusé par la famille / impossible (motif) » ; relance ciblée ; rencontre collective possible (réunion par classe, 19) **comptée seulement** pour les familles présentes.
- Parents séparés : une rencontre peut réunir les deux titulaires ou être faite séparément ; chaque titulaire voit **sa** convocation (4).

### 8.3 La psychologue [R]
- Le 37 propose « demander un entretien avec la psychologue » (élève, famille, PP) ; la demande est **transmise au socle 36** ; la psychologue répond dans son outil.
- Le fil de parcours n'affiche que ce qu'elle **partage** (« entretien tenu le … », sans contenu) ; par défaut **rien** (R-OR21).

---

## 9. Le carnet de parcours de l'élève [R]

- **Auteur** : l'élève. Entrées : réflexion libre, intérêt (domaines, métiers), retour d'une activité, document (CV, lettre, attestation de stage), formation repérée (catalogue §10.1).
- **Visibilité par entrée** : `privee` (l'élève seul), `partagee_equipe` (PP, référent, psychologue s'il le souhaite), `partagee_famille` (titulaires) ; préréglage S37-03 : « partagée équipe ».
- **Élève majeur** : il seul décide du partage avec sa famille ; à la majorité, rien de privé ne devient visible (R-OR23).
- **Continuité** : le carnet suit l'élève d'année en année et **à son départ** (export pour lui, socle 27) ; il n'est **jamais** transmis à un autre établissement sans lui (et sans sa famille s'il est mineur).
- **Modération** : aucune lecture des entrées privées par le personnel ; signalement possible par l'élève ; une entrée partagée inquiétante → le PP peut alerter le **36** (geste explicite, jamais automatique).
- **Pas d'IA** sur les entrées privées sans demande de l'élève (R-OR24).

---

## 10. Affelnet : préparer, suivre, n'oublier personne

### 10.1 Catalogue des formations cibles [R]
- Formations **externes** (lycées du secteur, voies pro, CAP, apprentissage, établissements privés) : formation (libellé, code BCN quand chargé par le 9), établissement (UAI, nom, commune, statut via l'**annuaire de l'éducation** [V 09 V#29]), voie, éventuellement « section ou recrutement particulier » ; **ajout libre** par le secrétariat (le catalogue national n'est pas supposé exhaustif).
- Liée à une **destination** du 23 (2ᵈᵉ GT, 2ᵈᵉ pro, CAP…) pour le **contrôle de cohérence**.

### 10.2 Campagne [R]
- Campagne par année : fenêtre **famille** (téléservice), fenêtre **établissement**, date des résultats, tours (1, 2…), dates d'inscription dans l'établissement d'accueil, nombre maximal de vœux (**paramètre**, [NV]).

### 10.3 Vœux d'affectation [R]
- **Préparés** depuis les vœux définitifs du 23 (`orientation::voeux_definitifs`) : même rang, destination proposée → formations cibles à choisir (la fiche de dialogue dit « 2ᵈᵉ GT », Affelnet veut « 2ᵈᵉ GT au lycée X ») ; ou saisis par l'équipe à la demande de la famille, ou déclarés par la famille dans ScolIA (copie de ce qu'elle a saisi au téléservice, **facultatif**).
- **Contrôle de cohérence** avec la décision du 23 (R-OR11) : vœu en 2ᵈᵉ GT alors que la décision est « 2ᵈᵉ pro » → **alerte** au PP et à la direction (jamais blocage : la famille saisit au national) ; vœu sans décision définitive → « en attente de décision ».
- **Liste de saisie** pour le secrétariat (élèves dont la saisie est faite par l'établissement) : élève, INE, rangs, formations (codes), avec ce qui **manque** ; cocher « saisi dans Affelnet le … » (horodaté, par qui).
- **Vœu vers le privé** : démarche directe attendue (RG-O10) → tâche de suivi « dossier déposé au lycée privé ? ».
- **Pièces et procédures particulières** (sections à recrutement particulier, avis du chef d'établissement demandé par l'académie [NV]) : tâche et document (7).

### 10.4 Résultats et tours [R]
- Résultats saisis par l'établissement d'origine depuis la notification nationale ou un **fichier** (tableur, lot OR12 si un format existe) : par élève et par tour ; **liste supplémentaire** avec rang ; « non affecté ».
- **Second tour** : élèves concernés listés ; nouveaux vœux préparés.
- Les familles voient le résultat dans ScolIA **après** la date nationale de publication (paramètre) (R-OR16).

### 10.5 Inscription effective et sans-solution [R]
- Pour chaque affecté : **inscription dans l'établissement d'accueil** attendue avant la date paramétrée ; si l'accueil est un établissement ScolIA, l'information vient du **socle 3** (événement `inscription.confirmee`) ; sinon, déclarée par la famille ou le secrétariat.
- À la date de contrôle : tous les élèves sans affectation, sur liste supplémentaire sans place, ou affectés non inscrits deviennent **« sans solution »** (R-OR26) : responsable de suivi désigné (PP, référent, CPE, psychologue), actions datées (appel, rendez-vous, commission départementale [H], CIO), **clôture motivée** (« inscrit au lycée X », « apprentissage », « départ hors académie », « injoignable : signalé au … »).
- **16 ans et plus** : signalement possible à la mission de lutte contre le décrochage [NV] par un administratif, **jamais automatique** (R-OR28).

### 10.6 Côté lycée d'accueil [R]
Le lycée qui **reçoit** des affectés (fichier de la DSDEN, FX-05 du 9) les inscrit par le **socle 3** ; le 37 n'y intervient pas, sauf pour les **lycées privés** qui reçoivent des candidatures directes (socle 3 aussi).

---

## 11. Parcoursup : accompagner, préparer la fiche Avenir, suivre jusqu'en septembre

### 11.1 Accompagner les vœux [R]
- Calendrier personnel de l'élève de terminale (et de sa famille si mineur ou si l'élève majeur le partage) : ouverture, limite des vœux, confirmation, résultats, phase complémentaire (paramètres, RG-O12).
- **Vœux déclarés** (facultatif, R-OR17) : l'élève peut déclarer ses vœux à son équipe (formation, établissement, type : licence, BUT, BTS, CPGE, école, apprentissage) pour préparer la fiche Avenir et l'accompagnement ; **jamais exigé** ; il peut les retirer.
- Ateliers « projet de formation motivé » (activité §7) ; relecture d'un projet motivé à la demande de l'élève (entretien §8).
- **Élèves qui n'ont pas confirmé** : l'établissement ne le sait pas par Parcoursup sans interface ; l'élève coche « vœux confirmés » (rappel la veille de la date) (ORI-262).

### 11.2 Conseil du 2ᵉ trimestre [V-r]
Le conseil **examine les vœux** : le dossier du conseil (23) affiche les vœux déclarés **et** l'état de la fiche Avenir préparée de l'élève (lecture du port du 37) ; les propositions du conseil y restent au 23.

### 11.3 Fiche Avenir préparée [R]
- **Appréciations du professeur principal** par champ (méthode de travail, autonomie, capacité à s'investir, engagement et esprit d'initiative [V* R-31], liste exacte chargée par référentiel annuel) : brouillon pré-rempli depuis les **appréciations validées** (21) et l'**appréciation générale** (23), **jamais** depuis la vie scolaire, la santé, les dispositifs ni les accompagnements (R-OR19, R-V37 du 21, M-1344 du 13) ; contrôle de contenu (mots interdits, mention de sanction effacée) ; deux PP : co-rédaction, une seule validation.
- **Avis du chef d'établissement par vœu** : brouillon proposé à partir de l'**avis du conseil** du 2ᵉ trimestre (23) et du type de formation ; la direction valide vœu par vœu ou en lot (avec relecture obligatoire de chaque avis défavorable ou réservé, R-OR18) ; incohérence avec l'avis du conseil → alerte.
- États : `brouillon` → `validee` → `reportee` (dans Parcoursup, le … par …) → `figee` (au 2 avril, paramètre) ; correction après report : nouvelle version, motif, rappel « à reporter aussi dans Parcoursup ».
- **Lecture par l'élève et la famille** dans ScolIA seulement **à partir de la date nationale** (1ᵉʳ juin [V-r]) : la fiche est un document **administratif communicable** [H juridique, Q-OR9] : demande d'accès anticipée traitée par la direction.
- **Report** : tant qu'aucune certification n'existe, la direction et le PP recopient dans l'interface Parcoursup ; ScolIA fournit une **vue de report** (champ par champ, compteur de caractères selon le paramètre) et le geste « reporté » (R-OR12).

### 11.4 Suivi de la phase d'admission [R]
- Juin–juillet : l'élève déclare (facultatif mais relancé) : « proposition acceptée », « en attente », « aucune proposition », « démission » ; le PP voit l'état de sa classe ; la direction voit les **sans proposition**.
- **Sans proposition** au 1ᵉʳ juillet (paramètre) → **suivi** (comme §10.5) : rendez-vous, **CAES** (commission d'accès à l'enseignement supérieur) [H, NV], phase complémentaire ; clôture motivée.
- Après le bac (résultats du 38) : un bachelier sans proposition reste suivi jusqu'à la date de clôture (septembre).

### 11.5 Élève majeur [R]
- Il déclare seul ; la famille ne voit vœux, fiche et suivi que s'il **partage** (R-OR23) ; information de la famille sur le **calendrier** seulement.

---

## 12. Réorientation et passerelles [R]
- Demande de réorientation en cours d'année (2ᵈᵉ pro vers GT, 1ʳᵉ techno vers pro, abandon d'une spécialité…) : l'élève ou la famille la formule dans ScolIA ; entretien (§8) ; **procédure hors calendrier du 23** ouverte par la direction (le 23 décide) ; affectation éventuelle (rectorat [NV]) ; place dans un autre établissement → socle 3.
- **Changement de spécialité** en cours d'année : socle 5 (offre) et 23 (procédure) ; le 37 affiche au fil.

---

## 13. Devenir des élèves

### 13.1 Ce qu'on recueille [R]
Situation à N+1 (et N+3 facultatif) : `etudes_superieures`, `lycee` (2ᵈᵉ, etc.), `apprentissage`, `emploi`, `service_civique`, `recherche`, `autre`, `inconnue` ; formation et établissement (UAI) quand connus ; **source** (`declaration_ancien_eleve`, `etablissement_accueil`, `affectation`, `admission_parcoursup`, `secretariat`) ; date.

### 13.2 Comment [R]
- **Automatiquement** depuis les résultats d'affectation et d'admission (§10, §11) et les inscriptions effectives : c'est la source principale, sans rien demander.
- **Questionnaire** à l'ancien élève (socle 18) à l'occasion d'un événement (cérémonie du 38, reprise de M17) ou à une date ; **facultatif** ; consentement explicite ; jamais par une page publique qui renvoie des données (ND-01, R-OR22).

### 13.3 Réseau d'anciens [R]
- Un ancien élève peut **consentir** à être recontacté pour intervenir (§7.4) ; consentement daté, révocable ; rien d'autre n'est conservé.

### 13.4 Indicateurs [R]
Taux d'affectation sur le premier vœu, taux d'élèves sans solution au 1ᵉʳ septembre, devenir par niveau et voie (au socle 47) ; agrégés, **jamais par élève** au-delà du personnel concerné ; petits effectifs masqués (< 5, R-OR27).

---

## 14. Le fil de parcours et ce que voit chacun

### 14.1 Fil de parcours de l'élève [R]
Chronologie de la 5ᵉ à la terminale : activités suivies, entretiens tenus (faits), rencontres obligatoires, stages (39), carnet partagé, vœux et décisions (lus au 23), affectation et admission, résultats d'examen (38), devenir. **Titres neutres** dans la chronologie 360° (socle 2).

### 14.2 Tableau du professeur principal [R]
Par élève de ses groupes (D-022) : rencontre obligatoire, activités manquées, carnet partagé récent, état des vœux (23), état Affelnet ou Parcoursup, fiche Avenir, sans-solution ; filtres « à relancer ».

### 14.3 Tableau de la direction [R]
Par niveau : demi-journées tenues, rencontres faites, saisies Affelnet faites, fiches Avenir validées et reportées, avis du chef à donner, sans-solution, cohérence vœux / décisions.

---

## 15. Cas limites (catalogue)

| # | Situation | Traitement [R] |
|---|---|---|
| COR-01 | Les deux parents séparés déclarent des vœux Affelnet différents | les vœux du 37 viennent des **vœux définitifs** du 23, qui a résolu le conflit (orn_conflit_resolution) ; sans résolution : « en attente de décision » |
| COR-02 | La famille a saisi au téléservice des vœux différents de la fiche de dialogue | le 37 ne voit que ce qui est déclaré ; si déclaré : alerte de cohérence, pas de correction |
| COR-03 | Vœu Affelnet en 2ᵈᵉ GT, décision « 2ᵈᵉ pro » | alerte (R-OR11) ; la direction décide d'un entretien ; la famille peut faire appel (23) |
| COR-04 | Appel en cours au moment de la saisie Affelnet | vœux préparés « sous réserve de la commission » ; résultat d'appel → nouvelle cohérence |
| COR-05 | Maintien de droit demandé (D. 331-37) | aucune affectation attendue ; état « maintien » ; pas de sans-solution |
| COR-06 | Élève parti en cours d'année (déménagement) | suivi transféré : état « parti », sans-solution non créé ; information à l'établissement d'accueil seulement par le 3 |
| COR-07 | Élève affecté qui ne s'inscrit pas | sans-solution à la date ; relance ; clôture motivée |
| COR-08 | Élève sur liste supplémentaire rang 3, place libérée en septembre | résultat mis à jour ; inscription ; clôture |
| COR-09 | Élève vers un lycée privé hors Affelnet | vœu « démarche directe » ; état suivi par déclaration |
| COR-10 | Élève qui part en apprentissage (CFA) | formation cible « apprentissage » ; contrat signé déclaré ; devenir |
| COR-11 | Élève allophone arrivé en mai | vœux par l'équipe et la famille avec traduction (socle 8) ; campagne en cours ; procédures académiques [NV] |
| COR-12 | Élève de SEGPA ou d'ULIS | destinations adaptées (23) ; référent (35) dans l'entretien avec accord de la famille |
| COR-13 | Élève handicapé qui demande des formations accessibles | informations générales du catalogue ; jamais de diagnostic dans le 37 ; référent handicap (35) |
| COR-14 | Élève majeur en terminale, parents qui demandent ses vœux Parcoursup | refus motivé si l'élève ne partage pas (R-OR23) ; information calendrier seulement |
| COR-15 | Élève qui devient majeur pendant la phase d'admission | bascule des droits à la date (4) ; ce qu'il avait partagé reste partagé, révocable |
| COR-16 | Deux professeurs principaux en terminale, désaccord sur l'appréciation | une seule version validée ; l'historique garde les deux brouillons ; la direction tranche |
| COR-17 | PP absent (arrêt maladie) pendant la fiche Avenir | délégation par la direction (6, 15) à un collègue ; trace |
| COR-18 | Correction d'une appréciation au bulletin après validation de la fiche Avenir | alerte « la source a changé » ; la fiche ne change pas seule (R-OR19) |
| COR-19 | Appréciation de bulletin qui cite un dispositif (PAP) | exclue du pré-remplissage ; alerte de contenu |
| COR-20 | Sanction effacée citée par un professeur dans un brouillon | contrôle de contenu (CD-49 du 13) : refus de validation |
| COR-21 | Élève sans aucun vœu Parcoursup déclaré | normal (facultatif) ; fiche Avenir sur la base des champs du PP ; avis du chef saisis dans Parcoursup |
| COR-22 | Élève qui ne confirme pas ses vœux avant le 1ᵉʳ avril | rappel la veille et le jour (8) si l'élève a coché « non confirmé » ; jamais d'information à la famille d'un majeur |
| COR-23 | Élève de terminale redoublant | fiche Avenir de l'année ; bulletins de N-1 au 21 |
| COR-24 | Lycée privé qui quitte Charlemagne en cours d'année | remontée Parcoursup par Charlemagne (D-032, Q-X34 du 9) ; ScolIA prépare la fiche |
| COR-25 | Élève de 3ᵉ dans un collège privé, vœu vers un lycée public | même campagne Affelnet (saisie en établissement public et privé sous contrat [V-r]) |
| COR-26 | Collège privé hors contrat | hors Affelnet [H, NV] ; démarches directes ; le 37 suit |
| COR-27 | Activité d'orientation un jour de grève ou de fermeture | report ; participants recalculés |
| COR-28 | Élève absent à toutes les activités d'un niveau | liste « rien suivi » (§7.6) ; signal au PP, au CPE (12) |
| COR-29 | Intervenant extérieur qui demande la liste des élèves présents | refus : nombre et classe seulement (R-OR25) |
| COR-30 | Ancien élève qui demande la suppression de sa situation | effacement (sauf statistiques agrégées déjà calculées) |
| COR-31 | Élève qui veut supprimer une entrée partagée de son carnet | il peut la rendre privée ou la supprimer ; trace de l'action, pas du contenu |
| COR-32 | Entrée du carnet inquiétante (idées noires) | le PP ne la voit que si partagée ; geste « alerter le 36 » ; jamais d'analyse automatique |
| COR-33 | Psychologue qui ne veut rien partager | le fil n'affiche rien ; la demande d'entretien apparaît « transmise » seulement |
| COR-34 | Parent sans exercice qui veut voir les vœux | il **reçoit** les informations importantes (décision, résultat d'affectation) ; il ne voit pas le carnet ni la fiche Avenir préparée |
| COR-35 | Interdiction de communication avec un parent (décision de justice) | aucune information (4) ; la tâche de rencontre obligatoire est faite avec l'autre titulaire |
| COR-36 | Résultats Affelnet annoncés avant la date officielle par un collègue | ScolIA ne publie qu'à la date paramétrée (R-OR16) |
| COR-37 | Erreur de saisie dans Affelnet découverte après la clôture | trace « saisi » corrigée avec motif ; procédure académique [NV] hors ScolIA |
| COR-38 | Fichier de résultats d'affectation avec un INE inconnu | ligne rejetée, jamais un élève créé (9) |
| COR-39 | Élève sans INE | rapprochement par l'identité ScolIA ; alerte |
| COR-40 | Élève sorti du système (décès) | toutes les tâches closes sans relance (2) |
| COR-41 | Élève en internat ou en placement (ASE) | titulaires et personnes à informer selon le 4 ; référent ASE informé selon droits |
| COR-42 | Élève scolarisé au CNED en complément | hors périmètre ; suivi par déclaration |
| COR-43 | Réorientation en novembre de 2ᵈᵉ pro vers GT | procédure du 23 hors calendrier ; place recherchée ; suivi jusqu'à l'inscription |
| COR-44 | Élève inscrit dans deux formations à la rentrée | déclaration ; une seule « situation » de devenir |
| COR-45 | Fermeture de la campagne Parcoursup avant que l'élève ait déclaré son état | état « inconnu » ; aucune extrapolation |
| COR-46 | Deux établissements du même organisme (collège et lycée) | aucun partage entre établissements (D-013) sauf consentement explicite et « groupe » futur (Q13) |

---

## 16. Intelligence artificielle (contrat IA du socle)

| Capacité | Lecture / commande | Ce que fait l'IA | Garde-fous |
|---|---|---|---|
| Expliquer une formation du catalogue | lecture | résume ce qu'est une formation, ses débouchés généraux (sources publiques citées) | pas d'avis sur **un élève** ; mention « information générale » |
| Résumer le parcours **pour l'élève** | lecture (droits de l'élève) | « voici ce que tu as fait cette année » | à la demande de l'élève ; jamais sur ses entrées privées sans sa demande |
| Préparer les relances | commande simulée | liste des élèves à relancer selon un état, brouillon de message | confirmation humaine avant envoi (CLAUDE.md) |
| Brouillon de compte rendu d'entretien | commande simulée | depuis les notes de l'auteur | relu, modifiable, validé par l'auteur |
| Repérer les incohérences | lecture | vœux / décision, fiche / avis du conseil | signaux **déterministes** d'abord ; l'IA explique |
| Relire une fiche Avenir | lecture | signale les mots risqués (sanction, santé, dispositif) | jamais de rédaction d'un avis ni d'une appréciation de fiche Avenir (R-OR24) |
| **Interdits** | — | recommander une formation à un élève ; prédire une réussite ; classer des élèves ; rédiger l'avis du chef d'établissement ; analyser les entrées du carnet ; toute donnée de santé ou de psychologie | R-OR24, RG-O17, RG-O18 |

Étude juridique « règlement IA » avant toute fonction à modèle de langage (comme D-032) ; jeu d'évaluation : 30 cas dont 10 de refus attendu.

---

## 17. Droits, minimisation, conservation

### 17.1 Catalogue d'actions (D-017) [R]
| Droit | Donné par défaut à |
|---|---|
| `orientation.plan_gerer` | direction, référent orientation |
| `orientation.activite_gerer` | référent, direction, professeurs (leurs activités), CDI |
| `orientation.participation_pointer` | encadrants de l'activité |
| `orientation.entretien_tenir` | PP (ses élèves), référent, direction, CPE |
| `orientation.parcours_lire` | PP et professeurs (élèves de leurs groupes, D-022), référent (portée de mission), direction, CPE |
| `orientation.carnet_ecrire` | élève (le sien) |
| `affectation.preparer` | secrétariat, PP (ses élèves), direction |
| `affectation.resultats_saisir` | secrétariat, direction |
| `affectation.suivi_sans_solution` | direction, référent, CPE, PP (ses élèves), secrétariat |
| `parcoursup.fiche_preparer` | PP (ses élèves) |
| `parcoursup.avis_chef` | direction (chef d'établissement, adjoint délégué) |
| `parcoursup.fiche_reporter` | PP, direction |
| `orientation.catalogue_gerer` | secrétariat, direction, référent |
| `orientation.devenir_gerer` | secrétariat, direction |
| `exports.officiels` (existant) | secrétariat, direction (D-022 : jamais un professeur) |

### 17.2 Ce que voit chacun [R]
| Qui | Voit |
|---|---|
| Élève | son fil, son carnet, ses vœux, ses états, sa fiche Avenir **à partir de la date nationale** |
| Titulaire (élève mineur) | calendrier, activités, rencontres, vœux et états d'affectation, carnet **partagé famille**, fiche Avenir à partir de la date nationale |
| Titulaire (élève majeur) | calendrier ; le reste si l'élève partage |
| Parent sans exercice | informations importantes (résultat d'affectation, décision du 23) ; rien d'autre |
| Professeur | parcours des élèves de **ses groupes** (D-022) ; pas la fiche Avenir d'un élève qui n'est pas dans ses groupes |
| PP | ses élèves : tout le 37 sauf entrées privées du carnet et dossier de la psychologue |
| Psychologue | fil et carnet partagé équipe des élèves de ses établissements ; son dossier au 36 |
| Direction | tout le 37 de l'établissement, sauf entrées privées du carnet |
| Intervenant extérieur | rien d'élève |

### 17.3 Garde-fous [R]
- Journal de lecture des fiches Avenir et du carnet partagé (E7) ; export de listes (affectation, sans-solution) réservé aux administratifs, journalisé.
- Aucune page publique ne renvoie de donnée personnelle (R-OR22).
- RLS stricte sur toutes les tables (E13).

### 17.4 Conservation (à valider avec le DPO et l'archiviste) [R]
| Donnée | Durée proposée |
|---|---|
| Activités, participations | année + 1 an, puis agrégats |
| Comptes rendus d'entretien | jusqu'à la sortie de l'élève + 1 an |
| Carnet de parcours | jusqu'à la sortie + 1 an ; export remis à l'élève à la sortie |
| Vœux et résultats d'affectation, admission | sortie + 2 ans (proche des durées nationales, RG-O15 ; commissions d'appel : 2 ans, 23 R-B14) |
| Fiche Avenir préparée | 2 ans après l'année de terminale |
| Suivi sans-solution | 3 ans [H] (preuve de l'accompagnement) |
| Devenir | 3 ans, puis agrégats anonymes |
| Consentement réseau d'anciens | tant que valide, puis effacement |

---

## 18. Notifications (socle 8)
Natures déclarées : `orientation.activite_publiee` (famille, élève), `orientation.rappel_activite` (élève), `orientation.rencontre_proposee`, `.rencontre_rappel` (titulaires), `affectation.campagne_ouverte`, `.rappel_saisie` (titulaires, J-7 et J-1 de la fin de fenêtre), `.resultat_publie` (titulaires, élève, **à la date nationale**), `.inscription_attendue` (titulaires), `affectation.sans_solution` (responsable du suivi), `parcoursup.rappel_confirmation` (élève), `.fiche_a_preparer` (PP), `.avis_a_donner` (direction), `.fiche_consultable` (élève, famille si autorisée, au 1ᵉʳ juin), `devenir.questionnaire` (ancien élève consentant). Parent sans exercice : `resultat_publie` seulement. « Ne pas solliciter » (D-028) respecté.

---

## 19. Actions métier (ORI-100 et suivantes)

Légende des acteurs : DIR direction, ADF administratif (secrétariat), REF référent orientation, PP professeur principal, PRF professeur, PSY psychologue, CPE, CDI documentaliste, PAR titulaire, PSE parent sans exercice, ELV élève mineur, ELM élève majeur, SYS système, EXT intervenant sans compte. ★ = confirmation renforcée.

### 19.1 Plan et paramètres
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| ORI-100 | Créer le plan d'orientation de l'année (objectifs par niveau, demi-journées) | DIR, REF | RG-O04 | `orientation.plan_cree` |
| ORI-101 | Reprendre le plan de l'année précédente | DIR, REF | dates décalées à valider | `orientation.plan_cree` |
| ORI-102 | Modifier, publier le plan | DIR | — | `orientation.plan_publie` |
| ORI-103 | Paramétrer les types d'activités (préréglage) | DIR, REF | D-022 | — |
| ORI-104 | Voir la couverture pluriannuelle (5ᵉ → terminale) et ses trous | DIR, REF | — | — |
| ORI-105 | Paramétrer les dates nationales et académiques de l'année (Affelnet, Parcoursup, fiche Avenir) | ADF, DIR | RG-O06, R-OR2 | `orientation.calendrier_maj` |
| ORI-106 | Exporter le plan en PDF de présentation | DIR, REF | socle 7 | — |
| ORI-107 | Désigner le référent orientation par niveau (mission 355) | DIR | socle 6 | — |
| ORI-108 | Charger le référentiel annuel de la fiche Avenir (champs, échelle de l'avis) | ADF | RG-O13 [NV] → saisi depuis le texte | — |
| ORI-109 | Gérer le catalogue des formations cibles (ajout, lien destination, UAI) | ADF, REF | §10.1 | — |
| ORI-110 | Importer des formations depuis l'annuaire de l'éducation et la BCN (socle 9) | ADF | 9 X4 | — |
| ORI-111 | Archiver une formation qui n'existe plus | ADF | jamais de suppression si utilisée | — |

### 19.2 Activités
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| ORI-120 | Créer une activité (type, date, public, lieu, encadrants, capacité) | REF, PRF, CDI, DIR | §7.2 | `orientation.activite_creee` |
| ORI-121 | Réserver salle et créneau (19) | REF | priorité R-AG21 du 19 | — |
| ORI-122 | Neutraliser ou déplacer les séances touchées (10) | REF, DIR ★ | — | `edt.exception` (10) |
| ORI-123 | Publier l'activité aux élèves et familles | REF | 8 | `orientation.activite_publiee` |
| ORI-124 | Ouvrir les inscriptions de volontaires, liste d'attente | REF | capacité | — |
| ORI-125 | S'inscrire, se désinscrire | ELV, ELM | délai paramétré | `orientation.inscription` |
| ORI-126 | Pointer les présences (web, mobile) | encadrant | R-OR10 | `orientation.participation` |
| ORI-127 | Clore l'activité, bilan, questionnaire de retour | REF | socle 18 | `orientation.activite_tenue` |
| ORI-128 | Liste des élèves qui n'ont suivi aucune activité du niveau | PP, REF, DIR | §7.6 | — |
| ORI-129 | Reporter, annuler une activité (avis aux inscrits) | REF | — | `orientation.activite_annulee` |
| ORI-130 | Inviter un intervenant extérieur (lien nominatif) | REF | socle 7, R-OR25 | — |
| ORI-131 | Confirmer sa venue, déposer un support | EXT | lien à usage limité | — |
| ORI-132 | Gérer l'annuaire des intervenants (minimal) | REF | §7.4 | — |
| ORI-133 | Publier l'agenda des événements extérieurs (portes ouvertes, salons) | REF, CDI | — | — |
| ORI-134 | Demander une immersion ou un mini-stage dans un lycée | ELV, PP | autorisation 18 ; 39 | `orientation.immersion_demandee` |
| ORI-135 | Autoriser l'immersion | PAR | socle 18 | — |
| ORI-136 | Enregistrer l'absence attendue (12) | SYS | — | — |
| ORI-137 | Retour d'expérience de l'immersion au carnet | ELV | — | — |
| ORI-138 | Relier une ressource du CDI à une activité | CDI | socle 45 | — |
| ORI-139 | Dupliquer une activité pour toutes les classes d'un niveau | REF | — | — |

### 19.3 Entretiens et rencontres
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| ORI-150 | Demander un entretien d'orientation | ELV, ELM, PAR, PP | créneau 19 | `orientation.entretien_demande` |
| ORI-151 | Proposer des créneaux, fixer le rendez-vous | PP, REF, DIR | 19 | — |
| ORI-152 | Tenir l'entretien, compte rendu minimal | auteur | R-OR20 | `orientation.entretien_tenu` |
| ORI-153 | Publier les suites convenues à la famille et à l'élève | auteur | — | — |
| ORI-154 | Ouvrir la campagne des rencontres obligatoires (3ᵉ, 2ᵈᵉ) | DIR, REF | RG-O04, R-OR9 | `orientation.rencontres_ouvertes` |
| ORI-155 | Proposer la rencontre à chaque famille | PP | — | `orientation.rencontre_proposee` |
| ORI-156 | Répondre (accepter un créneau, décliner avec motif) | PAR | — | — |
| ORI-157 | Marquer la rencontre faite, refusée, impossible (motif) | PP | — | `orientation.rencontre_faite` |
| ORI-158 | Tableau des rencontres par classe, relance ciblée | PP, DIR | — | — |
| ORI-159 | Réunion collective par classe comptée pour les présents | PP | 19 | — |
| ORI-160 | Demander un entretien avec la psychologue (transmis au 36) | ELV, PAR, PP | R-OR21 | `psy.demande` (36) |
| ORI-161 | Partager un fait d'entretien au fil (sans contenu) | PSY | par défaut non | — |
| ORI-162 | Déléguer le suivi des entretiens d'un PP absent | DIR | 6, 15 | — |

### 19.4 Carnet de parcours
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| ORI-180 | Écrire une entrée (réflexion, intérêt, retour, document) | ELV, ELM | §9 | — |
| ORI-181 | Choisir la visibilité d'une entrée | ELV, ELM | R-OR23 | — |
| ORI-182 | Repérer une formation au catalogue et l'ajouter à ses intérêts | ELV | — | — |
| ORI-183 | Lire le carnet partagé d'un élève | PP, REF, PSY, PAR | visibilité | — |
| ORI-184 | Commenter une entrée partagée (encouragement, ressource) | PP, REF | l'élève voit | — |
| ORI-185 | Alerter le 36 depuis une entrée partagée | PP, REF ★ | geste explicite | `suivi.alerte` (36) |
| ORI-186 | Exporter son carnet (PDF, fichiers) | ELV, ELM | 27 | — |
| ORI-187 | Supprimer ou rendre privée une entrée | ELV, ELM | COR-31 | — |
| ORI-188 | Joindre une attestation (stage, Pix, PSC1) depuis le 38 ou le 39 | ELV | — | — |

### 19.5 Affelnet
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| ORI-200 | Créer la campagne d'affectation (fenêtres, tours, nombre de vœux) | ADF, DIR | RG-O09, R-OR2 | `affectation.campagne_creee` |
| ORI-201 | Préparer les vœux depuis les vœux définitifs (23) | SYS, PP | R-OR11 | `affectation.voeux_prepares` |
| ORI-202 | Associer une formation cible à chaque vœu | PP, ADF, PAR | catalogue | — |
| ORI-203 | Déclarer les vœux saisis au téléservice (copie facultative) | PAR | — | — |
| ORI-204 | Demander que l'établissement saisisse à sa place | PAR | — | `affectation.saisie_demandee` |
| ORI-205 | Contrôler la cohérence vœux / décision | SYS | R-OR11 | `affectation.incoherence` |
| ORI-206 | Produire la liste de saisie (ce qui manque inclus) | ADF | — | — |
| ORI-207 | Cocher « saisi dans Affelnet le … » | ADF | R-OR12 | `affectation.saisi` |
| ORI-208 | Suivre la démarche directe vers un lycée privé | PP, ADF | RG-O10 | — |
| ORI-209 | Joindre une pièce demandée par une procédure particulière | ADF, PAR | 7 | — |
| ORI-210 | Relancer les familles qui n'ont pas saisi | ADF, PP | confirmation | `affectation.rappel_saisie` |
| ORI-211 | Saisir les résultats d'un tour (par élève) | ADF | — | `affectation.resultat` |
| ORI-212 | Importer les résultats depuis un fichier (tableur) | ADF | rejet ligne à ligne (9) | `affectation.resultat` |
| ORI-213 | Publier les résultats aux familles à la date nationale | SYS | R-OR16 | `affectation.resultat_publie` |
| ORI-214 | Préparer le second tour | ADF, PP | — | — |
| ORI-215 | Déclarer l'inscription effective au lycée d'accueil | PAR, ADF | — | `affectation.inscrit` |
| ORI-216 | Recevoir l'inscription d'un lycée ScolIA (socle 3) | SYS | — | `affectation.inscrit` |
| ORI-217 | Lister les affectés non inscrits | ADF, DIR | — | — |
| ORI-218 | Corriger une trace de saisie (motif) | ADF | COR-37 | — |
| ORI-219 | Exporter la liste de travail des vœux (non officielle) | ADF | D-022 ; X0 du 9 | `export.realise` |

### 19.6 Parcoursup et fiche Avenir
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| ORI-240 | Ouvrir la campagne Parcoursup de l'année (dates) | ADF | RG-O12 | — |
| ORI-241 | Déclarer ses vœux à l'équipe (facultatif) | ELV, ELM | R-OR17 | `parcoursup.voeu_declare` |
| ORI-242 | Retirer un vœu déclaré | ELV, ELM | — | — |
| ORI-243 | Voir les vœux déclarés de sa classe | PP | D-022 | — |
| ORI-244 | Préparer les appréciations de la fiche Avenir (pré-remplies) | PP | R-OR19 | `parcoursup.fiche_brouillon` |
| ORI-245 | Co-rédiger (second PP), commenter | PP | COR-16 | — |
| ORI-246 | Valider la fiche (contrôle de contenu) | PP | CD-49 du 13 | `parcoursup.fiche_validee` |
| ORI-247 | Préparer l'avis du chef par vœu depuis l'avis du conseil | SYS | R-OR18 | — |
| ORI-248 | Valider les avis du chef (un par un, ou en lot avec relecture des réservés) | DIR ★ | R-OR18 | `parcoursup.avis_valide` |
| ORI-249 | Vue de report champ par champ | PP, DIR | R-OR12 | — |
| ORI-250 | Marquer « reporté dans Parcoursup » | PP, DIR | — | `parcoursup.fiche_reportee` |
| ORI-251 | Figer les fiches à la date limite | SYS | paramètre | `parcoursup.fiches_figees` |
| ORI-252 | Corriger après report (version, motif, rappel de report) | PP, DIR | — | — |
| ORI-253 | Rendre la fiche consultable à l'élève à la date nationale | SYS | R-OR18 | `parcoursup.fiche_consultable` |
| ORI-254 | Traiter une demande d'accès anticipée | DIR | Q-OR9 | — |
| ORI-255 | Tableau d'avancement des fiches (PP, direction) | PP, DIR | — | — |
| ORI-256 | Pré-contrôle de ce qui partirait en remontée (moyennes, appréciations) | ADF | RG-O14 ; ITP-133 du 9 | — |
| ORI-257 | Déclarer l'état d'admission (accepté, en attente, sans proposition, démission) | ELV, ELM | — | `admission.etat` |
| ORI-258 | Voir l'état d'admission de sa classe | PP | — | — |
| ORI-259 | Ouvrir un suivi « sans proposition » | SYS, DIR | R-OR26 | `admission.sans_proposition` |
| ORI-260 | Proposer un rendez-vous, orienter vers la commission (CAES) ou la phase complémentaire | PP, REF | — | — |
| ORI-261 | Clore le suivi (motif) | REF, DIR | — | — |
| ORI-262 | Cocher « vœux confirmés » et recevoir un rappel sinon | ELV, ELM | COR-22 | — |
| ORI-263 | Atelier projet de formation motivé (activité) | REF, PRF | §7 | — |
| ORI-264 | Demander une relecture de son projet motivé | ELV | entretien | — |

### 19.7 Sans solution, devenir, réorientation
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| ORI-300 | Calculer la liste des sans-solution à une date | SYS | R-OR26 | `orientation.sans_solution` |
| ORI-301 | Désigner le responsable du suivi | DIR | — | — |
| ORI-302 | Enregistrer une action de suivi (appel, rendez-vous, commission) | responsable | — | — |
| ORI-303 | Clore un suivi (motif codé) | responsable, DIR | — | `orientation.suivi_clos` |
| ORI-304 | Préparer le signalement à la mission de lutte contre le décrochage (16 ans et plus) | ADF ★ | R-OR28 | — |
| ORI-305 | Voir les sans-solution avec le drapeau « suivi GPDS » (36) | DIR, CPE | sans contenu | — |
| ORI-306 | Transmettre la liste à la DSDEN ou au CIO (export, à la demande) | ADF ★ | D-022 | `export.realise` |
| ORI-320 | Demander une réorientation | ELV, ELM, PAR | §12 | `orientation.reorientation_demandee` |
| ORI-321 | Instruire (entretien, avis de l'équipe) | PP, DIR | — | — |
| ORI-322 | Ouvrir la procédure du 23 hors calendrier | DIR | 23 | — |
| ORI-323 | Suivre la recherche de place, l'inscription | ADF | 3 | — |
| ORI-330 | Enregistrer le devenir depuis l'affectation ou l'admission | SYS | §13.2 | `devenir.situation` |
| ORI-331 | Saisir le devenir déclaré (secrétariat) | ADF | — | — |
| ORI-332 | Envoyer le questionnaire de devenir aux anciens élèves consentants | ADF | 18, R-OR22 | — |
| ORI-333 | Répondre au questionnaire | ancien élève | lien nominatif ou compte | — |
| ORI-334 | Consentir au réseau d'anciens, retirer son consentement | ancien élève | §13.3 | — |
| ORI-335 | Indicateurs de devenir (agrégés) | DIR | R-OR27 | — |
| ORI-336 | Effacer sa situation | ancien élève | COR-30 | — |

### 19.8 Pilotage, échanges, IA, mobile
| Id | Action | Acteurs | Règles | Événement |
|---|---|---|---|---|
| ORI-350 | Tableau de bord orientation par niveau | DIR | §14.3 | — |
| ORI-351 | Tableau du PP | PP | §14.2 | — |
| ORI-352 | Fil de parcours dans la fiche 360° | personnels autorisés | §14.1 | — |
| ORI-353 | Calendrier personnel d'orientation (élève, famille) | ELV, PAR | — | — |
| ORI-354 | Mobile : pointer une présence, voir son calendrier, déclarer un état | encadrant, ELV | socle 11 | — |
| ORI-355 | IA : expliquer une formation | tous | §16 | — |
| ORI-356 | IA : résumé de parcours pour l'élève | ELV, ELM | §16 | — |
| ORI-357 | IA : brouillon de relances | PP, ADF | confirmation | — |
| ORI-358 | IA : brouillon de compte rendu | auteur | relu | — |
| ORI-359 | IA : relecture de risques d'une fiche Avenir | PP, DIR | signale seulement | — |
| ORI-360 | Export officiel Parcoursup (remontée) — bloqué D-2 | ADF | lot OR12 | `export.realise` |
| ORI-361 | Import officiel des affectés et résultats (si format) — bloqué | ADF | lot OR12 | — |
| ORI-362 | Reprise des archives Next.js utiles (situation des anciens) | ADF | §2.3, Q-OR10 | — |

**Total : 130 actions** (ORI-100 à ORI-362, numérotation non continue), plus 36 de la passe personas (§28.2) = **166**.

---

## 20. Règles transverses (R-OR1 et suivantes) [R]

| # | Règle |
|---|---|
| R-OR1 | Le 37 **n'écrit jamais** une donnée du circuit du 23 (vœux d'orientation, décision, appel). |
| R-OR2 | Toute date nationale ou académique est un **paramètre de campagne** daté et sourcé, jamais une constante. |
| R-OR3 | Un élève d'un palier a **toujours un état** affiché ; « inconnu » est un état, jamais un vide. |
| R-OR4 | Portée du professeur au **groupe** (D-022), partout, fiche Avenir comprise. |
| R-OR5 | Participants d'une activité **calculés à la date** de l'activité. |
| R-OR6 | Les intervenants extérieurs n'ont **pas de compte** et ne voient **aucune donnée d'élève**. |
| R-OR7 | Chaque écriture porte auteur, date, version ; pas d'écrasement silencieux (E6, E7). |
| R-OR8 | Le compteur de demi-journées est un **préréglage** (4), modifiable par l'établissement. |
| R-OR9 | La rencontre obligatoire est suivie **élève par élève** ; une réunion collective ne compte que pour les présents. |
| R-OR10 | La présence à une activité tenue sur une séance passe par l'**appel** du 12 ; pas de double pointage. |
| R-OR11 | Incohérence vœu / décision : **alerte**, jamais blocage ni correction automatique. |
| R-OR12 | Tant qu'aucune interface officielle n'existe, ScolIA **prépare et trace le report** ; il ne prétend jamais avoir transmis. |
| R-OR13 | ScolIA ne calcule **aucun barème**, rang ou probabilité d'affectation ou d'admission. |
| R-OR14 | Les résultats d'affectation sont **par tour** ; le dernier fait foi, l'historique reste. |
| R-OR15 | Un vœu vers le privé porte la **démarche directe** à suivre. |
| R-OR16 | Un résultat national n'est montré aux familles **qu'à la date nationale** de publication. |
| R-OR17 | La déclaration de vœux Parcoursup par l'élève est **facultative** ; aucun écran ne la présente comme obligatoire. |
| R-OR18 | Avis du chef par vœu : préparé depuis l'avis du conseil, **validé par une personne** ; tout avis réservé ou défavorable est relu un à un ; fiche visible de l'élève à la date nationale. |
| R-OR19 | Le pré-remplissage de la fiche Avenir ne prend que les appréciations **validées** ; jamais vie scolaire, santé, dispositif, accompagnement, sanction ; le contrôle de contenu du 13 (CD-49) s'applique ; une source modifiée ne change pas la fiche seule. |
| R-OR20 | Compte rendu d'entretien : minimal ; famille et élève ne voient que les **suites publiées**. |
| R-OR21 | Le dossier de la psychologue reste au 36 ; le 37 n'affiche que ce qu'elle partage ; par défaut rien. |
| R-OR22 | Aucune page publique ni lien ne renvoie une donnée personnelle à qui ne s'est pas authentifié ou n'a pas reçu un lien nominatif (ND-01). |
| R-OR23 | Le carnet appartient à l'élève ; majeur, il décide seul du partage avec sa famille ; rien de privé ne devient visible à un changement d'état. |
| R-OR24 | L'IA ne recommande, ne prédit, ne classe ni ne rédige aucun avis ; aucune décision automatique ; aucune analyse du carnet. |
| R-OR25 | Un intervenant ou un tiers reçoit des **effectifs**, jamais des listes nominatives. |
| R-OR26 | « Sans solution » est **calculé** à des dates paramétrées et **clos par une personne** avec un motif. |
| R-OR27 | Indicateurs : agrégés, petits effectifs (< 5) masqués. |
| R-OR28 | Aucune transmission automatique à un tiers (CIO, mission décrochage, DSDEN) ; export par un administratif, journalisé. |

---

## 21. Événements (outbox, dans la transaction du fait)
**Émis** : `orientation.plan_cree`, `.plan_publie`, `.calendrier_maj`, `.activite_creee`, `.activite_publiee`, `.activite_annulee`, `.activite_tenue`, `.inscription`, `.participation`, `.immersion_demandee`, `.entretien_demande`, `.entretien_tenu`, `.rencontres_ouvertes`, `.rencontre_proposee`, `.rencontre_faite`, `.reorientation_demandee`, `.sans_solution`, `.suivi_clos` ; `affectation.campagne_creee`, `.voeux_prepares`, `.incoherence`, `.saisie_demandee`, `.saisi`, `.resultat`, `.resultat_publie`, `.inscrit` ; `parcoursup.voeu_declare`, `.fiche_brouillon`, `.fiche_validee`, `.avis_valide`, `.fiche_reportee`, `.fiches_figees`, `.fiche_consultable` ; `admission.etat`, `.sans_proposition` ; `devenir.situation`.
**Consommés** : `orientation.demande_deposee`, `.decision_*`, `.appel_decide`, `.maintien_demande`, `.depart_prevu` (23) → préparation et cohérence ; `bulletins.publies` (23), `evaluations.appreciations_closes` (21) → sources de la fiche ; `inscription.confirmee` (3) → inscription effective ; `eleve.majeur`, `eleve.sorti`, `eleve.decede`, `liens.*` (2, 4) → droits et clôtures ; `examen.resultat_officiel` (38) → devenir et admission ; `stage.convention_signee` (39) → fil ; `psy.partage` (36) → fil.

---

## 22. Écrans clés (Leptos)
1. **Plan d'orientation** (tuiles par niveau, demi-journées, trous) — DIR, REF.
2. **Activité** (fiche, participants, pointage mobile, bilan).
3. **Tableau du PP** (dense : élèves × états, filtres « à relancer »).
4. **Fil de parcours** (chronologie de l'élève, aussi dans la fiche 360°).
5. **Carnet** (élève : entrées, visibilité en un geste).
6. **Affelnet** : liste de préparation, cohérence, liste de saisie, résultats par tour, non-inscrits.
7. **Parcoursup** : vœux déclarés, fiche Avenir (édition, contrôle, report), avis du chef par vœu (lot, relecture des réservés), états d'admission.
8. **Sans solution** (liste, responsable, actions, clôture).
9. **Devenir** (indicateurs agrégés).
10. **Famille et élève** : calendrier personnel, « ce qu'il reste à faire ».
Mobile (socle 11) : pointage, calendrier, déclaration d'état, rappels.

---

## 23. Modèle de données cible (brique `orientation_parcours`, sans JSON, E1–E14)

Toutes les tables : `id UUID` applicatif (E3), `etablissement_id UUID NOT NULL` (E1), FK **composites** `(etablissement_id, x_id)` (E2), `cree_le`, `modifie_le` (E6), RLS stricte (E13), classées au registre (E12) ; aucun JSON, `BYTEA` ni tableau (E10, E5) ; fichiers par `ent_objet` (E5).

```
-- Paramètres et référentiels
ori_parametre_annee           id, etablissement_id, annee_id, demi_journees_prevues_defaut SMALLINT DEFAULT 4,
                              date_controle_sans_solution DATE NULL, date_cloture_suivi DATE NULL, cree_le, modifie_le
ori_calendrier_date           id, etablissement_id, annee_id, nature ('affelnet_famille_debut','affelnet_famille_fin',
                              'affelnet_etab_debut','affelnet_etab_fin','affelnet_resultats','affelnet_tour2_debut','affelnet_tour2_fin',
                              'inscription_accueil_limite','psup_voeux_debut','psup_voeux_fin','psup_confirmation_fin',
                              'fiche_avenir_limite','fiche_avenir_consultable','psup_admission_debut','psup_admission_fin',
                              'psup_complementaire_fin','rencontres_debut','rencontres_fin'), le DATE, heure TIME NULL,
                              source VARCHAR(300), cree_le, modifie_le   UNIQUE (etablissement_id, annee_id, nature)
ori_type_activite             id, etablissement_id, code, libelle, compte_demi_journee BOOLEAN, ordre, archive_le NULL
ori_fiche_avenir_champ        id, etablissement_id, annee_id, code, libelle, longueur_max SMALLINT, ordre    -- référentiel annuel chargé (RG-O13)
ori_avis_chef_valeur          id, etablissement_id, annee_id, code, libelle, rang SMALLINT, exige_relecture BOOLEAN

-- Plan et activités
ori_plan                      id, etablissement_id, annee_id, etat ('brouillon','publie'), publie_le NULL, publie_par NULL, version
ori_plan_niveau               id, etablissement_id, plan_id, niveau_id, objectifs VARCHAR(1500), demi_journees_prevues SMALLINT
ori_activite                  id, etablissement_id, plan_id NULL, type_id, titre VARCHAR(200), description VARCHAR(3000),
                              debut TIMESTAMPTZ, fin TIMESTAMPTZ, lieu_salle_id NULL, lieu_externe VARCHAR(300) NULL,
                              capacite INT NULL, mode_public ('impose','volontaires'), etat ('brouillon','publiee','tenue','bilan',
                              'annulee','reportee'), reservation_id NULL (19), sortie_id NULL (42), version
ori_activite_public           id, etablissement_id, activite_id, niveau_id NULL, classe_id NULL, groupe_id NULL   -- CHECK un seul
ori_activite_encadrant        id, etablissement_id, activite_id, personne_id
ori_intervenant               id, etablissement_id, nom VARCHAR(150), structure VARCHAR(200), domaine VARCHAR(200),
                              courriel_pro VARCHAR(254) NULL, telephone_pro VARCHAR(30) NULL, ancien_eleve_consentement_id NULL, archive_le NULL
ori_activite_intervenant      id, etablissement_id, activite_id, intervenant_id, confirme_le NULL, lien_id NULL (7)
ori_participation             id, etablissement_id, activite_id, eleve_id, inscription ('imposee','volontaire','attente'),
                              presence ('inconnue','present','absent','excuse') DEFAULT 'inconnue', source ('appel','pointage'),
                              pointe_par NULL, pointe_le NULL   UNIQUE (etablissement_id, activite_id, eleve_id)
ori_activite_document         id, etablissement_id, activite_id, objet_id (ent_objet)
ori_immersion                 id, etablissement_id, eleve_id, formation_cible_id NULL, etablissement_accueil VARCHAR(200),
                              du DATE, au DATE, autorisation_id NULL (18), convention_id NULL (39), etat ('demandee','autorisee','faite','annulee')

-- Entretiens, rencontres
ori_entretien                 id, etablissement_id, eleve_id, demande_par_personne_id, nature ('orientation','rencontre_obligatoire',
                              'projet_motive','reorientation'), rendez_vous_id NULL (19), tenu_le NULL, auteur_personne_id NULL,
                              points_abordes VARCHAR(1500) NULL, suites VARCHAR(800) NULL, suites_publiees_le NULL, version
ori_entretien_participant     id, etablissement_id, entretien_id, personne_id, qualite ('eleve','titulaire','pp','referent','direction','autre')
ori_rencontre_attendue        id, etablissement_id, annee_id, eleve_id, etat ('attendue','proposee','faite','refusee','impossible'),
                              motif VARCHAR(400) NULL, entretien_id NULL, maj_par, maj_le   UNIQUE (etablissement_id, annee_id, eleve_id)
ori_fait_psy_partage          id, etablissement_id, eleve_id, le DATE, libelle VARCHAR(120), partage_par (psy)   -- sans contenu

-- Carnet
ori_carnet_entree             id, etablissement_id, eleve_id, nature ('reflexion','interet','retour','document','formation'),
                              titre VARCHAR(200), texte VARCHAR(4000) NULL, formation_cible_id NULL, activite_id NULL, objet_id NULL,
                              visibilite ('privee','partagee_equipe','partagee_famille'), supprime_le NULL, version
ori_carnet_commentaire        id, etablissement_id, entree_id, auteur_personne_id, texte VARCHAR(1000), supprime_le NULL

-- Catalogue
ori_formation_cible           id, etablissement_id, libelle VARCHAR(300), code_bcn VARCHAR(20) NULL, uai CHAR(8) NULL,
                              etablissement_libelle VARCHAR(200), commune VARCHAR(120) NULL, statut ('public','prive_contrat',
                              'prive_hors_contrat','apprentissage','superieur','autre'), voie VARCHAR(80) NULL,
                              destination_id NULL (orn_destination, 23), recrutement_particulier BOOLEAN, archive_le NULL

-- Affelnet
ori_aff_campagne              id, etablissement_id, annee_id, nb_voeux_max SMALLINT, nb_tours SMALLINT, etat ('ouverte','close')
ori_aff_dossier               id, etablissement_id, campagne_id, eleve_id, mode_saisie ('famille','etablissement','inconnu'),
                              etat ('sans_voeu','voeux_prets','saisi','affecte','liste_supplementaire','non_affecte','inscrit',
                              'maintien','parti','sans_solution'), saisi_le NULL, saisi_par NULL, version
                              UNIQUE (etablissement_id, campagne_id, eleve_id)
ori_aff_voeu                  id, etablissement_id, dossier_id, rang SMALLINT, formation_cible_id, origine ('fiche_dialogue',
                              'equipe','famille'), voeu_orientation_id NULL (orn_voeu, 23), coherence ('ok','alerte','en_attente'),
                              demarche_directe_faite_le NULL, retire_le NULL
ori_aff_resultat              id, etablissement_id, dossier_id, tour SMALLINT, voeu_id NULL, issue ('affecte','liste_supplementaire',
                              'refuse','sans_suite'), rang_attente INT NULL, source ('saisie','fichier'), saisi_par, saisi_le, publie_le NULL
ori_aff_inscription           id, etablissement_id, dossier_id, declare_par ('famille','secretariat','socle3'), le DATE,
                              etablissement_accueil VARCHAR(200), uai CHAR(8) NULL
ori_aff_trace_correction      id, etablissement_id, dossier_id, motif VARCHAR(400), par, le            -- ajout seul

-- Parcoursup
ori_psup_voeu_declare         id, etablissement_id, eleve_id, annee_id, formation VARCHAR(300), etablissement_libelle VARCHAR(200),
                              type ('licence','but','bts','cpge','ecole','apprentissage','autre'), retire_le NULL
ori_fiche_avenir              id, etablissement_id, eleve_id, annee_id, etat ('brouillon','validee','reportee','figee'),
                              valide_par NULL, valide_le NULL, reporte_par NULL, reporte_le NULL, consultable_le NULL, version
                              UNIQUE (etablissement_id, eleve_id, annee_id)
ori_fiche_avenir_appreciation id, etablissement_id, fiche_id, champ_id, texte VARCHAR(1000), auteur_personne_id, source_appreciation_id NULL,
                              version
ori_fiche_avenir_historique   id, etablissement_id, fiche_id, version, motif VARCHAR(400) NULL, par, le   -- ajout seul ; instantané texte
ori_avis_chef                 id, etablissement_id, fiche_id, voeu_declare_id NULL, libelle_voeu VARCHAR(300), valeur_id,
                              prepare_depuis_avis_conseil_id NULL, etat ('propose','valide'), valide_par NULL, valide_le NULL, version
ori_psup_admission            id, etablissement_id, eleve_id, annee_id, etat ('inconnu','acceptee','en_attente','sans_proposition',
                              'demission','complementaire'), declare_le, declare_par
                              -- une ligne par déclaration (ajout seul), la plus récente fait foi

-- Suivi, réorientation, devenir
ori_suivi                     id, etablissement_id, eleve_id, annee_id, origine ('affectation','admission','reorientation','autre'),
                              responsable_personne_id NULL, ouvert_le, clos_le NULL, motif_cloture ('inscrit','apprentissage','emploi',
                              'depart','maintien','injoignable_signale','autre') NULL, commentaire_cloture VARCHAR(600) NULL
ori_suivi_action              id, etablissement_id, suivi_id, nature ('appel','rendez_vous','courrier','commission','cio','autre'),
                              le, par, note VARCHAR(600)
ori_reorientation             id, etablissement_id, eleve_id, annee_id, demande_par_personne_id, objet VARCHAR(600),
                              etat ('demandee','instruite','procedure_ouverte','aboutie','abandonnee'), procedure_id NULL (orn_procedure, 23)
ori_devenir                   id, etablissement_id, eleve_id, annee_reference_id, situation ('etudes_superieures','lycee','apprentissage',
                              'emploi','service_civique','recherche','autre','inconnue'), formation VARCHAR(300) NULL, uai CHAR(8) NULL,
                              source ('affectation','admission','etablissement_accueil','declaration','secretariat'), le, efface_le NULL
ori_consentement_ancien       id, etablissement_id, eleve_id, nature ('recontact_intervention','questionnaire_devenir'),
                              donne_le, retire_le NULL
```
**Exceptions D-036** : aucune. **Files d'effets de bord** : aucune nouvelle (avis par l'outbox et la file du socle 8 ; liens nominatifs du 7, déjà E9). **Registre E12** : toutes en portée `etablissement` ; export `simple` sauf `ori_carnet_entree` (`particuliere` : remis à l'élève) et `ori_fiche_avenir*` (`double`). **Tables existantes** : `ent_voeu_inscription` et `ent_filiere` (077) restent au 3 et au 5 ; aucune reprise de données vers le 37.

---

## 24. Lots de code (équipe Sonnet) et migrations

### 24.1 Lots
| Lot | Contenu | Dépend de | Migr. | Taille | Critère de fin |
|---|---|---|---|---|---|
| **OR0** | OD1 (écriture des vœux par famille/élève fermée, journal) ; OD2 (publication en transaction) | — | — | S | tests : parent et élève refusés ; événement atomique ; non-régression 23 |
| **OR1** | paramètres, calendrier daté, types d'activités, catalogue des formations, référentiel fiche Avenir | 5, 9 (X4) | 2740, 2741 | M | dates sourcées ; formation liée à destination |
| **OR2** | plan, activités, public, encadrants, intervenants, participations, bilan | OR1 ; 19, 10, 12 | 2742, 2743 | L | activité imposée calculée à la date ; pointage mobile |
| **OR3** | entretiens, rencontres obligatoires, faits partagés par la psychologue | OR1 ; 19, 36 | 2744 | M | rencontre suivie par élève ; rien du 36 sans partage |
| **OR4** | carnet de parcours (visibilité, commentaires, export) | OR1 | 2745 | M | majeur : partage seul ; privé jamais lu |
| **OR5** | Affelnet : campagne, dossiers, vœux préparés depuis le 23, cohérence, liste de saisie, démarches | OR1 ; **B8 du 23** | 2746 | L | port `voeux_definitifs` lu ; alerte de cohérence |
| **OR6** | Affelnet : résultats par tour, import tableur, publication datée, inscription effective | OR5 ; 3, 9 (X2) | 2747 | M | publication à la date ; non-inscrits listés |
| **OR7** | Parcoursup : vœux déclarés, fiche Avenir (pré-remplissage, contrôle, validation, report, figeage, consultation datée), avis du chef | OR1 ; 21, 23, 13 (CD-49) | 2748, 2749 | L | aucune source interdite ; avis réservés relus |
| **OR8** | admission, sans-solution (calcul, suivi, clôture), réorientation | OR6, OR7 ; 23, 36 | 2750, 2751 | M | sans-solution calculé et clos par une personne |
| **OR9** | devenir (automatique, questionnaire, consentements), reprise M17 (situation) | OR8 ; 18, 38 | 2752 | M | aucune page publique ne renvoie de donnée (R-OR22) |
| **OR10** | tableaux PP et direction, fil de parcours (fiche 360°), indicateurs (47) | OR2–OR9 | — | M | petits effectifs masqués |
| **OR11** | contrat IA, mobile (socle 11), notifications | OR10 ; IA_COEUR | — | M | jeu d'évaluation ; refus attendus |
| **OR12** | formats officiels (remontée Parcoursup, import d'affectés si format) | **D-2 (inscription éditeur, certification)** ; 9 X7 | — | L | **bloqué** |
| **OR13** | RLS stricte, FK composites vérifiées, tests d'isolation, registre | tous | 2757 | S | matrice de séparation verte |

Réserve : **2753–2754** attribuées par la passe personas (§28.6), **2755, 2756, 2758** libres ; **2759** retraits (réservé, sur décision, E8). Aucune reprise de données vers le 37 (§23).

### 24.2 Ordre et calendrier (D-034, D-035)
- **V1 (janvier–février 2027)** : OR0, OR1, OR2, OR3, OR5 (sans import), OR7 (préparation et report), OR13 — de quoi tenir la campagne 2026-2027 **en doublon** sans rien transmettre.
- **Épaississement (février–août 2027)** : OR4, OR6, OR8, OR9, OR10, OR11 — **niveau production pour la rentrée 2027** pour le suivi Affelnet et Parcoursup de la campagne 2027-2028 (usage réel : ni Charlemagne ni Next.js ne le font [H]).
- **OR12** : après la démarche éditeur (F-06, vers mai 2027).

---

## 25. Questions, décisions appliquées par défaut (D-031) et points pour Florian

### 25.1 Tranchées selon la recommandation (*appliqué par défaut, D-031*)
| # | Question | Décision appliquée |
|---|---|---|
| Q-OR1 | Qui possède la fiche de dialogue (consigne de lancement : 37 ; 23 : Q-BU1) ? | **Le 23** (circuit D. 331 déjà conçu) ; le 37 lit ; consigne signalée (C1) |
| Q-OR2 | ScolIA saisit-il dans Affelnet ? | **Non** : liste de saisie, trace du report, suivi (RG-O11) |
| Q-OR3 | Fiche Avenir : saisie dans ScolIA puis report, ou rien avant certification ? | **Préparée dans ScolIA, reportée à la main**, tracée ; remontée officielle au lot OR12 |
| Q-OR4 | Vœux Parcoursup déclarés obligatoires pour l'accompagnement ? | **Facultatifs** (minimisation, autonomie de l'élève) |
| Q-OR5 | Visibilité par défaut d'une entrée du carnet | **Partagée équipe** ; privée en un geste ; famille sur choix |
| Q-OR6 | L'IA peut-elle suggérer des formations à un élève ? | **Non** (haut risque, RG-O17) ; information générale seulement |
| Q-OR7 | Devenir : questionnaire par page publique (comme Next.js) ? | **Non** : compte ou lien nominatif, aucune donnée renvoyée (R-OR22) |
| Q-OR8 | Calcul automatique d'un « risque de décrochage » pour l'orientation ? | **Non** : signaux déterministes (absences, rien suivi, sans-solution) ; jamais de score |
| Q-OR9 | Fiche Avenir visible de l'élève avant le 1ᵉʳ juin dans ScolIA ? | **Non par défaut** (calqué sur le national) ; demande d'accès traitée par la direction (droit d'accès, juriste F37-6) |
| Q-OR10 | Reprendre les « situations » de M17 (Next.js) ? | **Oui, seulement** celles de l'année en cours, avec consentement implicite de la réponse et durée §17.4 ; le reste perdable (T0 l. 121) |
| Q-OR11 | Public : fiche de dialogue tenue dans le téléservice national | **Au 23** : mode « procédure tenue dans le téléservice » (vœux reportés) — attente posée au 23 (C2) |
| Q-OR12 | Avis du chef préparé automatiquement depuis l'avis du conseil ? | **Oui en brouillon**, validé par une personne, réservés relus un à un |
| Q-OR13 | Rencontre obligatoire : une réunion collective suffit-elle ? | **Pour les présents seulement** (R-OR9) |
| Q-OR14 | Le 37 garde-t-il un catalogue de formations distinct des destinations du 23 ? | **Oui** (formation × établissement d'accueil), relié à la destination |

### 25.2 Ce qui engage Florian (non appliqué)
| # | Point | Pourquoi |
|---|---|---|
| **F37-1** | **ND-01 : page publique des invitations (Next.js, en production)** — corriger l'identification (trois critères exigés, aucun e-mail ni situation renvoyés, limitation du débit), ou désactiver le recueil de situation | Next.js est intouchable pour le pilote ; donnée personnelle exposée **aujourd'hui** |
| F37-2 | Démarche éditeur et **certification Parcoursup** (F-06, D-032) ; demander s'il existe une interface Affelnet ou un fichier de résultats | engage l'établissement à l'extérieur |
| F37-3 | Calendriers académiques Affelnet de l'académie du pilote (saisie en établissement, tours) | fait que seul l'établissement reçoit |
| F37-4 | Privé : qui fait le travail de la PsyEN (CIO, conseiller propre) ? convention avec le CIO ? | organisation et contrat |
| F37-5 | DPO : durées (§17.4), devenir des anciens élèves, questionnaire | données réelles, registre |
| F37-6 | Juriste : fiche Avenir communicable avant le 1ᵉʳ juin ? règlement IA pour toute fonction d'orientation | qualification juridique |
| F37-7 | Pratique actuelle : comment l'établissement suit-il aujourd'hui Affelnet et les sans-solution (Excel, Charlemagne) ? | calibrage du lot OR5 |

### 25.3 Réglages signalés (D-023)
| # | Réglage | Préréglage | Remarque |
|---|---|---|---|
| S37-01 | Demi-journées par niveau | 4 | RG-O04 |
| S37-02 | Types d'activités | 10 types préréglés | données de l'établissement |
| S37-03 | Visibilité par défaut du carnet | partagée équipe | |
| S37-04 | Dates de contrôle des sans-solution | 1ᵉʳ juillet, 1ᵉʳ septembre | |
| S37-05 | Questionnaire de devenir : bacheliers seulement / tous | tous les sortants | reprise de `bac_only` (M17) |
| S37-06 | Rappels de saisie Affelnet | J-7, J-1 | candidat à l'allègement |

---

## 26. Tests et recette
- **Isolation** : chaque route dans la matrice de séparation ; professeur hors groupe refusé sur parcours et fiche Avenir ; intervenant sans donnée ; deux établissements.
- **Règles** : R-OR11 (alerte sans blocage), R-OR16 (publication à la date, horloge simulée), R-OR18 (avis réservés relus), R-OR19 (sources interdites exclues : test avec une appréciation de vie scolaire et une sanction effacée), R-OR22 (aucune donnée renvoyée sans authentification), R-OR23 (majeur), R-OR26 (calcul et clôture).
- **Garde IA** : équivalence écran / IA, refus des 10 cas interdits.
- **Recette personas** : `PERSONAS_37.md` §3 rejouée sur données fictives ; campagne 3ᵉ complète (vœux du 23 → Affelnet → résultats → inscription) et terminale (fiche Avenir → report → admission → sans-proposition).

---

## 27. Faits, hypothèses, contradictions

**Faits [F]** : chemins et lignes cités relus sur `origin/ent-mc0hoe` le 2026-11-11 ; Next.js lu en lecture seule (M17 : `drizzle/0054`, `0057`, `app/lib/invitation-db.ts:645-750`, `app/api/invitation/lookup/route.ts`). Rien compilé ni exécuté.

**Hypothèses [H]** : pratiques de terrain (§1, §3), CAES, signalement à la mission de lutte contre le décrochage, deux PP en terminale.

**Contradictions**
- **C1** La consigne de lancement place les **fiches de dialogue** au 37 ; `TRANSITION_T0_INVENTAIRE.md` l. 207 aussi ; `23_BULLETINS.md` Q-BU1, `21_EVALUATIONS.md` §6.2 et `MATRICE_CHARLEMAGNE.md` NOT-17 les placent au **23**. Tranché (Q-OR1) : **23**. À reporter dans T0 l. 207 (fichier d'une autre session : signalé, non modifié ici).
- **C2** Le 23 conçoit la fiche de dialogue comme **outil de saisie des familles** ; dans le **public**, le téléservice national Orientation est l'outil officiel des familles (RG-O07) : le 23 doit prévoir le mode « tenue dans le téléservice » (report des vœux par l'établissement) ; attente posée au 23.
- **C3** Fin de la phase complémentaire Parcoursup 2027 : 9 septembre (calendrier officiel [V-r]) ou 7 septembre (Onisep [V-r]) : paramètre, à lire sur le calendrier PDF.
- **C4** `SOCLES.md` §37 cite « fiche Avenir » sous Parcoursup [H] : confirmé [V-r] ; il cite « Affelnet » au 37 : confirmé, sans interface éditeur [NV].
- **C5** `ent_voeu_inscription` (077) sert à la fois au socle 3 (vœux internes) et au conseil (181) : scindé (§2.1) ; cohérent avec `03_INSCRIPTIONS.md` l. 516 et le §21 du 23.

---

## 28. [ajout personas] Ce que change la passe personas (`PERSONAS_37.md`, 2026-11-11)

### 28.1 Bloquants tranchés (*appliqué par défaut, D-031*)
- **B37-1 Le parent hébergeant qui n'est pas titulaire** (beau-parent, famille d'accueil) : il **reçoit** le calendrier si le 4 lui donne `recevoir_informations`, il **ne déclare** ni vœu ni inscription ; l'**ASE** (tuteur) agit comme titulaire selon le 4 (R-OR29).
- **B37-2 Résultats d'affectation connus par la famille avant l'établissement** (téléservice) : la famille peut **déclarer** son résultat dans ScolIA dès la publication nationale ; la déclaration est marquée « déclarée par la famille » jusqu'à confirmation du secrétariat (R-OR30).
- **Compléments** : un **responsable de permanence estivale** (direction ou secrétariat, paramétrable) reprend les suivis des sans-solution quand le responsable désigné est absent (ORI-301, M37-29) ; dans le privé, le **conseiller d'orientation** de l'établissement suit la même règle de partage minimal que la psychologue (R-OR21), en attendant F37-4.
- **B37-3 Énumération et devinette** (persona attaquant) : toute recherche d'élève par un tiers non authentifié est **supprimée** du 37 ; les liens nominatifs ne révèlent ni l'établissement d'accueil ni le résultat avant authentification (R-OR22 renforcée, R-OR31).

### 28.2 Actions ajoutées
| Id | Action | Acteurs | Règles |
|---|---|---|---|
| ORI-370 | Déclarer un résultat d'affectation reçu par la famille | PAR | R-OR30 |
| ORI-371 | Confirmer ou corriger une déclaration de famille | ADF | R-OR30 |
| ORI-372 | Voir, pour un élève suivi par l'ASE, qui est titulaire et qui informer | PP, ADF | 4, R-OR29 |
| ORI-373 | Traduire le calendrier et les messages (langue de la famille) | SYS | 8 |
| ORI-374 | Imprimer la liste de saisie pour une famille sans numérique (avec ce qu'elle doit apporter) | ADF | — |
| ORI-375 | Prendre un rendez-vous de saisie accompagnée (famille sans ÉduConnect) | PAR, ADF | 19 |
| ORI-376 | Marquer un vœu « sous réserve de la commission d'appel » | SYS | COR-04 |
| ORI-377 | Recalculer la cohérence après la décision de la commission | SYS | — |
| ORI-378 | Lister les vœux en lycée privé sans démarche directe signalée | ADF | R-OR15 |
| ORI-379 | Accompagner un élève handicapé : convier le référent (35) à l'entretien avec accord | PP | 35 |
| ORI-380 | Indiquer qu'une formation propose un accueil adapté (information générale) | REF | sans donnée d'élève |
| ORI-381 | Déléguer la fiche Avenir d'un PP absent (avec historique) | DIR | COR-17 |
| ORI-382 | Voir l'origine de chaque phrase pré-remplie de la fiche Avenir | PP | R-OR19 |
| ORI-383 | Refuser une phrase pré-remplie (elle ne revient pas) | PP | — |
| ORI-384 | Alerte « la source a changé » sur une fiche validée | SYS | COR-18 |
| ORI-385 | Lot d'avis du chef par formation (même avis pour les vœux d'un élève vers des formations du même type) avec relecture | DIR ★ | R-OR18 |
| ORI-386 | Journal de lecture d'une fiche Avenir (qui l'a ouverte) | DIR, DPO | E7 |
| ORI-387 | Élève : demander la correction d'une appréciation de sa fiche (après consultation) | ELV, ELM | Q-OR9 |
| ORI-388 | Archiver la campagne (lecture seule) et produire la synthèse annuelle | DIR | — |
| ORI-389 | Reprendre la campagne de l'an passé (formations, dates décalées) | ADF | — |
| ORI-390 | Signaler qu'une famille n'a pas de compte ni d'e-mail (circuit papier) | ADF | 8 |
| ORI-391 | Envoyer un courrier papier de rappel de saisie | ADF | 7 |
| ORI-392 | Tableau des démarches de l'élève pour un lycée privé (date de dossier, entretien, réponse) | PP, PAR | R-OR15 |
| ORI-393 | Exporter pour l'auditeur le registre des traitements du socle (finalités, durées) | DPO | 27 |
| ORI-394 | Exporter pour une personne ses données d'orientation (droit d'accès) | ADF | 27 |
| ORI-395 | Purger selon les durées (rapport de purge) | SYS | §17.4 |
| ORI-396 | Afficher à l'élève « pourquoi je vois cette alerte » | SYS | R-OR24 |
| ORI-397 | Bloquer un intervenant (lien révoqué) | REF | — |
| ORI-398 | Signalement d'abus d'un lien nominatif (taux de tentatives) | SYS | R-OR31 |
| ORI-399 | Mode « doublon » : marquer une campagne comme tenue en parallèle de Charlemagne (aucun envoi aux familles sans accord) | DIR | D-034 |
| ORI-366 | Voir l'état des fiches Avenir de deux PP dans une vue commune | PP | COR-16 |
| ORI-367 | Joindre au dossier de suivi le compte rendu d'un échange avec le CIO | REF | R-OR28 |
| ORI-368 | Signaler une erreur dans une formation du catalogue | PP, PAR | — |
| ORI-369 | Voir combien d'élèves ont choisi une formation (sans noms) | REF | R-OR27 |
| ORI-364 | Rappeler à l'élève majeur ce qu'il partage avec sa famille (une fois par an) | SYS | R-OR23 |
| ORI-365 | Notification au second titulaire du résultat d'affectation | SYS | RG-O03 |

### 28.3 Règles ajoutées
| # | Règle |
|---|---|
| R-OR29 | Un adulte hébergeant non titulaire n'écrit rien dans le 37 ; il reçoit ce que le 4 lui ouvre. |
| R-OR30 | Une donnée nationale déclarée par la famille est marquée comme telle jusqu'à confirmation ; elle n'efface jamais une donnée saisie par le secrétariat. |
| R-OR31 | Les liens nominatifs du 37 n'affichent rien avant authentification par le lien (code ou compte) ; les tentatives répétées sont limitées et signalées. |
| R-OR32 | En mode « doublon » (D-034), aucune notification aux familles ni à l'élève sans décision explicite de la direction. |

### 28.4 Cas limites ajoutés
| # | Situation | Traitement |
|---|---|---|
| COR-47 | Famille d'accueil (ASE) et parents titulaires sans contact | titulaires informés selon le 4 ; l'ASE agit selon son mandat |
| COR-48 | Famille sans aucun moyen numérique | circuit papier, rendez-vous de saisie (ORI-374, 375) |
| COR-49 | Élève qui change d'avis après la clôture Affelnet | entretien, procédures de l'académie [NV] ; suivi |
| COR-50 | L'avis du chef prérempli contredit un avis de conseil modifié depuis | alerte, brouillon recalculé, jamais un avis validé modifié seul |
| COR-51 | Campagne tenue en doublon : la famille reçoit deux sources | mode doublon (R-OR32) |
| COR-52 | Tentatives répétées sur un lien de questionnaire de devenir | blocage, alerte (R-OR31) |

### 28.5 Questions ajoutées (*appliqué par défaut, D-031*)
- Q-OR15 : Les formations cibles saisies par un établissement sont-elles partagées avec les autres établissements ScolIA ? → **Non** (D-013) ; un catalogue commun viendra du référentiel national chargé par le 9.
- Q-OR16 : Le PP peut-il voir le carnet partagé équipe d'un élève d'un autre PP ? → **Non** : PP et professeurs de ses groupes seulement.

### 28.6 Compléments de modèle (réserve 2753–2756)
- **2753** : `ori_aff_resultat` + colonne `declare_par ('secretariat','famille')`, `confirme_par NULL`, `confirme_le NULL` (R-OR30) ; `ori_lien_tentative (id, etablissement_id, lien_id, le, issue)` (R-OR31, table volatile au registre).
- **2754** : `ori_campagne_mode (id, etablissement_id, annee_id, mode ('reel','doublon'), decide_par, decide_le)` (R-OR32) ; `ori_fiche_avenir_appreciation_source (id, etablissement_id, appreciation_id, source_appreciation_id, refusee BOOLEAN)` (ORI-382, 383).
- 2755–2756 : réserve.

### 28.7 Lots modifiés
OR0 inchangé ; OR6 reçoit ORI-370, 371, 374, 375 ; OR7 reçoit ORI-381 à 387 ; OR9 reçoit R-OR31 ; OR10 reçoit ORI-393 à 395 ; mode doublon (ORI-399) en **OR1**.
