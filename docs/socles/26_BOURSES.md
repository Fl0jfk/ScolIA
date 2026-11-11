# Socle 26 — Bourses et aides financières

**Date :** 2026-11-11 — **Statut :** conception de fond **et** passe personas, **non décidées** (à relire par Florian). Recommandations appliquées par défaut (D-031) et listées au §29. Aucun code écrit ni modifié ; aucun accès à une base, à un secret ou à un environnement de production. Next.js : recherche ciblée en lecture seule (aucun module bourses ou fonds sociaux, §2.4).
**Branche :** `ent-socle-26-conception`, partie de `origin/ent-mc0hoe` (`afeb8c4` puis mise à jour rapide).
**Passe personas :** `docs/socles/PERSONAS_26.md` (17 personas). Ses ajouts sont intégrés ici et marqués **« (personas) »** ; ils l'emportent en cas de contradiction (§30).
**À lire avec :** `CLAUDE.md` ; `docs/DECISIONS.md` (D-017, D-021, D-023, D-024, D-026, D-029, D-031, D-034, D-035, D-036, D-037 à D-046) ; `docs/SOCLES.md` §26 ; **`24_FINANCES.md`** (§5.7, §5.10, §6, §10.4, §10.5, FD-12, FD-27, R-F14, R-F19, R-F38, FAC-136 à 138, FAC-234, 235, 259 à 261) ; **`25_PAIEMENTS.md`** (reversements PAY-202, PAY-204, PAY-224) ; **`PERSONAS_24_25.md`** (P08 famille Haddad, §5 point 6) ; `42_SORTIES_VOYAGES.md` §12.4 ; `03_INSCRIPTIONS.md` ; `04_ADMINISTRATIF.md` ; `07_DOCUMENTS.md` ; `12_ASSIDUITE.md` ; `IA_COEUR.md` ; `PRA_SAUVEGARDES.md` §18.4.

**Conventions.** [F] fait vérifié dans le code (chemins depuis `ent-rust/`, commit `afeb8c4`) ; [V] règle lue sur une source officielle (§5, date de consultation) ; [V partiel] lue en partie ou sur une source secondaire officielle ; [NV] non vérifiée ; [H] hypothèse de terrain ; [R] recommandation.

**Numérotation propre (vérifiée libre sur les 134 branches distantes le 2026-11-11) :**

| Élément | Numéros |
|---|---|
| Défauts de l'existant | **BD-01**… |
| Actions | **BRS-100 à BRS-299** (conception), **BRS-300**… (personas) |
| Règles | **R-BA1**… |
| Cas limites | **CBA-01**… |
| Manques (personas) | **M-BA01**… (M-26xx est pris par le socle 12) |
| Réglages signalés | **S-2600 à S-2639** |
| Questions | **Q-BA1**… |
| Plage de migrations | **2520–2539** (imposée par le pilote ; remplace la réservation 5600–5699 / 560–579, §26) |

Sommaire : §0 En bref · §1 Thèse · §2 Existant, défauts, lot 0 · §3 Acteurs · §4 L'année vue du terrain · §5 Réglementaire · §6 Frontières et branchements · §7 Concepts · §8 Bourses nationales · §9 Fonds sociaux · §10 Aides locales, caisse de solidarité, APEL · §11 Quotient familial · §12 Voyages et sorties · §13 Familles séparées, majeurs, tiers · §14 Ce que voit chacun · §15 Cas limites · §16 IA · §17 Actions · §18 Règles · §19 Droits · §20 Événements et notifications · §21 Écrans · §22 Modèle de données · §23 Conservation · §24 Reprise · §25 Doublon 2027-2028 · §26 Lots et migrations · §27 Tests et recette · §28 Réglages signalés · §29 Décisions appliquées et questions · §30 Ajouts de la passe personas · §31 Faits, hypothèses, contradictions.

---

## 0. En bref

- **Ce que fait le socle** : il tient le **circuit de chaque aide** — information des familles, demande (téléservice national, papier, ScolIA, assistante sociale), pièces, instruction, avis, **décision** (prise par qui de droit : recteur, DASEN, chef d'établissement, collectivité, OGEC, APEL), notification, **attribution** à un **bénéficiaire nommé**, versements prévus et reçus, **déduction** sur les seules lignes autorisées (par le socle 24), **reversement** du reliquat (par le socle 25), retenues, retraits, transferts, recours, bilans. Il calcule aussi le **quotient familial** de l'établissement (le 24 n'en lit que la tranche).
- **Trois familles d'aides, trois logiques** :
  1. **Bourses nationales** (collège 3 échelons, lycée 6 échelons, primes, bourse au mérite) : l'État décide ; l'établissement informe, recueille, transmet, reçoit (public : agent comptable ; privé : **seulement avec procuration**), déduit la demi-pension ou la pension, reverse le reste, peut retenir pour absentéisme (> 15 jours injustifiés, décision motivée du chef).
  2. **Fonds sociaux de l'État** (collégien, lycéen, cantines ; circulaire 2017-122 [V]) : **public** : commission consultative anonymisée, décision du chef, aide en argent ou en nature ; **privé sous contrat** : le chef **instruit et propose**, **le recteur décide** ; fonds cantines = somme globale versée à l'établissement, **affectée** aux frais de cantine, avec **état de réalisation** et reversement du non-employé ; fonds collégien/lycéen = **versement direct de l'État à la famille** (l'établissement ne touche pas l'argent).
  3. **Aides de l'établissement et de tiers** (caisse de solidarité de l'OGEC, APEL, région, département, commune, CE, employeur) : règles locales, décision locale, imputation ou versement selon le dispositif.
- **Le bénéficiaire est une personne, jamais « l'élève » ni « le foyer »** (R-BA1). La bourse va au parent qui a la charge effective et qui a demandé ; en résidence alternée, **un seul** parent (deux demandes → irrecevables [V]) ; la déduction ne s'impute **que** sur les lignes de **ce** payeur (corrige BD-03, résidu de FD-12).
- **Cercle de confiance étroit** (R-BA2) : qui est aidé, de quoi, pourquoi, n'est connu que des personnes qui en ont besoin pour agir (instructeur, décideur, comptable pour le montant sans le motif, assistante sociale pour l'avis). Le professeur, l'organisateur de voyage, la vie scolaire, l'autre parent, les autres membres du foyer ne voient **rien** (agrégats seulement).
- **Existant [F]** : 3 tables (`ent_bourse_type`, `ent_eleve_bourse`, `ent_fin_bourse_bareme`), quotient par foyer (`ent_foyer_quotient`), 4 routes, calcul de déduction dans `finances.rs`. **9 défauts** (§2.2), dont **3 fuites** de la qualité de boursier (secrétariat, tous les parents liés, courriel et **notification push à tous les parents**) et une déduction partagée entre les payeurs. **Lot BA0** sans migration (§2.3).
- **Volume** : **172 actions** (BRS-100 à BRS-271) + **38** de la passe personas (BRS-300 à BRS-337) = **210** ; **46 règles** ; **74 cas limites** ; **lots BA0 à BA15** ; migrations **2520–2539**.
- **Pour Florian** (§29.2) : AIPD et durées avec le DPO ; modèle de procuration et clause du règlement financier (juriste) ; usage réel ou doublon de la commission fonds social pendant 2027-2028 ; interface avec les outils nationaux (SIECLE Bourses, Scolarité services) ; existence d'une caisse de solidarité de l'OGEC et d'aides APEL individuelles.

---

## 1. Thèse

**Une aide est une décision d'autrui, prouvée, adressée à une personne, et vue par le moins de monde possible.**

1. **Décision d'autrui** : presque aucune aide n'est décidée par la comptabilité. La bourse est décidée par l'État, le fonds social privé par le recteur, la caisse de solidarité par la direction, l'aide de la région par la région. ScolIA **enregistre** la décision avec sa source (notification, référence, date, pièce), il ne l'invente pas, ne la « calcule » pas à la place de l'autorité, et ne la modifie pas : une correction est une **nouvelle version** (R-BA4).
2. **Prouvée** : chaque euro déduit renvoie à une décision, chaque euro reçu à un versement de l'État ou d'un financeur, chaque euro reversé à un reliquat calculé et à un remboursement du 25. À tout moment on sait répondre à « pourquoi la demi-pension de Yasmine fait 0 € en décembre ? » et à l'inspecteur qui demande l'**état de réalisation** du fonds cantines.
3. **Adressée à une personne** : le bénéficiaire (parent demandeur, élève majeur, tuteur) est une personne du socle 4 ; la déduction, le reversement, l'avis et la notification lui sont propres. C'est la clé des familles séparées.
4. **Vue par le moins de monde possible** : la pauvreté est la donnée la plus stigmatisante qu'un établissement détienne après la santé. La circulaire impose la **discrétion**, l'**anonymisation des dossiers** de commission et du compte rendu, l'**anonymat des bénéficiaires** [V]. ScolIA en fait des **gardes techniques** (projections, pseudonymes, journal des consultations), pas des consignes.
5. **La famille agit sans honte** : elle peut demander, suivre, compléter et comprendre **en ligne, depuis son téléphone**, sans passer par un guichet ni expliquer sa situation à plusieurs personnes. L'établissement va **au-devant** (rappels de campagne à tous, jamais ciblés sur un profil supposé pauvre, R-BA17).

---

## 2. Existant

### 2.1 Ce qu'il y a [F]

| Élément | Où | Ce que ça fait |
|---|---|---|
| Types de bourse | `migrations/017_eleve_repas_bourses.sql` : `ent_bourse_type` (code, libellé, actif, par établissement) | liste libre par établissement ; aucune référence nationale |
| Bourse de l'élève | `017` : `ent_eleve_bourse` (`eleve_id`, `bourse_type_id`, `echelon TEXT`, `avec_procuration BOOLEAN`, `status` demandee / acceptee / refusee / cloturee, `date_debut`, `date_fin`) ; clé composite élève posée par `240` ; RLS par `582` | rattachée à l'**élève**, sans bénéficiaire, sans année, sans montant, sans source de décision ; échelon en texte libre |
| Barème | `migrations/200_fin_parametres_tarifs.sql:121-135` : `ent_fin_bourse_bareme` (année, type, échelon, montant **annuel** saisi par l'établissement) | montant national ressaisi à la main |
| Ventilation sur échéances | `033_echeance_bourse.sql` : `ent_fac_echeance.bourse_centimes` | part de bourse par échéance |
| Quotient | `075_quotient_tva.sql` : `ent_foyer_quotient` (foyer, année, quotient, parts ; `ON DELETE CASCADE` depuis le foyer ; une ligne par foyer et année, **écrasée**) ; `domain-eleve/src/quotient.rs` | quotient du **lieu de vie**, pas du payeur ; aucune pièce |
| Calcul | `ent-db/src/finances.rs:1052-1105` | bourse acceptée **avec procuration** → ligne négative au prorata des mois, **répartie entre les foyers payeurs** (`calcul::repartir`), plafonnée depuis F0 aux lignes restauration et internat de chaque foyer (FD-12 partiel) |
| Commandes | `ent-db/src/service.rs:10627-10809` : `list_bourse_types_for_actor`, `creer_bourse_type`, `affecter_bourse`, `decider_bourse` (accepter / refuser), `bourses_deductibles_eleve` | création et décision **par mise à jour en place** (`upsert_bourse`) |
| Routes | `ent-api/src/routes.rs:323-327` : `GET/POST /bourses/types`, `POST /eleves/{id}/bourses`, `POST /bourses/{id}/decider` ; section `bourses` du dossier élève (`routes.rs:1563`) | droit `bourses.gerer` (Admin, Direction, Compta depuis F0, `ent-auth/src/roles.rs:216`) |
| Événements | `ent-events/src/lib.rs:83-85` : `bourse.type_created`, `eleve.bourse_affectee`, `eleve.bourse_decidee` | charge utile avec **échelon** et procuration |
| Notification | `service.rs:10742-10785` | à la décision : **courriel et push à tous les responsables** de l'élève (« Décision bourse — Prénom Nom : la demande de bourse … est acceptée ») |
| Interface | `ent-api/src/ui.rs` (222 occurrences de « bourse ») ; Leptos : compteur « Bourses » de la fiche (`web/crates/ent-web-app/src/modules/dossier/modele.rs:286`) | F-001 : ne pas étendre `ui.rs` |
| Registre E12 | `ent-db/src/registre_donnees.rs:266, 309, 373` | 3 tables classées « finances », `ent_eleve_bourse` personnelle et exportée |

### 2.2 Défauts (BD-01 à BD-09)

| # | Gravité | Défaut | Conséquence | Où [F] | Lot |
|---|---|---|---|---|---|
| **BD-01** | **Élevée** | Le **secrétariat** (rôle Administratif, droit `eleves.gerer`) reçoit la section `bourses` du dossier élève | toute la scolarité lit qui est boursier et à quel échelon ; contraire à R-F38 et à la discrétion de la circulaire [V] ; FD-27 n'a corrigé que les droits financiers, pas la section du dossier | `service.rs:4680-4694` (`role_can_manage_eleves` → `bourses: true`) | **BA0** |
| **BD-02** | **Élevée** | **Tout parent lié** voit la section `bourses` (y compris le parent non demandeur, non payeur, sans droit de consultation élargi) | en résidence alternée, l'autre parent apprend la bourse, l'échelon (donc une tranche de revenus) et la procuration de son ex-conjoint | `service.rs:4761-4781` (`Role::Parent` → `bourses: true`) | **BA0** |
| **BD-03** | **Élevée** | **Notification de la décision par courriel et par push à tous les responsables**, avec le nom de l'élève et « bourse acceptée / refusée » ; hors des natures du socle 8 | fuite à l'autre parent, au tiers lié, sur l'écran verrouillé d'un téléphone partagé ; un refus de bourse affiché en notification | `service.rs:10742-10785` | **BA0** |
| **BD-04** | **Élevée** | **Déduction répartie entre les foyers payeurs** (`calcul::repartir(du, &parts)`) au lieu d'être imputée au seul **bénéficiaire** | l'autre parent profite de la bourse du premier (résidu de FD-12 après F0) ; avec une répartition 50/50, la moitié de la bourse disparaît si l'autre foyer n'a pas de demi-pension | `finances.rs:1084-1104` | **BA0** (règle provisoire), BA3 |
| BD-05 | Moyenne | **Procuration réduite à un booléen**, sans pièce, sans signataire, sans date | déduction faite sans preuve de la procuration (privé, C. éduc. R. 531-34 [V]) ; impossible de répondre à une contestation | `017` | BA2 |
| BD-06 | Moyenne | **Décision écrasée** (`upsert`) : aucune version, aucun motif, aucune référence de notification, aucune année ; échelon en **texte libre** ; aucun montant propre à la décision (le barème de l'établissement fait foi) | on ne sait pas quelle décision s'appliquait le jour d'une facture ; échelon « 7 » ou « cinq » accepté ; primes, bourse au mérite et retenues impossibles à représenter | `017`, `service.rs:10716-10741` | BA1, BA2 |
| BD-07 | Moyenne | **Barème national ressaisi par chaque établissement** (montant annuel par type et échelon) | erreur de saisie = mauvaise déduction pour toutes les familles ; aucun lien avec les montants publiés (§5) | `200:121-135` | BA1 |
| BD-08 | Moyenne | **Quotient familial par foyer (lieu de vie)**, écrasé chaque année, sans pièce, `ON DELETE CASCADE` | un parent séparé hérite du quotient de l'autre foyer ; on ne sait pas d'où vient la tranche ; une suppression de foyer efface la preuve | `075` | BA8 |
| BD-09 | Faible | **Charge utile des événements** avec échelon et procuration ; types de bourse lisibles par tout rôle qui lit tous les élèves (CPE…) | l'échelon (indice de revenus) se retrouve dans le journal des faits et la chronologie ; minimisation | `service.rs:10627-10641`, `:10694-10698` | BA0 (charge utile), BA1 |

### 2.3 Lot BA0 — correctifs urgents (aucune migration, code seulement)

À faire **avant toute autre chose** du domaine, et avant la démo de décembre si la fiche élève est montrée avec des données de labo :
1. **BD-01** : section `bourses` du dossier **fermée** au rôle Administratif et à tout rôle sans `bourses.gerer` ; le compteur « Bourses » de la fiche Leptos disparaît pour eux (pas un zéro : l'absence de tuile).
2. **BD-02** : section `bourses` **fermée aux parents** dans le dossier ; la famille verra sa bourse **dans son compte financier** (FAC-259 du 24), où seul le payeur bénéficiaire la voit ; d'ici là, rien.
3. **BD-03** : **suppression** du courriel et du push automatiques ; la décision crée une **tâche** pour la comptabilité (« informer le bénéficiaire ») ; aucune notification ne nomme une bourse tant que le socle 8 n'a pas la nature `aide.decision` adressée au **seul** bénéficiaire (BA2).
4. **BD-04** (règle provisoire) : si l'élève a **plus d'un foyer payeur** sur les catégories restauration ou internat, **aucune déduction automatique** ; l'aperçu de génération signale « bourse non déduite : bénéficiaire à désigner » (tâche à la comptabilité). Un seul foyer payeur : comportement actuel conservé.
5. **BD-09** : charges utiles réduites à `{statut}` (ni échelon, ni procuration) pour les nouveaux événements ; les anciens restent (instantanés, D-036).
6. **Tests** : quatre tests de non-régression (secrétariat sans section ; parent sans section ; décision sans envoi ; deux foyers payeurs → aucune ligne de bourse et une anomalie).

### 2.4 Ce que fait l'application Next.js de production

Recherche ciblée en lecture seule le 2026-11-11 (`grep` sur `app/`, `db/`, `drizzle/`, `types/`) : **aucun module bourses ni fonds social**. Trois mentions seulement : la fiche RGPD « facturation… bourses » (`app/lib/rgpd-processing-fiches.ts:497, 504`), la balise `BOURSE` / `CODE_BOURSE` de l'import des nomenclatures SIECLE (`app/lib/nomenclature-import/siecle-nomenclature-parse.ts:34`) et, dans le module voyages (lu par la conception du socle 42), les **aides individuelles nominatives** de la fiche budget (NX-51, défaut VD-37 de `42_SORTIES_VOYAGES.md`). **À ne pas perdre** : la synthèse annuelle APEL des aides individuelles (NX-58), reprise par le 42 et alimentée par le 26 (§12). Rien d'autre à reprendre (D-035 : pas d'ébauche à refaire).

### 2.5 Verdict
- **Garder** : la règle « pas de procuration, pas de déduction » ; le plafonnement aux lignes restauration et internat (F0) ; le principe de proposition sans émission.
- **Corriger** tout de suite : BA0.
- **Remplacer** : `ent_eleve_bourse`, `ent_bourse_type`, `ent_fin_bourse_bareme`, `ent_foyer_quotient` par la brique `aides` (§22), avec reprise (§24) et retrait en 2539 sur décision (E8).

---

## 3. Acteurs, besoins réels, irritants

| Acteur | Ce qu'il fait | Ce qui lui fait perdre du temps | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Chef d'établissement** (public) / **chef d'établissement et OGEC** (privé) | Informe, préside la commission, décide (public) ou propose au recteur (privé), décide la retenue, la caisse de solidarité, l'exonération d'un voyage | Dossiers incomplets, compte rendu à rédiger, enveloppe dont on ne connaît pas le reliquat, signer dix propositions une à une | Un élève qui ne mange plus faute d'argent ; un reliquat de fonds social à rendre alors que des familles en avaient besoin ; une fuite (« la fille du boucher est boursière ») | Un tableau : enveloppe, engagé, versé, reliquat, dossiers en attente ; des propositions préparées et signées en lot par passkey |
| **Adjoint gestionnaire / comptable** (public : régisseur et agent comptable ; privé : comptable de l'OGEC) | Reçoit les versements, déduit, reverse, rapproche, établit l'état de réalisation, les mandats (6576), le bilan | Rapprocher un virement global de l'État avec 40 élèves ; recalculer à chaque changement de régime ; reverser à la main | Déduire sans procuration ; reverser au mauvais parent ; un contrôle qui ne retrouve pas l'emploi d'une subvention | Versement reçu → ventilation proposée → déduction et reversement en un geste ; état de réalisation prêt |
| **Secrétariat de scolarité** | Distribue l'information, reçoit les dossiers papier, délivre l'accusé de réception, relance les pièces, transmet | Photocopies, dossiers incomplets, familles qui demandent « où en est mon dossier ? » | Perdre un dossier ; être le témoin involontaire de la situation des familles | Un registre de dépôt avec accusé imprimé ; une liste de pièces manquantes **sans** voir les revenus ; une réponse « où en est » que la famille lit seule |
| **Assistante de service social** (souvent partagée entre plusieurs établissements [H]) | Évalue, accompagne la demande, donne un **avis technique**, oriente vers d'autres aides | Ressaisir la situation dans un formulaire ; être sollicitée hors de ses jours | Que son évaluation (secret professionnel) se retrouve dans le dossier lu par la commission | Déposer un avis codé (favorable, réservé, priorité) sans narration ; voir ses dossiers sur ses seuls établissements |
| **CPE, vie scolaire** | Repère (élève qui ne déjeune plus, absences), alerte | Ne pas savoir à qui signaler sans exposer | Mettre en cause une famille ; apprendre ce qu'il ne doit pas savoir | **Signaler un besoin** (« à regarder ») qui part à l'assistante sociale et au chef, sans en connaître la suite (R-BA21) |
| **Professeur principal** | Relaie l'information à toutes les familles (circulaire [V]) | Distribuer des papiers, collecter | Être perçu comme celui qui sait | Un rappel unique envoyé à toutes les familles de la classe par l'établissement ; aucun nom en retour |
| **Organisateur de voyage** | Monte le budget, annonce le coût | Ne pas savoir combien d'aides arriveront | Un élève qui renonce pour l'argent | Le total des aides accordées et le reste à charge agrégé, jamais les noms (R-SV18) |
| **Membres externes de la commission** (parents élus, délégués des élèves) | Donnent un avis | Lire des dossiers épais | Reconnaître une famille | Dossiers **pseudonymisés** (« D-2026-014 »), sans nom, adresse, classe identifiante ; engagement de discrétion signé |
| **Famille** (dont séparée, allophone, sans compte) | Demande, fournit les pièces, suit, reçoit, comprend | Comprendre quelle aide demander, à qui, quand ; les pièces ; attendre | Être repérée ; perdre la bourse pour un papier ; que l'autre parent sache | Un parcours « aides » en ligne : ce à quoi j'ai sans doute droit (simulateur public, sans enregistrement), demander, suivre, voir la déduction et le reversement |
| **Élève mineur** | Peut solliciter lui-même le fonds social (lycéen surtout [H]) | Ne pas oser | Que ses camarades sachent | Un bouton discret « j'ai besoin d'aide » qui va à l'assistante sociale |
| **Élève majeur** | Bénéficiaire direct possible (bourse s'il n'est pas rattaché au foyer fiscal de ses parents ; fonds social) [V] | Démarches seules | Que ses parents soient informés malgré lui | Être le seul destinataire de ses décisions |
| **Services académiques, inspection** | Décident (privé) ; contrôlent l'emploi des fonds | États incomplets | Fonds non employés non reversés | Bilan et état de réalisation exacts, sans nom quand le nom n'est pas requis |
| **Collectivités, APEL, OGEC** | Financent des aides | Justifier | Double financement | Synthèse par financeur et par dispositif |

---

## 4. L'année vue du terrain [H, à confirmer avec Florian]

| Moment | Bourses | Fonds sociaux et aides | Ce que fait ScolIA |
|---|---|---|---|
| **Avril – juin** | Inscriptions et réinscriptions : **accord pour l'étude automatique** du droit à bourse, recueilli via le téléservice ou le formulaire d'inscription [V] | Préparation du règlement de la caisse, des critères (avis du CA au public pour le fonds cantines [V]) | Case et état civil du concubin éventuel dans le formulaire d'inscription (socle 3) ; information |
| **Juillet** | Notifications de la campagne de printemps (lycée) [V partiel] | Notification de l'enveloppe [H] | Saisie des décisions reçues, procurations |
| **1er sept. – 15 oct.** | Campagne de rentrée collège et lycée : demandes en ligne ou papier [V] ; il faut redemander chaque année [V] | Information de toutes les familles (circulaire [V]) | Rappels à **toutes** les familles ; registre des dépôts papier ; accusés ; pièces manquantes |
| **Octobre – décembre** | Réponses au 1er trimestre [V] ; recours (collège : 2 mois ; lycée : 15 jours francs) [V] | Premières commissions ; urgences | Décisions, attributions, déduction **prévue** sur l'échéancier |
| **Fin décembre, fin mars, fin juin** | Versements trimestriels [V] | — | Réception, ventilation, déduction **constatée**, reversement du reliquat |
| **Toute l'année** | Retenue pour absentéisme > 15 jours injustifiés ; perte de la charge de l'enfant ; changement d'établissement | Changements de situation (chômage, séparation) : fonds social **en attendant** la campagne suivante [V] | Alertes, propositions, tâches |
| **Juin – juillet** | Clôture de l'année | Bilan de l'utilisation (public) ; recensement par **année civile** (Orquestra) [V] ; état de réalisation (privé) [V] | Bilans calculés, documents |

---

## 5. Réglementaire (vérifié le 2026-11-10 et le 2026-11-11)

### 5.1 Bourses nationales [V]
Sources : service-public.gouv.fr, fiches **F984** (bourse de collège) et **F616** (bourse de lycée), « vérifiées le 30 avril 2026 », consultées le **2026-11-11** ; Code de l'éducation, articles cités dans `PERSONAS_24_25.md` §5 point 6 (consultés le 2026-11-10 sur Légifrance, section `LEGISCTA000020743143`).

1. **Collège** : 3 montants trimestriels en 2026-2027 : **40 €, 112 €, 175 €** (soit 120, 336, 525 € par an) ; ressources = **revenu fiscal de référence** de l'avis 2026 (revenus 2025) ; plafond selon le nombre d'enfants à charge (1 enfant : 18 488 € ; 2 : 22 754 € ; 3 : 27 020 € ; 4 : 31 286 € ; 5 : 35 553 € ; 6 : 39 819 € ; 7 : 44 085 € ; 8 et plus : 48 352 €) [V, F984].
2. **Lycée** : **6 échelons**, montants trimestriels 2026-2027 : 168, 207, 244, 282, 319, 357 € (504 à 1 071 € par an) [V, F616] ; **prime d'internat** 327 à 672 € et **prime d'équipement** 341,71 € (2026-2027) [V, `PERSONAS_24_25.md`] ; **bourse au mérite** pour les boursiers ayant une mention bien ou très bien au DNB, sans demande séparée, versée avec la bourse ; montants annuels 402 à 1 002 € selon l'échelon [V partiel : barème du ministère relayé par un lycée académique, colonne non relue sur l'original] ; aides propres à la voie professionnelle [NV : non détaillées].
3. **Demande** : accord pour l'**étude automatique** donné à l'inscription ou à la réinscription, qui **vaut demande** (état civil du demandeur et du concubin pour récupérer les données fiscales) ; sinon demande en ligne (ÉduConnect) ou papier **du 1er septembre au 15 octobre 2026** ; à refaire chaque année ; réponse au cours du 1er trimestre [V, F984, F616].
4. **Charge de l'enfant** : le demandeur doit avoir la **charge effective et permanente** (entretien, responsabilité affective et éducative) ; s'il la perd en cours d'année, **le versement lui est retiré** et le nouveau responsable peut demander [V, F984]. **Parent isolé** : ses seules ressources ; **concubinage** : ressources du couple ; **résidence alternée** : les parents **choisissent** lequel bénéficie ; **deux demandes → irrecevables** ; **il est interdit de fournir une attestation de l'autre parent** s'engageant à ne pas demander [V, F984].
5. **Versement** : en 3 fois, à la fin de chaque trimestre (généralement fin décembre, fin mars, fin juin) [V]. **Public** : versée par l'**agent comptable**, **après déduction** des frais de demi-pension et de pension [V, F984 ; R. 531-33]. **Privé sous contrat** (et hors contrat habilité) : versée par les **services académiques** à la famille ; avec une **procuration**, l'établissement la perçoit au nom de la famille et lui verse **le solde après déduction** des frais de pension ou de demi-pension [V, F984, F616 ; R. 531-34]. **Boursier majeur** : peut être payé lui-même (R. 531-35) ; un élève majeur ou émancipé peut demander s'il n'est pas rattaché au foyer fiscal de ses parents [V, F616].
6. **Retenue** : bourse conditionnée à l'**assiduité** ; au-delà de **15 journées** d'absences **injustifiées et répétées** depuis le début de l'année, **retenue possible** sur le montant annuel ; **décision du chef d'établissement, justifiée** [V, F984 ; R. 531-31].
7. **Recours** : collège : **2 mois** après la notification, recours administratif devant le chef d'établissement ou le recteur, puis le juge [V, F984] ; lycée : **15 jours francs** auprès du rectorat ou de la DSDEN, puis 2 mois auprès du ministre ou du tribunal [V, F616].
8. **Qui décide la bourse de collège au public** : la fiche oriente le recours vers « le directeur du collège ou le recteur » [V] ; la répartition exacte des compétences (chef d'établissement ou DSDEN) n'a pas été relue dans le code [NV] → ScolIA enregistre le **décideur** comme une donnée de la décision, sans le figer.
9. **Enseignement agricole** : bourses gérées par le ministère de l'agriculture (formulaire cerfa propre) [V partiel, documents d'établissements agricoles] ; même modèle de données, dispositif distinct (§8.6).

### 5.2 Fonds sociaux [V]
Source : **circulaire n° 2017-122 du 22-8-2017**, BO n° 28 du 31 août 2017, pages 33 à 36, PDF officiel `cache.media.education.gouv.fr/file/28/41/3/BO-MEN-28-OK_803413.pdf`, lu intégralement le **2026-11-11** ; toujours appliquée [V partiel : chartes d'établissements de 2020 à 2026 et réponse ministérielle qui s'y réfèrent].

1. **Fonds social pour les cantines** : faciliter l'accès à la restauration ; gratuité **exceptionnelle et temporaire** ; information de toute la communauté au début de l'année ; dossier **simple**. **Public** : le chef décide après avis de la communauté (assistant de service social, CPE, infirmière, adjoint gestionnaire), sur des **critères soumis à l'avis du conseil d'administration** ; l'aide **vient en déduction** du tarif ; **jamais versée directement à la famille** ; notifiée au responsable ; comptes 44116, 6576 (mandats pour ordre), 7411. **Privé** : le chef **instruit** et **propose au recteur** ; les sommes sont versées **au représentant légal de l'établissement**, qui les **affecte** aux frais de cantine des élèves concernés ; **état de réalisation** pour chaque versement ; **sommes non employées reversées** ; programme 139 (codes 00ASFS01 collège, 00ASFS02 lycée) ; l'établissement doit signaler ses **mécanismes de solidarité interne** et les aides des collectivités.
2. **Fonds social collégien et lycéen** : dépenses de scolarité et de vie scolaire : vêtements de travail, matériel professionnel ou de sport, manuels (lycée), fournitures ; **transports et sorties scolaires** (« éviter toute forme d'exclusion ») ; **besoins élémentaires** : soins bucco-dentaires, lunettes, appareils auditifs ou dentaires ; liste **non limitative** ; maintien en formation après un échec à l'examen. Si le fonds cantines est insuffisant, la restauration peut être prise sur ces fonds.
3. **Commission (public)** : constituée par le chef, **sous sa présidence**, pouvant comprendre CPE, adjoint gestionnaire, assistant de service social, infirmière, délégués des élèves, délégués des parents, autres membres ; **dossiers anonymisés**, **compte rendu anonymisé**, **discrétion**, **anonymat des bénéficiaires** ; le chef **arrête la décision** au vu de l'avis ; **en urgence**, il accorde sans commission et l'**informe a posteriori** ; aide **en argent ou en nature** ; allouée à la famille ou au représentant légal ; **élève majeur : directement**.
4. **Privé (collégien, lycéen)** : le chef instruit et **formule des propositions** ; propositions et dossiers **transmis au recteur**, qui attribue dans la limite de l'enveloppe ; le chef est informé de la décision ; **concours financier direct à la famille** (virement par la DDFIP/DRFIP au vu de l'état de liquidation du recteur).
5. **Bilans** : public : bilan global en fin d'année ; **recensement du nombre de bénéficiaires et des montants par année civile** (enquête Orquestra) ; privé : les services académiques doivent pouvoir faire ce bilan (l'établissement fournit).
6. **Rôle du service social** (circulaire 2017-055 citée) : avis technique sur les situations, participation à la définition des modalités ; changements de situation professionnelle après le 1er janvier non pris en compte par la bourse → **fonds social privilégié** en attendant [V].
7. **Secret professionnel** de l'assistant de service social (code de l'action sociale et des familles, art. L. 411-3 ; code pénal 226-13) [NV dans cette passe : textes connus, non relus] → R-BA9.

### 5.3 Autres aides et règles voisines
- **Sorties et voyages** : circulaire du 13/06/2023 : la contribution aux sorties facultatives ne doit en aucun cas conduire à l'exclusion d'un élève pour raisons financières ; aucune participation pour un déplacement obligatoire ; surcoût lié au handicap jamais imputé [V, `PERSONAS_24_25.md` §5 point 8].
- **Remise de principe** (public) : abrogée depuis le 01/09/2016 ; remise d'ordre selon le règlement de la collectivité [V, idem point 4]. → aucune « remise de principe » dans ScolIA.
- **Aides des collectivités** (régions pour les lycées, départements pour les collèges : restauration, tarification sociale, équipement, transport) : variables, non recensées [NV] → dispositifs locaux paramétrables (§10).
- **Allocation de rentrée scolaire** (CAF) : hors établissement ; ScolIA ne la gère pas, il peut l'**indiquer** dans le guide des aides (§17.10).
- **Données** : référentiel CNIL des impayés (accès habilités et tracés) [V, 24] ; durées de conservation des dossiers d'aide [NV] → DPO (Q-BA1) ; AIPD probablement requise (données financières et sociales de mineurs à grande échelle [H, liste CNIL NV]) → Q-BA2.

### 5.4 Ce que le réglementaire impose au logiciel [R]
1. Un **bénéficiaire unique** par bourse (R-BA1) ; jamais d'attestation de non-demande demandée à l'autre parent (R-BA6).
2. **Pas de déduction sans procuration** dans le privé, déduction **par défaut** dans le public (R-BA12).
3. **Retenue** = décision **justifiée** du chef, après 15 jours d'absences injustifiées, jamais automatique (R-BA14).
4. **Commission** : dossiers et compte rendu **pseudonymisés** par construction (R-BA8) ; décision d'urgence possible avec information a posteriori (R-BA10).
5. **Privé** : le chef ne « décide » ni le fonds collégien/lycéen ni le fonds cantines : il **propose** ; ScolIA garde la proposition, la décision du recteur, et ne fait aucun versement des fonds collégien/lycéen (R-BA11).
6. **Fonds cantines** : affectation **exclusive** aux frais de restauration (R-BA13) ; état de réalisation ; reliquat reversé.
7. **Bilans par année civile** (R-BA26) alors que tout le reste est par année scolaire.

---

## 6. Frontières et branchements sur le cœur

### 6.1 Qui possède quoi

| Sujet | Possède | Le 26 |
|---|---|---|
| Élève, scolarité, régime à une date | 2 | lit (`eleve::scolarite_a`) |
| Inscription, réinscription, **accord pour l'étude automatique** | 3 | **demande** au 3 un champ et une pièce ; lit l'accord |
| Personnes, foyers, responsables, **charge effective**, résidence, autorité, majeur protégé | 4 | lit ; le bénéficiaire est une personne du 4 ; jamais d'écriture |
| Structure, année, calendrier, trimestres | 5 | lit |
| Personnels (assistante sociale partagée, membres de commission) | 6 | lit |
| Pièces (avis d'imposition, procuration, RIB, devis), documents émis (accusé, notification, état de réalisation, compte rendu) | 7 | **demande** ; domaine documentaire **restreint** `aides` |
| Avis et notifications | 8 | déclare ses natures (§20.2) |
| Interfaces nationales (SIECLE, Scolarité services) | 9 | demande des imports (Q-BA6) |
| Absences injustifiées (retenue) | 12 | lit un **compte** de demi-journées injustifiées, jamais les motifs |
| Formulaires de demande | 18 | le 26 a son propre parcours (pièces typées), le 18 sert aux questionnaires libres de l'établissement |
| Lignes, déduction, aide affectée, régularisation | **24** | **demande** (`facturation::facturer_source`, idempotent) ; lit les lignes cibles |
| Réceptions, reversements, remboursements, rapprochement | **25** | **demande** (`paiements::reverser`, `paiements::rattacher_reception`) |
| À-faire | 29 | déclare ses natures de tâche |
| Suivi social (narration, informations préoccupantes) | 36 | **ne lit jamais** ; reçoit un **avis codé** de l'assistante sociale |
| Restauration, internat | 40, 41 | lit le régime et l'existence de lignes, jamais les consommations détaillées |
| Voyages | 42 | demande d'aide depuis le séjour ; renvoie des **agrégats** |
| Écritures (6576, produits, tiers) | 48 | produit les écritures d'aides (via 24/25) |

### 6.2 Règles de frontière [R]
1. Le 26 **n'écrit jamais** dans `ent_fac_*`, `ent_fin_*` (24, 25) : il **demande**. Le 24 n'interprète jamais une décision : il reçoit une **aide affectée** (montant, période, lignes cibles, payeur bénéficiaire, clé de source `aide:{attribution}:{trimestre}`).
2. Le 24 ne connaît **ni le motif, ni le dispositif détaillé** pour les aides sociales : libellé « Aide n° A-2026-014 » côté comptable ; la famille bénéficiaire voit le libellé clair (« Bourse de lycée, 1er trimestre »).
3. Le 26 **ne stocke pas** le revenu fiscal de référence pour les bourses nationales (l'État calcule) ; il ne stocke des ressources que pour les **dispositifs propres** de l'établissement (quotient, caisse), et seulement les grandeurs nécessaires (§11, R-BA19).
4. Le 26 **ne lit jamais** le socle 36 ni la santé (34) ; une aide « lunettes » est codée `besoin_elementaire`, sans diagnostic (R-BA20).

### 6.3 Répercussions automatiques (toile des événements)
| Fait d'un autre socle | Effet au 26 |
|---|---|
| `scolarite.depart_*` (3), changement d'établissement | attribution **suspendue** à la date ; tâche « transfert de bourse » (§8.5) ; prochain versement non déduit |
| `eleve.regime_changed` (2) | recalcul des lignes cibles ; excédent prévisionnel recalculé ; reversement proposé si l'élève devient externe |
| `personnes.charge_modifiee`, `residence_modifiee`, décès (4) | attribution **à revoir** (tâche, jamais retrait automatique) ; un changement de payeur ne transfère **jamais** la bourse à l'autre parent (R-BA3) |
| `eleve.majeur` (2) | bénéficiaire inchangé (la décision court) ; l'élève majeur devient destinataire de ses nouvelles demandes |
| Seuil de 15 demi-journées ou journées injustifiées atteint (12) | **proposition** de retenue au chef (jamais automatique) |
| `aide.accordee` (26) | 24 : ligne d'aide ; 42 : agrégat du séjour ; 29 : tâche « informer » ; 8 : avis au seul bénéficiaire |
| Versement reçu (25, relevé bancaire) | ventilation proposée par attribution et trimestre |

---

## 7. Concepts et vocabulaire [R]

| Terme | Définition |
|---|---|
| **Dispositif** | Une sorte d'aide : référence nationale (bourse de collège, de lycée, prime d'équipement, prime d'internat, bourse au mérite, fonds social collégien, lycéen, cantines, bourse agricole) ou locale (caisse de solidarité de l'OGEC, aide APEL voyage, aide restauration de la région…) ; porte le **financeur**, le **décideur**, le **mode de versement** et les **lignes cibles** permises |
| **Campagne** | Fenêtre de demande d'un dispositif pour une année (dates, statut) |
| **Demande** | Une demande d'aide pour un élève, portée par un **demandeur** (personne), par un **canal** ; numérotée `D-2026-0014` par établissement |
| **Pièce** | Document typé rattaché à une demande (socle 7, domaine restreint) |
| **Instruction** | Contrôle de complétude, recevabilité, avis ; produit une **proposition** (privé) ou un dossier pour la **commission** (public) |
| **Avis social** | Avis **codé** de l'assistante sociale (favorable, réservé, sans avis ; priorité), sans narration |
| **Commission** | Séance consultative ; examine des dossiers **pseudonymisés** ; rend des avis ; produit un compte rendu anonymisé |
| **Décision** | Acte de l'autorité compétente : accord (échelon, montant, primes), refus (motif codé), ajournement ; versionnée ; avec voies et délais de recours |
| **Attribution** | Le droit en cours qui découle d'une décision : bénéficiaire, élève, période, montant annuel, **mode de versement** (déduit par l'établissement, versé par l'établissement, versé directement par l'État, en nature) ; états : active, suspendue, retirée, transférée, close |
| **Procuration** | Pièce signée par le bénéficiaire qui autorise l'établissement privé à percevoir la bourse ; datée, révocable |
| **Échéance prévue** | Montant attendu par trimestre |
| **Réception** | Argent reçu d'un financeur (virement de l'État, d'une collectivité), ventilé sur des attributions |
| **Déduction** | Imputation d'une part de l'aide sur des lignes cibles du **payeur bénéficiaire** (faite par le 24) |
| **Reliquat** | Part reçue non déduite → **reversement** au bénéficiaire (fait par le 25) |
| **Retenue** | Diminution décidée et motivée d'une bourse (absentéisme) |
| **Enveloppe** | Crédits d'un dispositif pour une période (notifiés, engagés, consommés, reliquat) |
| **État de réalisation** | Document du privé justifiant l'emploi d'un versement du fonds cantines |
| **Signalement de besoin** | Message d'un personnel (« à regarder ») vers l'assistante sociale et le chef, sans retour sur la suite |
| **Quotient familial** | Calcul propre de l'établissement (ressources / parts) produisant une **tranche** par **payeur** |

---

## 8. Bourses nationales

### 8.1 Information et campagne
- **Campagnes** créées automatiquement chaque année à partir des dates nationales (référence globale, §22.1) ; l'établissement peut **avancer** sa date limite interne de dépôt papier (S-2600), jamais au-delà de la date nationale.
- **Information de toutes les familles** : un avis du socle 8 à **tous** les responsables de **tous** les élèves (jamais ciblé, R-BA17), traduit (langues du socle 8), avec : ce que sont les bourses, le simulateur officiel (lien), la date, comment demander (étude automatique, en ligne, papier), qui contacter, la promesse de discrétion ; relais par le professeur principal **sans liste** (il voit seulement « l'information a été envoyée à 31 familles de la classe »).
- **Étude automatique** : le formulaire d'inscription et de réinscription du socle 3 porte une **case d'accord** et l'état civil du concubin éventuel [V] ; l'accord est une donnée du 26 (demande au canal `etude_automatique`, statut « transmis à l'administration ») ; sa transmission à l'outil national dépend du socle 9 (Q-BA6) ; en attendant, **liste d'export** pour le secrétariat de scolarité (action réservée, journalisée).

### 8.2 Dépôt papier et suivi
- **Registre des dépôts** : le secrétariat enregistre le dépôt (élève, demandeur, date, pièces présentes : cases), imprime un **accusé de réception** daté et numéroté (socle 7) ; il ne saisit **pas** les revenus.
- **Pièces manquantes** : liste par demande, relance **au demandeur seul** ; le secrétariat voit « pièce manquante : avis d'imposition », jamais son contenu une fois numérisé (le contenu est visible du seul instructeur habilité, S-2601).
- **Double demande** (résidence alternée) : si deux demandes existent pour le même élève et le même dispositif, ScolIA **alerte l'instructeur** (sans prévenir l'autre parent) et propose d'**informer chaque demandeur séparément** de la règle (deux demandes → irrecevables [V]) pour qu'ils retirent l'une d'elles ; **jamais** d'attestation de l'autre parent (R-BA6).
- **« Où en est ma demande ? »** : la famille (demandeur) voit les étapes (déposée, complète, transmise, décidée) dans son espace et sur l'appli.

### 8.3 Décision, attribution, procuration
- **Saisie de la décision** (ou import, Q-BA6) : décideur, date, référence de la notification, **échelon** (contrôlé selon le dispositif : 1–3 collège [R : le montant suffit], 1–6 lycée), **montant annuel** (pré-rempli depuis le barème national de référence, modifiable seulement avec motif, BD-07), primes (internat, équipement, mérite) en lignes, pièce (notification scannée), voies et délais de recours (affichés dans l'avis au bénéficiaire).
- **Attribution** créée : bénéficiaire = demandeur (modifiable par l'instructeur avec motif si la notification nomme une autre personne) ; **mode de versement** : public → `etablissement_deduit` ; privé avec procuration → `etablissement_deduit` ; privé sans procuration → `direct_etat` (aucune déduction, l'établissement **sait** seulement pour informer la famille, et peut même ne rien enregistrer : S-2602, défaut « enregistrer pour le suivi seulement si la famille fournit la notification »).
- **Procuration** (privé) : modèle fourni (trame D-023), signée par le bénéficiaire (signature du socle 7, en ligne ou papier scanné), datée, pour une ou plusieurs années (S-2603, défaut : l'année), **révocable** à tout moment par le bénéficiaire (effet au prochain trimestre non encore versé). Sans pièce valide, **aucune déduction** (BD-05).
- **Prévision** : échéances prévues par trimestre (montant annuel / 3, arrondi au centime, reste sur le 3e) ; la **déduction est prévue** dès l'échéancier (S-159 du 24 : anticipée) et **constatée** à la réception.

### 8.4 Réception, déduction, reliquat
1. Le 25 enregistre le virement de l'État (souvent **global** pour plusieurs élèves [H]).
2. Le 26 **propose** une ventilation : attributions actives du trimestre, montants prévus ; écarts signalés (élève en plus, en moins, montant différent) ; la comptabilité **valide** (N2).
3. Pour chaque attribution : **déduction** demandée au 24 à hauteur des lignes `demi_pension` / `internat` **du payeur bénéficiaire** pour la période couverte ; **reliquat** = reçu − déduit ; **reversement** demandé au 25 vers le **bénéficiaire** (IBAN du bénéficiaire ; règle du 25 pour un IBAN neuf : seconde validation et délai, PAY-238), **sans qu'il le demande** (M-729 du 24).
4. **Prime d'équipement** : **toujours reversée** (non déductible [V partiel, 24 §5.7]) ; prime d'internat : déduite de l'internat si la procuration le prévoit [H], sinon reversée.
5. **Public** : même mécanique avec l'agent comptable ; écritures et mandats au 48 (6576 pour les fonds sociaux ; bourses selon la M9.6 [NV]) ; lot BA13.

### 8.5 Changements en cours d'année
| Événement | Traitement |
|---|---|
| **Changement d'établissement** | attribution suspendue à la date de départ ; **attestation de bourse** remise au bénéficiaire pour le nouvel établissement [H : transfert entre établissements géré par les services académiques, NV] ; pas de déduction des trimestres non couverts ; à l'arrivée d'un élève boursier : attribution créée sur pièce, mode selon public / privé et procuration |
| **Redoublement, changement de voie** | nouvelle demande l'année suivante (à refaire chaque année [V]) ; la bourse au mérite suit ses propres conditions [V partiel] |
| **Perte de la charge de l'enfant** | **retrait** à la date ; tâche pour informer le nouveau responsable qu'il **peut** demander [V] ; jamais de bascule automatique vers l'autre parent (R-BA3) |
| **Changement de régime** | lignes cibles recalculées ; si externat : plus de déduction, reliquat intégral reversé |
| **Retenue pour absentéisme** | au-delà de 15 journées injustifiées et répétées depuis le début de l'année : **proposition** au chef avec le décompte (sans motifs) ; décision **motivée**, montant, pièce, notification au bénéficiaire avec voies de recours ; jamais de retenue automatique (R-BA14) ; le décompte « journées » et « demi-journées » est paramétré à partir du socle 12 (Q-BA4) |
| **Décès du bénéficiaire** | attribution suspendue ; tâche à la direction ; aucun courrier automatique au défunt ; reliquat selon la succession (juriste, Q-BA9) |
| **Élève majeur rattaché ou non** | si la notification est au nom de l'élève majeur, il est le bénéficiaire ; sinon le parent demandeur reste bénéficiaire ; l'élève majeur voit la déduction sur **ses** lignes s'il est payeur |
| **Recours** | la famille dépose son recours auprès de l'autorité (ScolIA affiche où et quand) ; si l'établissement est l'autorité (collège public, retenue) : recours enregistré, instruit, décidé, versionné |

### 8.6 Variantes
- **Enseignement agricole** : dispositif `bourse_agricole` (référence globale), mêmes mécanismes, barème propre [V partiel] ; activé seulement si l'établissement relève de l'agriculture.
- **Hors contrat habilité** : même circuit que le privé [V, F984] (ScolIA cible le privé sous contrat, D-029).
- **CNED, lycées militaires** : hors périmètre.

---

## 9. Fonds sociaux

### 9.1 Enveloppes
- Une **enveloppe** par dispositif (collégien, lycéen, cantines) et par période ; **public** : montant notifié, engagements, consommation, reliquat (la répartition « tient compte des fonds encore disponibles » [V] : afficher le reliquat est utile au chef) ; **privé** : pour le fonds cantines, montant **versé par les services académiques** à la suite des propositions ; pour collégien/lycéen, **pas d'enveloppe locale** (l'État paie la famille) : ScolIA suit le nombre et le montant des propositions et des décisions.
- **Caisse de solidarité** (OGEC, établissement), aides APEL : enveloppes locales (§10).

### 9.2 Demande
- **Canaux** : famille depuis son espace (« demander une aide » : objet codé, montant estimé, pièces facultatives), élève (lycéen, majeur ou mineur : un **besoin** qui va à l'assistante sociale, sans pièce), assistante sociale (pour une famille qu'elle accompagne), personnel (CPE, professeur, accueil : **signalement de besoin**, pas une demande au nom de la famille, R-BA21), demande d'aide **depuis un voyage** (§12), depuis le compte financier (FAC-261 du 24).
- **Dossier simple** [V] : pièces **facultatives par défaut** (S-2604) ; l'établissement peut exiger une liste courte par dispositif, jamais plus de 3 pièces (S-2605, D-023) ; aucune pièce de santé (une aide « lunettes » joint un **devis** de l'opticien, jamais une ordonnance : rappel affiché, R-BA20).
- **Objet codé** : restauration, internat, transport, sortie ou voyage, fournitures et manuels, tenue et matériel professionnel ou de sport, besoin élémentaire (soins, lunettes, appareil), maintien en formation, autre (texte court, 200 caractères, lu par l'instructeur seul).

### 9.3 Instruction et avis
- **Instructeur** (adjoint gestionnaire, CPE ou secrétaire de direction désignés, S-2606) : complétude, solutions alternatives (bourse non demandée ? aide de la collectivité ? [V : « vérifier que toutes les solutions alternatives ont été envisagées »]) — ScolIA affiche une **liste de contrôle** de ces alternatives, sans deviner la situation.
- **Avis social** codé de l'assistante sociale (favorable, réservé, défavorable, sans avis ; priorité 1 à 3 ; « situation connue du service social » oui/non) ; **aucune narration** dans le 26 ; son évaluation reste dans son dossier (socle 36, cabinet).
- **Urgence** : le chef accorde **sans commission** (public), en motivant l'urgence ; la commission en est **informée** à la séance suivante (liste pseudonymisée) [V].

### 9.4 Commission (public ; option de bonne pratique au privé)
- **Composition** paramétrée par le chef (membres nommés, qualité) ; chaque membre **signe un engagement de discrétion** (pièce, une fois par année) avant d'accéder à un dossier ; membres externes (parents, élèves délégués) **sans compte personnel requis** : accès par la séance projetée ou par un **lien nominatif** à usage limité (D-027 C) limité aux dossiers de la séance, pendant la séance.
- **Ordre du jour** : dossiers complets ; **pseudonyme** par dossier (`D-2026-014`, jamais l'ordre alphabétique ni la classe) ; **fiche de commission** générée : niveau (pas la classe, S-2607), régime, objet codé, montant demandé, coût en jeu (ex. demi-pension restante), aides déjà obtenues dans l'année (montants), avis social codé, quotient en tranche (si l'établissement en a un) ; **jamais** : nom, prénom, adresse, profession, nationalité, composition détaillée du foyer, motif narratif, santé.
- **Séance** : avis par dossier (favorable, montant proposé, en nature / en argent, défavorable, ajourné avec pièce demandée) ; **vote** facultatif (S-2608, défaut : avis consensuel noté) ; le chef **arrête** ensuite chaque décision (geste distinct, passkey).
- **Compte rendu** **anonymisé** (trame D-023) : date, membres présents (qualités), nombre de dossiers, avis par pseudonyme, total engagé, reliquat ; aucun nom ; déposé au socle 7 (domaine `aides`).
- **Table de correspondance** pseudonyme ↔ élève : visible de l'instructeur, du chef et du comptable (pour l'exécution) seulement.

### 9.5 Décision et exécution
- **Public** : décision du chef (accord, montant, forme : déduction d'une facture, paiement d'un fournisseur, aide en nature — tickets, fournitures achetées —, versement à la famille ou à l'élève majeur) ; fonds cantines **jamais versé à la famille** [V] → seulement déduction ou paiement de l'organisme gestionnaire (avec justificatif de la réduction [V]).
- **Privé — fonds cantines** : proposition au recteur (état récapitulatif pseudonymisé ou nominatif selon ce que demandent les services académiques, Q-BA7) ; à réception de la somme globale : **affectation** par élève aux frais de restauration (aide affectée au 24, lignes restauration uniquement, R-BA13) ; **état de réalisation** par versement ; **reliquat** non employé : reversement aux services académiques (opération du 25) ; signalement des **mécanismes de solidarité interne** (caisse de l'OGEC) dans la proposition [V].
- **Privé — collégien/lycéen** : propositions transmises (document), décision du recteur saisie à réception de l'information ; **aucune imputation** (l'État paie la famille) ; la famille voit « aide accordée par le rectorat : 120 € versés directement par l'État » dans son espace.
- **Notification** au bénéficiaire seul (avis du socle 8, document au coffre) ; **voies de recours** [R : recours gracieux devant le chef, ou hiérarchique devant le recteur, à faire valider par le juriste, Q-BA9].
- **Aide en nature** : bon (fournitures, tenue) remis par le gestionnaire ; trace de la remise (date, objet, valeur), sans facture à la famille.

### 9.6 Bilans
- **Bilan de fin d'année** (public) et **recensement par année civile** (nombre de bénéficiaires, montants, par dispositif) : calculés, agrégés, exportables pour l'enquête Orquestra [V] (format à confirmer, Q-BA6).
- **Privé** : données de bilan prêtes pour les services académiques.
- **Conseil d'administration / OGEC** : rapport agrégé (aides par dispositif et par objet, nombre de bénéficiaires, comparaison N-1) ; aucun nom.

---

## 10. Aides locales, caisse de solidarité, APEL

- **Dispositif local** : créé par l'établissement à partir de modèles (D-023) : caisse de solidarité de l'OGEC, réduction sociale de la contribution [H : souvent sous forme de barème au quotient au 24], aide APEL (voyage, fournitures), aide de la région (restauration, équipement), du département, de la commune, d'un comité d'entreprise, d'une fondation.
- Chaque dispositif déclare : **financeur** (organisme du socle 4/24), **décideur** (direction, commission, financeur), **mode** (déduit par l'établissement, versé par l'établissement, versé par le financeur, en nature), **lignes cibles permises** (catégories du 24), **plafonds** (par élève et par an, en pourcentage du coût), **cumul** avec d'autres aides (jamais au-delà de 100 % du coût de la ligne, R-BA15), **pièces**, **circuit** (commission ou décision directe), **règlement** (pièce, publié aux familles).
- **Aides perçues par l'établissement pour le compte d'un financeur** (région qui verse une aide restauration à l'établissement) : réception, affectation, justification (état par financeur).
- **APEL** : aide décidée par l'APEL ou par la direction avec l'accord de l'APEL [H] ; la **liste nominative** n'est transmise à l'APEL que si une convention le prévoit (Q-V5 du 42 ; défaut : **agrégats par séjour** seulement).
- **Contrôle de double financement** : une ligne ne reçoit pas deux aides au-delà de son montant ; l'excédent est refusé à l'imputation et signalé (« aide APEL non imputable : coût déjà couvert ») (R-BA15).

---

## 11. Quotient familial

- **Calcul** : ressources (revenu fiscal de référence **ou** quotient CAF, selon le règlement de l'établissement, S-2609) / nombre de parts (règle de l'établissement : parts fiscales ou barème propre) → **quotient** → **tranche** (bornes de l'établissement, versionnées par année).
- **Par payeur**, pas par foyer (corrige BD-08) : chaque payeur a son quotient ; un couple marié ou pacsé qui paie ensemble : un quotient commun ; parents séparés : chacun le sien (règle du 24 conservée [F]).
- **Minimisation** (R-BA19) : on conserve **la tranche**, le quotient calculé, l'année des revenus, la source et la date ; le **revenu** et le nombre de parts saisis ne sont conservés que jusqu'à la fin de l'année de facturation + délai de contestation (S-2610, défaut : 3 mois après la fin de l'année), puis **effacés** (la tranche reste) ; la pièce (avis d'imposition ou attestation CAF) suit la même durée (DPO, Q-BA1).
- **Saisie par la famille** : en ligne, avec la pièce ; ou **déclaration sur l'honneur** si le règlement l'admet (S-2611) avec contrôle par sondage ; **sans quotient** : tarif plein ou tranche par défaut déclarée par le règlement (24 §10.4).
- **Simulateur public** (sans enregistrement, 24 T-18) : la famille saisit ses ressources et ses parts et voit sa tranche probable.
- **Changement en cours d'année** : nouvelle tranche à partir du mois suivant (S-158 du 24), avoir proposé ; historique des tranches (jamais écrasé).
- **Qui voit** : la comptabilité voit la **tranche** ; le calcul détaillé n'est visible que de l'agent qui l'instruit (droit `aides.quotient`) ; la famille voit le sien.

---

## 12. Voyages et sorties (avec le socle 42)

- **Demande depuis le séjour** : sur la page du voyage, le payeur voit « une aide est possible » ; il demande (objet `sortie_voyage`, séjour lié par clé composite) ; l'organisateur **n'en sait rien** (R-SV18).
- **Sources** : fonds social collégien/lycéen (public : décision du chef ; privé : proposition au recteur, puis versement direct à la famille → l'établissement ne connaît que la décision), caisse de solidarité, APEL, collectivité, exonération décidée par la direction (FAC-235 du 24).
- **Calendrier** : les aides doivent être connues **avant** l'échéance de solde ; le 26 signale au chef « 3 demandes d'aide liées au séjour de Rome, solde le 15/03 » ; tant qu'une demande est en cours, **aucune relance** de l'échéance concernée (demande au 25, R-BA24) et **aucun retrait** du séjour pour raison financière (R-SV19).
- **Ce que reçoit le 42** : par séjour : nombre d'aides accordées, total par source, reste à charge **agrégé** ; synthèse APEL annuelle (NX-58) : totaux par séjour, liste nominative à la comptabilité seule.
- **Désistement** : l'aide affectée suit l'avoir proposé (proportionnelle) ; elle revient à l'enveloppe (fonds social public, caisse) ou est reversée au financeur selon sa règle.
- **Reprise Next.js** : les aides individuelles nominatives de la fiche budget (VD-37) sont reprises comme **attributions** du 26 (dispositif local selon la source), et la fiche budget du 42 n'affiche plus que des agrégats.

---

## 13. Familles séparées, majeurs, tiers

1. **Bénéficiaire** = la personne qui a demandé et qui a la charge effective (R-BA1). Le 26 lit au socle 4 la **charge** et la **résidence** pour **alerter**, jamais pour choisir à la place des parents.
2. **Résidence alternée** : un seul bénéficiaire ; double demande → alerte à l'instructeur, information **séparée** de chaque demandeur (R-BA6) ; pas d'attestation de l'autre parent.
3. **Déduction** : uniquement sur les lignes du **payeur bénéficiaire** ; si le bénéficiaire n'est pas payeur de la demi-pension (l'autre parent la paie), **aucune déduction**, reliquat intégralement reversé au bénéficiaire, et la famille est informée par une phrase neutre (« votre bourse vous est reversée en totalité ») ; c'est le cas « pas de déduction si le bénéficiaire n'est pas celui qui paie » [V partiel, 24 §5.7].
4. **L'autre parent ne voit rien** : ni la demande, ni la décision, ni la déduction (sa facture ne porte que ses lignes), ni le reversement, ni les avis ; son assistant IA ne peut pas l'en informer (R-BA2) ; une absence de ligne ne doit rien révéler (sa facture est identique qu'il y ait ou non une bourse chez l'autre).
5. **Parent protégé** (adresse protégée, interdiction de contact, socle 4) : les documents d'aide ne mentionnent jamais son adresse sur un document que l'autre peut voir ; un reversement n'emprunte jamais un compte joint sans accord (Q-BA9).
6. **Élève majeur** : bénéficiaire direct possible (bourse s'il n'est pas rattaché au foyer fiscal ; fonds social [V]) ; ses décisions ne sont **pas** envoyées à ses parents ; sa demande de fonds social reste la sienne (les parents ne voient rien sauf s'il les associe).
7. **Élève mineur demandeur** (lycéen) : son besoin va à l'assistante sociale ; la demande formelle et la décision sont **adressées au responsable** [V : « allouée à la famille ou au représentant légal »] ; l'assistante sociale accompagne la famille ; **jamais** d'information aux parents à l'insu de l'élève sans l'évaluation de l'assistante sociale (R-BA22 ; question délicate, Q-BA10).
8. **Tiers** : enfant confié (ASE, tuteur, tiers digne de confiance) : bénéficiaire = la personne ou le service qui a la charge [H] ; majeur protégé : tuteur ou curateur selon la mesure (24 §5.10 point 14).

---

## 14. Ce que voit chacun (projections) [R]

| Qui | Demandes | Décisions, montants | Motifs, objet | Pièces | Avis social | Agrégats |
|---|---|---|---|---|---|---|
| Bénéficiaire (famille, élève majeur) | les siennes | les siennes | les siens | les siennes | — | — |
| Autre parent, autre membre du foyer | **rien** | **rien** | **rien** | **rien** | — | — |
| Élève mineur | son besoin signalé (envoyé) | — | — | — | — | — |
| Secrétariat (dépôt) | registre des dépôts, complétude | statut seulement (« décidée ») | — | présence, pas le contenu | — | — |
| Instructeur | celles de son dispositif | oui | oui | oui | oui | oui |
| Assistante sociale | celles qu'elle accompagne ou qui lui sont soumises, sur ses établissements | décision finale | oui | non par défaut | la sienne | oui |
| Membre externe de commission | dossiers **pseudonymisés** de la séance, pendant la séance | avis de la séance | objet codé | non | codé | séance |
| Chef d'établissement | toutes | oui | oui | sur demande, journalisé | oui | oui |
| Comptabilité | attributions à exécuter (pseudonyme ou nom pour l'exécution) | montants | **non** (libellé « aide n° ») | procuration, RIB | non | oui |
| CPE, professeur, accueil, organisateur | **rien** (signalements envoyés seulement) | rien | rien | rien | rien | séjour : agrégats |
| IA | selon l'utilisateur, minimisé (§16) | — | jamais le texte libre | jamais | jamais | oui |
| Services académiques (export) | ce que la procédure exige | oui | codé | selon demande | non | oui |

Toute consultation nominative hors du bénéficiaire est **journalisée** (journal des accès du socle 27) ; un personnel qui est **parent d'un élève** concerné ou **proche déclaré** ne peut pas l'instruire, le décider ni l'exécuter (R-BA23, reprise de R-F35).

---

## 15. Cas limites (CBA-01 à CBA-60 ; CBA-61 à CBA-74 au §30)

### 15.1 Bourses
| # | Cas | Traitement |
|---|---|---|
| CBA-01 | Résidence alternée, les deux parents déposent | alerte instructeur ; information **séparée** de chaque demandeur ; aucune attestation demandée à l'autre (R-BA6) ; rien n'est transmis tant que l'un n'a pas retiré |
| CBA-02 | Bénéficiaire non payeur de la demi-pension (l'autre parent la paie) | aucune déduction ; reliquat intégral reversé au bénéficiaire ; facture de l'autre parent inchangée (R-BA3) |
| CBA-03 | Deux payeurs de la demi-pension dont le bénéficiaire (50/50) | déduction sur la seule part du bénéficiaire ; reliquat reversé |
| CBA-04 | Privé, bourse accordée, procuration non signée | mode `direct_etat` ; aucune déduction ; la famille voit « l'État vous verse directement » ; l'échéancier reste brut |
| CBA-05 | Procuration révoquée en février | effet au trimestre non encore versé ; échéancier recalculé (avoir ou facture de régularisation **proposés**, R-F14 du 24) |
| CBA-06 | Virement global de l'État différent de la somme attendue | ventilation proposée avec écarts ; aucune déduction constatée tant que la comptabilité n'a pas validé |
| CBA-07 | Versement reçu pour un élève parti | reliquat reversé au bénéficiaire ou restitué aux services académiques selon la période couverte (Q-BA8) ; jamais imputé sur une autre facture |
| CBA-08 | Arrivée en cours d'année d'un boursier d'un autre établissement | attribution sur pièce (notification ou attestation) ; trimestres couverts par l'ancien établissement exclus |
| CBA-09 | Changement de régime en cours de trimestre (externe) | déduction limitée aux lignes restantes ; reliquat reversé |
| CBA-10 | Bourse supérieure à la demi-pension | reliquat reversé à chaque trimestre, sans demande (M-729) |
| CBA-11 | Bourse décidée après l'émission de la facture annuelle | avoir proposé (déduction) ; jamais de réémission (D-027 E) |
| CBA-12 | Bourse retirée (perte de la charge) après déduction anticipée | facture de régularisation **proposée** au bénéficiaire pour les trimestres non versés ; validée par la comptabilité ; jamais émise sans geste |
| CBA-13 | 16 journées injustifiées dont 4 contestées par la famille | proposition de retenue mise **en attente** tant que les justifications contestées ne sont pas tranchées (socle 12) |
| CBA-14 | Absences injustifiées d'un élève en situation de handicap ou de décrochage suivi | la proposition rappelle « vérifier l'accompagnement en cours » (sans dire lequel) ; la décision reste au chef |
| CBA-15 | Élève majeur non rattaché fiscalement | bénéficiaire = l'élève ; aucune information aux parents |
| CBA-16 | Élève devient majeur en cours d'année | la décision court ; bénéficiaire inchangé ; nouvelles demandes au nom de l'élève si non rattaché |
| CBA-17 | Bénéficiaire décédé | suspension ; tâche direction ; aucune relance ; reliquat selon avis du juriste (Q-BA9) |
| CBA-18 | Notification au nom d'une personne absente du dossier administratif | instructeur crée la personne par le socle 4 (jamais par le 26) ; tâche secrétariat |
| CBA-19 | Bourse au mérite retirée en cours d'année | nouvelle version de décision ; échéances suivantes recalculées |
| CBA-20 | Échelon saisi incohérent avec le montant | refus de la saisie sauf motif (« montant de la notification ») ; signalé au contrôle |
| CBA-21 | Établissement agricole et dispositif national confondu | dispositifs distincts ; barème propre |
| CBA-22 | Famille qui veut que la bourse paie la contribution des familles (privé) | impossible par défaut (lignes cibles restauration, internat) ; sur demande écrite du bénéficiaire, **reversement** puis paiement par la famille (deux opérations tracées), jamais une imputation directe (R-BA12) |

### 15.2 Fonds sociaux et commission
| # | Cas | Traitement |
|---|---|---|
| CBA-23 | Urgence (élève sans repas depuis trois jours) | décision du chef sans commission, motif « urgence », information a posteriori (R-BA10) |
| CBA-24 | Membre de commission parent d'un élève concerné | déclaration de conflit ; dossier retiré de son écran ; avis rendu sans lui (R-BA23) |
| CBA-25 | Délégué élève mineur à la commission | accès en séance seulement, pas de lien hors séance ; dossiers pseudonymisés ; engagement de discrétion signé par lui (et information de son responsable [R], Q-BA10) |
| CBA-26 | Dossier identifiable malgré le pseudonyme (seul élève de terminale bac pro en internat) | règle des **petits effectifs** : si un attribut affiché laisse moins de 3 élèves possibles, il est masqué (niveau → cycle, régime masqué) (R-BA8) |
| CBA-27 | Enveloppe épuisée en mars | demandes mises en attente avec information ; orientation vers caisse locale ou collectivité ; jamais de refus « faute de crédits » sans le dire |
| CBA-28 | Reliquat en fin d'année (public) | signalé dès avril au chef ; bilan |
| CBA-29 | Aide en nature (fournitures achetées par l'établissement) | remise tracée, pas de facture ; valeur dans le bilan |
| CBA-30 | Fonds cantines demandé pour un élève externe | refus d'imputation (lignes cibles restauration absentes) ; orientation vers le fonds collégien/lycéen |
| CBA-31 | Privé, recteur accorde moins que proposé | décision saisie telle quelle ; famille informée du montant accordé |
| CBA-32 | Privé, somme globale du fonds cantines non entièrement employée | état de réalisation et reversement du reliquat proposés ; aucune affectation à un autre usage (R-BA13) |
| CBA-33 | Demande d'aide pour lunettes avec ordonnance jointe | ScolIA refuse le dépôt d'une pièce typée « santé » ; propose un devis à la place ; pièce déjà déposée → retirée par l'instructeur, journalisé (R-BA20) |
| CBA-34 | Famille déjà aidée trois fois dans l'année | la fiche montre le cumul (montants), pas les motifs antérieurs |
| CBA-35 | Assistante sociale absente pour longue durée | avis « sans avis » possible ; le chef n'est pas bloqué |
| CBA-36 | Demande déposée par un professeur au nom d'une famille | refusée comme demande ; transformée en **signalement de besoin** vers l'assistante sociale (R-BA21) |
| CBA-37 | Famille qui refuse que l'assistante sociale soit sollicitée | son choix est noté ; l'instruction continue sans avis social |
| CBA-38 | Élève lycéen mineur qui demande sans vouloir que ses parents le sachent | besoin reçu par l'assistante sociale ; aucune information automatique aux parents ; suite selon l'évaluation professionnelle (R-BA22) |

### 15.3 Aides locales, quotient, voyages, données
| # | Cas | Traitement |
|---|---|---|
| CBA-39 | Aides cumulées dépassant le coût de la ligne | imputation plafonnée à 100 % ; excédent refusé et signalé (R-BA15) |
| CBA-40 | Aide de la région versée à l'établissement pour 12 élèves, dont un parti | affectation des 11 ; part du parti remboursée à la région ou reversée selon la convention |
| CBA-41 | Quotient fourni en avril | tranche à partir de mai (S-158) ; avoir proposé si le règlement le prévoit |
| CBA-42 | Deux parents mariés payeurs : un seul quotient ou deux ? | un quotient du couple payeur ; séparés : chacun le sien |
| CBA-43 | Quotient déclaré sur l'honneur, contrôle révèle une erreur | régularisation proposée ; motif ; jamais de sanction automatique |
| CBA-44 | Voyage : aide accordée puis désistement | aide réduite au prorata de l'avoir ; retour à l'enveloppe ou au financeur |
| CBA-45 | Voyage : 2 élèves aidés sur 48, l'organisateur voit « 2 aides » | petits nombres : l'agrégat est affiché « moins de 3 » (R-BA8) |
| CBA-46 | Voyage obligatoire facturé (public) | aucune participation due [V] ; le 26 n'a rien à aider ; anomalie signalée au 42 |
| CBA-47 | APEL demande la liste nominative | refus par défaut ; convention requise (Q-V5) |
| CBA-48 | Personnel parent d'un élève boursier | son propre dossier est **protégé** (accès nominatif motivé, journalisé) ; il ne l'instruit pas (R-BA23) |
| CBA-49 | Demande de droit d'accès (RGPD) d'un parent sur « tout ce que l'établissement sait de mon enfant » | l'extrait n'inclut **pas** les demandes et décisions dont l'autre parent est bénéficiaire (données de l'autre personne) ; inclut les siennes (Q-BA1, DPO) |
| CBA-50 | Export de portabilité de l'établissement | tables du 26 en catégorie « double » (PORTABILITE §4) ; pièces chiffrées |
| CBA-51 | Restauration de sauvegarde | file d'avis neutralisée (E9) ; reversements jamais rejoués (clé d'idempotence côté 25) |
| CBA-52 | Fusion de deux fiches élèves | demandes et attributions rattachées à l'élève conservé ; doublons de demandes signalés |
| CBA-53 | Fermeture de l'année | attributions closes ; reliquats calculés ; aucune attribution reconduite automatiquement (à redemander chaque année [V]) |
| CBA-54 | Élève sans responsable joignable (famille sans compte, sans courriel) | courrier postal généré (socle 7) ; aucune perte de droit pour un canal numérique absent |
| CBA-55 | Famille allophone | information et formulaire dans sa langue (socle 8) ; documents officiels en français avec traduction d'aide |
| CBA-56 | Demande en ligne depuis un téléphone partagé | pas d'aperçu de contenu dans les notifications push (R-BA16) |
| CBA-57 | Secrétaire qui est aussi voisine de la famille | elle voit le dépôt (nécessaire) mais pas les pièces ni les montants ; le journal trace ses consultations |
| CBA-58 | Comptable qui remarque une aide sur la facture d'un collègue | la ligne porte « aide n° A-… », sans dispositif ni motif |
| CBA-59 | Établissement public et privé dans le même groupe | dispositifs et circuits par établissement ; aucune visibilité croisée (R-F26 du 24) |
| CBA-60 | Contrôle des services académiques sur l'emploi du fonds cantines | état de réalisation par versement, justificatifs, reliquat, journal |

---

## 16. Intelligence artificielle (contrat IA du socle, `IA_COEUR.md`)

### 16.1 Ce que l'IA fait [R]
- **Prépare** : la note d'information annuelle aux familles (texte relu, traductions) ; le compte rendu anonymisé à partir des avis saisis ; l'état de réalisation et le bilan (chiffres calculés par le code, texte de présentation proposé) ; la liste des pièces manquantes et les relances (brouillons, envoi après confirmation).
- **Contrôle** (déterministe d'abord, IA pour l'explication) : écarts entre versement reçu et attendu, attributions sans procuration, aides dépassant le coût, enveloppe en voie d'épuisement ou reliquat inemployé, demandes en attente avant une échéance de voyage, double demande d'un même élève.
- **Explique** à la famille bénéficiaire, avec des phrases fixes : « votre bourse de 244 € du 1er trimestre a réglé la demi-pension d'octobre à décembre (198 €) ; les 46 € restants vous ont été virés le 08/01 ».
- **Oriente** : le guide des aides (quelle aide pour quel besoin, quelles dates), **sans** demander ni déduire la situation de la personne.

### 16.2 Ce que l'IA ne fait pas [R]
- **Jamais** décider, proposer un montant pour un dossier, classer des dossiers par « besoin », recommander un refus, ni une retenue ; **jamais** prédire qu'une famille « a probablement droit » à partir de ses données (profilage social, R-BA17) ; la famille peut **elle-même** utiliser le simulateur.
- **Jamais** recevoir : le texte libre d'une demande, une pièce, un avis social, une donnée de santé (objet « besoin élémentaire » réduit à son code), un revenu, l'identité d'un bénéficiaire pour une tâche de lot (pseudonymes, `IA_COEUR.md` §7).
- **Jamais** répondre à un utilisateur sur l'aide d'une autre personne (« mon ex touche-t-elle la bourse ? » → « je ne peux pas répondre à cette question »), ni laisser deviner par une différence de réponse (R-BA2).
- **Jamais** envoyer : tout envoi est confirmé par une personne (N3 pour les décisions, N2 pour les relances).

### 16.3 Capacités déclarées (registre `ent-ia`)
`aides.guide` (lecture publique), `aides.expliquer_ma_situation` (bénéficiaire), `aides.brouillon_information`, `aides.brouillon_compte_rendu` (chef, à partir d'avis pseudonymisés), `aides.controles` (comptabilité, chef), `aides.brouillon_relance_pieces` (instructeur), `aides.brouillon_bilan` (chef). Chacune : droits de l'utilisateur, contexte minimisé, journal.

### 16.4 Cadre
Fonctionnement complet sans IA ; fournisseur et garanties : `IA_COEUR.md` (Mistral prioritaire sous réserve contractuelle) ; étude du règlement européen sur l'IA avant les fonctions à modèle de langage (D-032) : l'accès à une aide sociale peut relever des usages à haut risque si un système **évalue** l'éligibilité [H] → raison de plus pour que l'IA n'évalue rien (Q-BA3).

---

## 17. Actions métier (BRS-100 à BRS-271)

Légende : DIR chef d'établissement (ou délégataire nommé) ; INS instructeur désigné ; SEC secrétariat ; CPT comptabilité (agent comptable, régisseur, comptable OGEC) ; ASS assistante sociale ; COM membre de commission ; PER tout personnel ; FAM bénéficiaire ou responsable demandeur ; ELV élève ; SYS automatisme ; ★ passkey récente (N3).

### 17.1 Paramétrage et référentiel (BRS-100 à 114)
| # | Action | Qui · droit | Garde | Événement | Lot |
|---|---|---|---|---|---|
| BRS-100 | Charger le référentiel national des dispositifs et barèmes de l'année (mise à jour ScolIA) | plateforme | source et date de consultation obligatoires | `aides.referentiel_publie` | BA1 |
| BRS-101 | Voir les barèmes nationaux en vigueur | INS, CPT, DIR · `aides.lire` | lecture | — | BA1 |
| BRS-102 | Activer les dispositifs nationaux qui concernent l'établissement (collège, lycée, agricole) | DIR · `aides.parametrer` | selon le type d'établissement (5) | `aides.dispositif_active` | BA1 |
| BRS-103 | Créer un dispositif local à partir d'un modèle (caisse, APEL, région…) | DIR · `aides.parametrer` | financeur, décideur, mode, lignes cibles obligatoires | `aides.dispositif_cree` | BA7 |
| BRS-104 | Définir les lignes cibles permises d'un dispositif | DIR · `aides.parametrer` | fonds cantines : restauration seule (R-BA13) | — | BA7 |
| BRS-105 | Fixer plafonds et règles de cumul | DIR · `aides.parametrer` | jamais > 100 % du coût | — | BA7 |
| BRS-106 | Déposer le règlement d'un dispositif local (pièce, publication aux familles) | DIR | version datée | `aides.reglement_publie` | BA7 |
| BRS-107 | Désigner les instructeurs par dispositif | DIR · `aides.parametrer` | personne nommée (D-017) ; pas un rôle entier | `aides.instructeur_designe` | BA1 |
| BRS-108 | Rattacher une assistante sociale partagée à l'établissement (périmètre) | DIR | lecture limitée à cet établissement | — | BA5 |
| BRS-109 | Fixer la liste courte des pièces par dispositif local | DIR | ≤ 3, facultatives par défaut (S-2604, S-2605) | — | BA1 |
| BRS-110 | Paramétrer la composition de la commission | DIR | qualités ; membres externes possibles | — | BA5 |
| BRS-111 | Enregistrer l'avis du CA sur les critères du fonds cantines (public) | DIR | pièce de la délibération | `aides.criteres_votes` | BA5 |
| BRS-112 | Définir les tranches du quotient (versionnées) | DIR, CPT · `aides.parametrer` | jamais modifiées après vote ; nouvelle version | `aides.tranches_publiees` | BA8 |
| BRS-113 | Choisir le décompte de l'absentéisme pour la retenue (journées / demi-journées) | DIR | valeur défaut 15 journées [V] | — | BA4 |
| BRS-114 | Importer les barèmes et décisions depuis un fichier national (si disponible) | CPT · `aides.importer` | aperçu sans écriture ; socle 9 | `aides.import` | BA11 |

### 17.2 Information et campagnes (BRS-115 à 126)
| # | Action | Qui · droit | Garde | Événement | Lot |
|---|---|---|---|---|---|
| BRS-115 | Ouvrir les campagnes de l'année (dates nationales pré-remplies) | SYS, DIR | date interne ≤ date nationale | `aides.campagne_ouverte` | BA1 |
| BRS-116 | Envoyer l'information annuelle à **toutes** les familles | DIR · `aides.informer` | audience = tous les responsables (R-BA17) ; langues | `aides.information_envoyee` | BA1 |
| BRS-117 | Relancer **toutes** les familles avant la date limite | DIR | jamais une sélection par profil | — | BA1 |
| BRS-118 | Voir au professeur principal « information envoyée à N familles » | PER (PP) | aucun nom | — | BA1 |
| BRS-119 | Publier le guide des aides (page publique, sans donnée personnelle) | DIR | socle 17 | — | BA10 |
| BRS-120 | Afficher le lien vers les simulateurs officiels | SYS | — | — | BA10 |
| BRS-121 | Recueillir l'accord d'étude automatique dans l'inscription (socle 3) | FAM | case non cochée par défaut ; état civil du concubin facultatif | `aides.accord_etude_automatique` | BA1 |
| BRS-122 | Retirer l'accord d'étude automatique | FAM | avant transmission ; ensuite : information sur la démarche | — | BA1 |
| BRS-123 | Produire la liste des accords à transmettre à l'outil national | SEC · `aides.transmettre` | export journalisé, minimal (identité, accord) | `aides.export_accords` | BA1 |
| BRS-124 | Clore une campagne | SYS | dépôts tardifs : enregistrés « hors délai » | `aides.campagne_close` | BA1 |
| BRS-125 | Voir le tableau de campagne (dépôts, complets, transmis, décidés) | INS, DIR · `aides.lire` | agrégé + liste pour l'instructeur | — | BA1 |
| BRS-126 | Rappeler la campagne de printemps aux familles de 3e (lycée l'an prochain) | DIR | toutes les familles du niveau | — | BA1 |

### 17.3 Demandes et pièces (BRS-127 à 146)
| # | Action | Qui · droit | Garde | Événement | Lot |
|---|---|---|---|---|---|
| BRS-127 | Déposer une demande en ligne (dispositif local ou fonds social) | FAM, ELV majeur · `aides.demander` | demandeur = soi ; élève lié | `aides.demande_deposee` | BA1 |
| BRS-128 | Enregistrer un dépôt papier au guichet | SEC · `aides.deposer` | aucune saisie de revenus | `aides.demande_deposee` | BA1 |
| BRS-129 | Imprimer l'accusé de réception numéroté | SEC | document au coffre du demandeur | `documents.emis` | BA1 |
| BRS-130 | Joindre une pièce (scan, photo depuis l'appli) | FAM, SEC, INS | types autorisés ; jamais « santé » (R-BA20) | `aides.piece_ajoutee` | BA1 |
| BRS-131 | Voir la présence des pièces (sans contenu) | SEC | — | — | BA1 |
| BRS-132 | Lire une pièce | INS · `aides.pieces_lire` | journalisé | — | BA1 |
| BRS-133 | Marquer une demande complète / incomplète | INS | motif codé | `aides.demande_etat` | BA1 |
| BRS-134 | Relancer les pièces manquantes au demandeur seul | INS | brouillon IA possible ; confirmation | `aides.relance_pieces` | BA1 |
| BRS-135 | Détecter une double demande (même élève, même dispositif) | SYS | alerte instructeur seulement | `aides.double_demande` | BA1 |
| BRS-136 | Informer séparément chaque demandeur de la règle de résidence alternée | INS | deux messages distincts, sans citer l'autre | — | BA1 |
| BRS-137 | Retirer sa demande | FAM | avant décision | `aides.demande_retiree` | BA1 |
| BRS-138 | Enregistrer une demande déposée hors délai | SEC | marque « hors délai » ; transmission selon autorité | — | BA1 |
| BRS-139 | Transmettre une demande à l'autorité (DSDEN, rectorat) | INS · `aides.transmettre` | bordereau (document) ; date | `aides.demande_transmise` | BA1 |
| BRS-140 | Voir « où en est ma demande » | FAM | ses demandes | — | BA10 |
| BRS-141 | Compléter une demande (pièce demandée) | FAM | — | `aides.piece_ajoutee` | BA10 |
| BRS-142 | Corriger le demandeur d'une demande (erreur de saisie au guichet) | INS | motif ; ancien demandeur jamais informé du contenu | `aides.demande_corrigee` | BA1 |
| BRS-143 | Rechercher une demande par numéro ou élève | INS, DIR | journalisé | — | BA1 |
| BRS-144 | Voir l'historique d'une demande (états datés, auteurs) | INS, DIR | ajout seul | — | BA1 |
| BRS-145 | Rattacher une demande à un séjour (socle 42) | FAM, INS | clé composite | — | BA9 |
| BRS-146 | Rattacher une demande à une facture ou une échéance (24) | FAM, INS | aucune relance de l'échéance pendant l'instruction (R-BA24) | `aides.echeance_suspendue` | BA3 |

### 17.4 Décisions, attributions, procurations (BRS-147 à 166)
| # | Action | Qui · droit | Garde | Événement | Lot |
|---|---|---|---|---|---|
| BRS-147 | Saisir une décision reçue (accord, échelon, montant, primes, référence, pièce) | INS · `aides.decisions_saisir` | montant pré-rempli du barème ; écart → motif | `aides.decision_enregistree` | BA2 |
| BRS-148 | Saisir un refus (motif codé, voies de recours) | INS | — | `aides.decision_enregistree` | BA2 |
| BRS-149 | Corriger une décision (nouvelle version) | INS ★ | ancienne version conservée | `aides.decision_corrigee` | BA2 |
| BRS-150 | Créer l'attribution (bénéficiaire, période, mode) | SYS, INS | bénéficiaire = personne du socle 4 (R-BA1) | `aides.accordee` | BA2 |
| BRS-151 | Changer le bénéficiaire d'une attribution | INS ★ | pièce (notification) ; motif ; jamais par changement de payeur (R-BA3) | `aides.beneficiaire_change` | BA2 |
| BRS-152 | Envoyer la notification au bénéficiaire seul | INS, SYS | avis sans contenu sur l'écran verrouillé (R-BA16) ; document au coffre | `aides.notifiee` | BA2 |
| BRS-153 | Proposer la procuration à signer (privé) | INS, SYS | trame D-023 ; au bénéficiaire seul | `aides.procuration_proposee` | BA2 |
| BRS-154 | Signer la procuration en ligne | FAM ★ (passkey) | socle 7 | `aides.procuration_signee` | BA2 |
| BRS-155 | Enregistrer une procuration papier | SEC, INS | pièce scannée ; date | `aides.procuration_signee` | BA2 |
| BRS-156 | Révoquer sa procuration | FAM | effet au trimestre non versé | `aides.procuration_revoquee` | BA2 |
| BRS-157 | Voir les attributions sans procuration (privé) | CPT, INS | — | — | BA2 |
| BRS-158 | Voir les attributions actives (liste d'exécution) | CPT · `aides.executer` | libellé neutre | — | BA2 |
| BRS-159 | Voir le détail d'une attribution (décision, échéances, déductions, reliquats) | INS, CPT, DIR, FAM bénéficiaire | — | — | BA3 |
| BRS-160 | Ajouter une prime à une attribution (internat, équipement, mérite) | INS | ligne typée ; prime d'équipement non déductible | `aides.prime_ajoutee` | BA2 |
| BRS-161 | Suspendre une attribution (départ, décès, enquête) | INS ★ | motif ; date | `aides.attribution_suspendue` | BA4 |
| BRS-162 | Retirer une attribution (perte de charge, décision de l'autorité) | INS ★ | pièce ; régularisation proposée | `aides.attribution_retiree` | BA4 |
| BRS-163 | Délivrer une attestation de bourse (changement d'établissement) | INS, SEC (sur demande du bénéficiaire) | au bénéficiaire seul | `documents.emis` | BA4 |
| BRS-164 | Créer une attribution d'arrivée sur pièce | INS | trimestres déjà couverts exclus | `aides.accordee` | BA4 |
| BRS-165 | Clore les attributions de l'année | SYS | aucune reconduction (R-BA5) | `aides.annee_close` | BA4 |
| BRS-166 | Enregistrer une attribution `direct_etat` pour information | INS | aucune déduction ; famille informée | `aides.accordee` | BA2 |

### 17.5 Versements, déductions, reliquats (BRS-167 à 184)
| # | Action | Qui · droit | Garde | Événement | Lot |
|---|---|---|---|---|---|
| BRS-167 | Calculer les échéances prévues par trimestre | SYS | montant / 3, reste au 3e | — | BA3 |
| BRS-168 | Demander la déduction prévue au 24 (échéancier net) | SYS | lignes cibles du payeur bénéficiaire seulement (R-BA3) | `aides.deduction_prevue` | BA3 |
| BRS-169 | Rattacher un versement reçu (25) à un dispositif et un trimestre | CPT · `aides.executer` | — | `aides.reception_enregistree` | BA3 |
| BRS-170 | Voir la ventilation proposée d'un versement global | CPT | écarts listés | — | BA3 |
| BRS-171 | Valider la ventilation | CPT (N2) | somme = versement ; écarts motivés | `aides.reception_ventilee` | BA3 |
| BRS-172 | Constater la déduction (24) | SYS | idempotent par `aide:{attribution}:{trimestre}` | `facturation.aide_imputee` | BA3 |
| BRS-173 | Calculer le reliquat par attribution | SYS | reçu − déduit | — | BA3 |
| BRS-174 | Demander le reversement du reliquat au 25 | SYS → CPT | vers le bénéficiaire ; IBAN neuf : règle PAY-238 | `aides.reversement_demande` | BA3 |
| BRS-175 | Reverser la prime d'équipement en totalité | SYS → CPT | jamais déduite | `aides.reversement_demande` | BA3 |
| BRS-176 | Signaler un versement attendu non reçu | SYS | à J+30 de la fin de trimestre | `aides.versement_manquant` | BA3 |
| BRS-177 | Régulariser après retrait (facture proposée) | SYS → CPT | proposition ; émission N3 au 24 | `facturation.regularisation_proposee` | BA4 |
| BRS-178 | Restituer un trop-perçu de l'État (élève parti) | CPT ★ | opération du 25 | `aides.restitution` | BA3 |
| BRS-179 | Recalculer les déductions après changement de régime | SYS | proposition | — | BA3 |
| BRS-180 | Voir l'état des bourses par trimestre (attendu, reçu, déduit, reversé) | CPT, DIR | agrégé ; détail pour CPT | — | BA3 |
| BRS-181 | Rapprocher les reversements avec le relevé bancaire | CPT | 25 | — | BA3 |
| BRS-182 | Exporter les écritures d'aides vers la comptabilité | CPT | 48 ; libellés neutres | — | BA13 |
| BRS-183 | Expliquer une ligne d'aide à la famille bénéficiaire | FAM, IA | phrases fixes | — | BA10 |
| BRS-184 | Bloquer la déduction si la procuration expire | SYS | — | `aides.deduction_bloquee` | BA3 |

### 17.6 Changements, retenues, recours (BRS-185 à 200)
| # | Action | Qui · droit | Garde | Événement | Lot |
|---|---|---|---|---|---|
| BRS-185 | Proposer une retenue au chef quand le seuil d'absentéisme est franchi | SYS | décompte seul, aucun motif d'absence ; contestations en cours exclues | `aides.retenue_proposee` | BA4 |
| BRS-186 | Décider une retenue motivée | DIR ★ · `aides.decider` | motif obligatoire ; montant ≤ bourse annuelle | `aides.retenue_decidee` | BA4 |
| BRS-187 | Écarter une proposition de retenue (motif) | DIR | — | `aides.retenue_ecartee` | BA4 |
| BRS-188 | Notifier la retenue au bénéficiaire avec voies de recours | SYS | document ; avis neutre | `aides.notifiee` | BA4 |
| BRS-189 | Annuler une retenue (absences justifiées après coup) | DIR ★ | régularisation | `aides.retenue_annulee` | BA4 |
| BRS-190 | Enregistrer un recours (famille) | FAM, INS | délai calculé et affiché | `aides.recours_depose` | BA4 |
| BRS-191 | Décider un recours (établissement autorité) | DIR ★ | nouvelle version de décision | `aides.recours_decide` | BA4 |
| BRS-192 | Enregistrer l'issue d'un recours auprès de l'autorité | INS | — | `aides.decision_corrigee` | BA4 |
| BRS-193 | Réagir à un départ (suspension, attestation) | SYS | — | — | BA4 |
| BRS-194 | Réagir à une perte de charge signalée (tâche) | SYS → INS | jamais de bascule automatique | — | BA4 |
| BRS-195 | Informer le nouveau responsable qu'il peut demander | INS | message générique | — | BA4 |
| BRS-196 | Réagir au décès d'un bénéficiaire | SYS → DIR | aucun envoi automatique | — | BA4 |
| BRS-197 | Réagir à la majorité de l'élève | SYS | — | — | BA4 |
| BRS-198 | Signaler un changement de situation (famille) | FAM | orientation fonds social [V] | `aides.situation_signalee` | BA10 |
| BRS-199 | Reprendre les décisions d'une fusion d'élèves | SYS | socle 2 | — | BA14 |
| BRS-200 | Voir le délai de recours restant | FAM, INS | — | — | BA4 |

### 17.7 Fonds sociaux : enveloppes, demandes, instruction (BRS-201 à 216)
| # | Action | Qui · droit | Garde | Événement | Lot |
|---|---|---|---|---|---|
| BRS-201 | Enregistrer l'enveloppe notifiée (montant, pièce) | CPT, DIR | — | `aides.enveloppe_notifiee` | BA5 |
| BRS-202 | Voir l'enveloppe (notifiée, engagée, consommée, reliquat) | DIR, CPT, INS | — | — | BA5 |
| BRS-203 | Alerter sur un reliquat important en avril | SYS | — | — | BA5 |
| BRS-204 | Alerter sur une enveloppe épuisée | SYS | — | — | BA5 |
| BRS-205 | Demander une aide (famille) avec objet codé | FAM · `aides.demander` | texte libre ≤ 200 car. | `aides.demande_deposee` | BA5 |
| BRS-206 | Exprimer un besoin (élève) | ELV | va à l'assistante sociale | `aides.besoin_signale` | BA5 |
| BRS-207 | Signaler un besoin (personnel) | PER · `aides.signaler` | aucun retour sur la suite (R-BA21) | `aides.besoin_signale` | BA15 |
| BRS-208 | Voir et prendre en charge les besoins signalés | ASS, DIR | — | — | BA5 |
| BRS-209 | Déposer une demande pour une famille accompagnée | ASS | accord de la famille noté | `aides.demande_deposee` | BA5 |
| BRS-210 | Donner un avis social codé | ASS · `aides.avis_social` | aucune narration | `aides.avis_social_donne` | BA5 |
| BRS-211 | Cocher la liste des solutions alternatives | INS | — | — | BA5 |
| BRS-212 | Voir le cumul des aides de l'année pour l'élève | INS, DIR | montants, pas les motifs anciens | — | BA5 |
| BRS-213 | Inscrire un dossier à une commission | INS | complet | — | BA5 |
| BRS-214 | Accorder en urgence sans commission | DIR ★ | motif « urgence » | `aides.accordee` | BA5 |
| BRS-215 | Informer la commission des décisions d'urgence | SYS | liste pseudonymisée | — | BA5 |
| BRS-216 | Orienter vers un autre dispositif (bourse, collectivité) | INS | message à la famille | — | BA5 |

### 17.8 Commission et décision (BRS-217 à 232)
| # | Action | Qui · droit | Garde | Événement | Lot |
|---|---|---|---|---|---|
| BRS-217 | Convoquer une commission (date, membres, ordre du jour pseudonymisé) | DIR | socle 19 pour la date | `aides.commission_convoquee` | BA5 |
| BRS-218 | Signer l'engagement de discrétion | COM ★ ou lien nominatif | une fois par année | `aides.discretion_signee` | BA5 |
| BRS-219 | Déclarer un conflit d'intérêts sur un dossier | COM | dossier masqué pour lui | `aides.conflit_declare` | BA5 |
| BRS-220 | Ouvrir la séance (projection, accès des externes) | DIR | accès des liens limité à la durée | `aides.seance_ouverte` | BA5 |
| BRS-221 | Consulter les fiches pseudonymisées | COM | petits effectifs masqués (R-BA8) | — | BA5 |
| BRS-222 | Saisir l'avis de la commission par dossier | DIR, secrétaire de séance | — | `aides.avis_commission` | BA5 |
| BRS-223 | Clore la séance (accès externes coupés) | DIR | — | `aides.seance_close` | BA5 |
| BRS-224 | Générer le compte rendu anonymisé | SYS, IA (brouillon) | aucun nom | `documents.emis` | BA5 |
| BRS-225 | Arrêter les décisions (public) en lot, une par une confirmée | DIR ★ | geste distinct de l'avis | `aides.accordee` / `aides.refusee` | BA5 |
| BRS-226 | Choisir la forme de l'aide (déduction, fournisseur, nature, versement) | DIR, CPT | fonds cantines jamais versé à la famille | — | BA5 |
| BRS-227 | Enregistrer une aide en nature et sa remise | CPT | valeur ; date ; remis à | `aides.remise_nature` | BA5 |
| BRS-228 | Payer un fournisseur pour le compte d'une famille | CPT ★ | 25 ; facture du fournisseur | `aides.paiement_tiers` | BA5 |
| BRS-229 | Verser l'aide à la famille ou à l'élève majeur (public, fonds collégien/lycéen) | CPT ★ | 25 | `aides.versement` | BA5 |
| BRS-230 | Notifier la décision au responsable (ou à l'élève majeur) | SYS | avis neutre | `aides.notifiee` | BA5 |
| BRS-231 | Voir la correspondance pseudonyme ↔ élève | INS, DIR, CPT | journalisé | — | BA5 |
| BRS-232 | Rouvrir un dossier ajourné | INS | — | — | BA5 |

### 17.9 Privé : propositions, fonds cantines, état de réalisation (BRS-233 à 240)
| # | Action | Qui · droit | Garde | Événement | Lot |
|---|---|---|---|---|---|
| BRS-233 | Préparer les propositions au recteur (collégien, lycéen, cantines) | INS, DIR | signalement des solidarités internes [V] | `aides.propositions_preparees` | BA6 |
| BRS-234 | Signer et transmettre les propositions | DIR ★ | bordereau ; pièces | `aides.propositions_transmises` | BA6 |
| BRS-235 | Saisir la décision du recteur | INS | montant accordé ≤ proposé ou non | `aides.decision_enregistree` | BA6 |
| BRS-236 | Enregistrer le versement global du fonds cantines | CPT | 25 | `aides.reception_enregistree` | BA6 |
| BRS-237 | Affecter la somme aux élèves (restauration seulement) | CPT | R-BA13 | `facturation.aide_imputee` | BA6 |
| BRS-238 | Produire l'état de réalisation d'un versement | CPT, DIR | document | `documents.emis` | BA6 |
| BRS-239 | Reverser le reliquat non employé | CPT ★ | 25 | `aides.restitution` | BA6 |
| BRS-240 | Informer la famille d'une aide versée directement par l'État | SYS | avis neutre | `aides.notifiee` | BA6 |

### 17.10 Aides locales, quotient, voyages (BRS-241 à 256)
| # | Action | Qui · droit | Garde | Événement | Lot |
|---|---|---|---|---|---|
| BRS-241 | Décider une aide de la caisse de solidarité | DIR ★ | plafonds ; pièces | `aides.accordee` | BA7 |
| BRS-242 | Enregistrer une aide décidée par un financeur externe (région, APEL) | INS | pièce | `aides.accordee` | BA7 |
| BRS-243 | Recevoir et affecter une aide versée par un financeur | CPT | — | `facturation.aide_imputee` | BA7 |
| BRS-244 | Produire l'état par financeur | CPT | agrégé ou nominatif selon convention | `documents.emis` | BA7 |
| BRS-245 | Refuser une imputation au-delà du coût (double financement) | SYS | R-BA15 | `aides.imputation_refusee` | BA7 |
| BRS-246 | Saisir son quotient en ligne avec pièce | FAM | — | `aides.quotient_depose` | BA8 |
| BRS-247 | Calculer et valider le quotient (tranche) | INS · `aides.quotient` | — | `aides.quotient_valide` | BA8 |
| BRS-248 | Transmettre la tranche au 24 | SYS | tranche seule | `aides.tranche_changee` | BA8 |
| BRS-249 | Effacer les ressources détaillées à échéance | SYS | tranche conservée (R-BA19) | `aides.ressources_effacees` | BA14 |
| BRS-250 | Contrôler par sondage les déclarations sur l'honneur | INS | — | — | BA8 |
| BRS-251 | Demander une aide depuis la page d'un séjour | FAM | organisateur non informé | `aides.demande_deposee` | BA9 |
| BRS-252 | Signaler au chef les demandes liées à un séjour avant l'échéance | SYS | — | — | BA9 |
| BRS-253 | Transmettre les agrégats d'aides au séjour (42) | SYS | petits nombres masqués | `aides.agregat_sejour` | BA9 |
| BRS-254 | Ajuster l'aide après un désistement | SYS → CPT | prorata | — | BA9 |
| BRS-255 | Produire la synthèse APEL annuelle (totaux) | CPT | NX-58 ; nominatif seulement avec convention | `documents.emis` | BA9 |
| BRS-256 | Reprendre les aides individuelles des séjours Next.js | ADM (pilote) | §24 | `aides.reprise` | BA14 |

### 17.11 Famille et élève (BRS-257 à 264)
| # | Action | Qui · droit | Garde | Événement | Lot |
|---|---|---|---|---|---|
| BRS-257 | Voir « mes aides » (demandes, décisions, versements, déductions, reliquats) | FAM, ELV majeur | les siennes | — | BA10 |
| BRS-258 | Télécharger notifications, accusés, attestations | FAM | coffre | — | BA10 |
| BRS-259 | Demander une aide depuis l'appli mobile | FAM | D-003 | — | BA10 |
| BRS-260 | Voir l'échéancier net de la bourse | FAM | 24 FAC-259 | — | BA10 |
| BRS-261 | Mettre à jour l'IBAN de reversement | FAM ★ | 25 | — | BA10 |
| BRS-262 | Choisir la langue des documents d'aide | FAM | socle 8 | — | BA10 |
| BRS-263 | Demander un rendez-vous avec l'assistante sociale | FAM, ELV | socle 19 ; motif non affiché aux autres | — | BA10 |
| BRS-264 | Utiliser le simulateur public (sans enregistrement) | public | aucune donnée conservée | — | BA10 |

### 17.12 Pilotage, bilans, IA, système (BRS-265 à 271)
| # | Action | Qui · droit | Garde | Événement | Lot |
|---|---|---|---|---|---|
| BRS-265 | Tableau de bord des aides (agrégé) | DIR · `aides.pilotage` | aucun nom | — | BA11 |
| BRS-266 | Bilan de fin d'année des fonds sociaux | DIR | document | `documents.emis` | BA11 |
| BRS-267 | Recensement par année civile (export Orquestra) | DIR · `aides.exporter` | réservé administratifs (D-022) | `aides.export` | BA11 |
| BRS-268 | Rapport au CA / à l'OGEC | DIR | agrégé, N-1 | `documents.emis` | BA11 |
| BRS-269 | Contrôles automatiques et leur explication | SYS, IA | §16.1 | — | BA12 |
| BRS-270 | Journal des consultations des dossiers d'aide | DIR, DPO | socle 27 | — | BA1 |
| BRS-271 | Purger les dossiers à l'échéance de conservation | SYS | durées du DPO | `aides.purge` | BA14 |

---

## 18. Règles transverses (R-BA1 à R-BA40 ; R-BA41 à R-BA46 au §30)

| # | Règle | Lot |
|---|---|---|
| R-BA1 | **Le bénéficiaire d'une aide est une personne** (socle 4), jamais l'élève ni le foyer ; chaque attribution en a exactement un | BA1 |
| R-BA2 | **Cercle de confiance** : demande, décision, montant, motif ne sont visibles que du bénéficiaire et des personnes habilitées à agir (§14) ; aucune réponse, aucun libellé, aucune absence de ligne ne doit révéler l'aide à un tiers | BA0 |
| R-BA3 | Une déduction ne s'impute que sur les lignes cibles **du payeur bénéficiaire** ; un changement de payeur ou de résidence ne transfère jamais une aide à une autre personne | BA0 (provisoire), BA3 |
| R-BA4 | Une décision ne se modifie pas : toute correction est une **nouvelle version** datée et motivée | BA2 |
| R-BA5 | Aucune attribution n'est **reconduite** d'une année sur l'autre | BA4 |
| R-BA6 | Jamais d'attestation de non-demande exigée de l'autre parent ; double demande → information séparée de chaque demandeur | BA1 |
| R-BA7 | Toute décision porte son **décideur**, sa date, sa référence, ses voies et délais de recours | BA2 |
| R-BA8 | Dossiers de commission et compte rendu **pseudonymisés par construction** ; tout attribut qui réduit l'ensemble des élèves possibles à moins de 3 est masqué ; agrégats < 3 affichés « moins de 3 » | BA5 |
| R-BA9 | L'avis social est **codé** ; aucune narration de l'assistante sociale dans le 26 ; le 26 ne lit jamais le socle 36 | BA5 |
| R-BA10 | Décision d'urgence possible sans commission, motivée, portée à la connaissance de la commission suivante | BA5 |
| R-BA11 | **Privé** : le chef **propose**, le recteur décide (fonds sociaux) ; aucune imputation des fonds collégien/lycéen (versés par l'État à la famille) | BA6 |
| R-BA12 | **Public** : bourse déduite par défaut ; **privé** : **pas de procuration valide, pas de déduction** ; jamais d'imputation d'une aide sur la contribution des familles sans accord écrit du bénéficiaire (et alors par reversement puis paiement) | BA2, BA3 |
| R-BA13 | Fonds social pour les cantines : affecté **exclusivement** aux frais de restauration ; jamais versé à la famille ; reliquat reversé | BA5, BA6 |
| R-BA14 | **Retenue** pour absentéisme : jamais automatique ; proposée au chef au-delà du seuil, décidée par lui, motivée, notifiée avec voies de recours | BA4 |
| R-BA15 | La somme des aides imputées sur une ligne ne dépasse jamais son montant ; l'excédent est refusé et signalé | BA7 |
| R-BA16 | Notifications push et SMS **sans contenu** (« Nouveau document dans votre espace ») ; aucun nom d'aide sur un écran verrouillé | BA2 |
| R-BA17 | Information et relances adressées à **toutes** les familles ; aucun ciblage par profil social supposé (ni par la machine, ni par l'IA) | BA1 |
| R-BA18 | Toute consultation nominative d'un dossier d'aide par un personnel est journalisée et visible du DPO | BA1 |
| R-BA19 | **Minimisation des ressources** : pas de revenu stocké pour les bourses nationales ; pour le quotient, revenu et parts effacés à échéance, tranche conservée | BA8, BA14 |
| R-BA20 | Aucune pièce ni donnée de santé ; un besoin de soins est un **code** ; pièces typées « santé » refusées | BA1 |
| R-BA21 | Un personnel **signale un besoin** ; il ne dépose pas de demande au nom d'une famille et ne connaît pas la suite | BA15 |
| R-BA22 | Le besoin exprimé par un élève mineur ne déclenche aucune information automatique aux parents ; la suite relève de l'évaluation de l'assistante sociale | BA5 |
| R-BA23 | Aucun personnel n'instruit, ne décide ni n'exécute une aide qui le concerne, lui ou un proche déclaré ; il ne siège pas sur ce dossier | BA15 |
| R-BA24 | Une échéance (24/25) liée à une demande d'aide en cours n'est **pas relancée** ; un élève n'est jamais retiré d'un séjour ou de la cantine pour raison financière pendant l'instruction | BA3, BA9 |
| R-BA25 | Les reversements suivent les gardes du 25 (IBAN neuf : seconde validation et délai ; jamais vers un autre que le bénéficiaire) | BA3 |
| R-BA26 | Les bilans des fonds sociaux sont produits **par année civile** comme l'exige la circulaire, en plus de l'année scolaire | BA11 |
| R-BA27 | La comptabilité voit un libellé neutre (« Aide n° A-2026-014 »), jamais l'objet ni le dispositif social détaillé, sauf pour l'état de réalisation exigé | BA2 |
| R-BA28 | Un montant différent du barème national exige un motif et la pièce de la notification | BA2 |
| R-BA29 | Les membres externes de la commission n'accèdent qu'aux dossiers de la séance, pendant la séance, après engagement de discrétion | BA5 |
| R-BA30 | L'IA propose, ne décide pas ; jamais de score, de classement, de recommandation d'accord ou de refus ; jamais de texte libre, de pièce, d'avis social ni de santé | BA12 |
| R-BA31 | Toute décision qui engage de l'argent (accord, retenue, reversement, restitution) est un geste nominatif avec passkey récente | BA2 |
| R-BA32 | Une aide accordée ne disparaît jamais d'un document déjà remis : correction par avoir ou facture de régularisation | BA3 |
| R-BA33 | Une demande n'est jamais perdue faute de canal : courrier postal si pas de compte ni de courriel | BA1 |
| R-BA34 | Le secrétariat voit la présence des pièces, pas leur contenu, ni les montants, ni les décisions détaillées | BA0, BA1 |
| R-BA35 | L'élève majeur bénéficiaire est seul destinataire de ses décisions | BA2 |
| R-BA36 | Les données d'aides d'un établissement ne sont jamais visibles d'un autre (groupe compris) | BA1 |
| R-BA37 | Les pièces d'aide vivent dans un domaine documentaire **restreint** `aides` (socle 7), exclu des exports de dossier élève et du dossier transmis au nouvel établissement | BA1 |
| R-BA38 | Réceptions et reversements idempotents (clés de source) ; une restauration ne rejoue rien (E9) | BA3 |
| R-BA39 | Les droits sont donnés à des **personnes nommées** par dispositif, pas à un rôle entier (D-017) | BA1 |
| R-BA40 | Charges utiles d'événements minimales : identifiants et statut, jamais échelon, montant ni motif | BA0 |

---

## 19. Droits et minimisation

### 19.1 Catalogue d'actions (D-017) [R]
Remplace `bourses.gerer` (trop large) par :

| Droit | Portée | Défaut |
|---|---|---|
| `aides.lire` | dispositif | instructeurs désignés, direction |
| `aides.parametrer` | établissement | direction |
| `aides.informer` | établissement | direction |
| `aides.deposer` | établissement | secrétariat (registre des dépôts, présence des pièces) |
| `aides.pieces_lire` | dispositif | instructeurs |
| `aides.transmettre` | dispositif | instructeurs (et secrétariat de scolarité pour la liste des accords) |
| `aides.decisions_saisir` | dispositif | instructeurs |
| `aides.decider` ★ | établissement | chef d'établissement (délégation nommée et datée possible, M-669 du 24) |
| `aides.executer` | établissement | comptabilité |
| `aides.avis_social` | établissements rattachés | assistante sociale |
| `aides.commission_sieger` | séance | membres nommés |
| `aides.quotient` | établissement | instructeur du quotient |
| `aides.signaler` | établissement | tout personnel |
| `aides.demander` | soi | responsables, élèves majeurs ; élèves (besoin) |
| `aides.pilotage` | établissement | direction, OGEC (agrégé) |
| `aides.exporter` | établissement | direction (D-022) |
| `aides.importer` | établissement | comptabilité |

`bourses.gerer` est conservé comme alias de transition (Admin, Direction, Compta, F0) jusqu'au retrait de `ui.rs`.

### 19.2 Pièges de minimisation à tester
1. Secrétariat : dossier élève sans section aides (BD-01) ; registre des dépôts sans montants.
2. Parent non bénéficiaire : aucune section, aucun avis, facture identique avec ou sans aide chez l'autre (BD-02, BD-03).
3. Notification push : sans contenu (R-BA16).
4. Professeur, CPE, organisateur : aucune liste, aucun compteur nominatif ; agrégats « moins de 3 ».
5. Comptabilité : libellé neutre, pas d'objet (R-BA27).
6. Membre externe : rien hors séance ; lien expiré après la séance.
7. Assistante sociale partagée : seulement ses établissements.
8. IA : contexte sans texte libre ni pièce ni avis ; refus sur la question d'un tiers.
9. Événements et journaux applicatifs : aucun échelon, montant ou motif (R-BA40, E14).
10. Export du dossier élève, dossier de départ, droit d'accès : sans les données d'aide d'un autre bénéficiaire (CBA-49).
11. Un élève (compte élève) ne voit pas les aides demandées par ses parents (sauf majeur bénéficiaire).
12. Recherche globale (29) : aucune recherche « boursier » hors droit `aides.lire`.
13. Fiche 360° (2) : tuile aides absente sans droit (pas « 0 »).

---

## 20. Événements et notifications

### 20.1 Événements (outbox, dans la transaction du fait) [R]
**Publiés** : `aides.demande_deposee`, `aides.demande_etat`, `aides.demande_transmise`, `aides.decision_enregistree`, `aides.decision_corrigee`, `aides.accordee`, `aides.refusee`, `aides.attribution_suspendue`, `aides.attribution_retiree`, `aides.procuration_signee`, `aides.procuration_revoquee`, `aides.deduction_prevue`, `aides.reception_ventilee`, `aides.reversement_demande`, `aides.retenue_proposee`, `aides.retenue_decidee`, `aides.besoin_signale`, `aides.agregat_sejour`, `aides.tranche_changee`, `aides.purge`. Charge utile : identifiants, statut, période (R-BA40).
**Consommés** : `inscription.validee` (accord d'étude automatique), `scolarite.depart_*` (3) ; `eleve.regime_changed`, `eleve.majeur`, `eleve.fusionne` (2) ; `personnes.charge_modifiee`, `personne.decedee`, `payeurs.modifies` (4) ; seuil d'absences injustifiées (12, compteur seul) ; `facturation.ligne_emise`, `facturation.avoir_emis` (24) ; `paiements.reception`, `paiements.reversement_execute` (25) ; `sorties.desistement`, `sorties.echeance_solde` (42).

### 20.2 Natures de notification (socle 8) [R]
| Nature | Destinataire | Contenu de l'avis (push / SMS) | Contenu dans l'espace |
|---|---|---|---|
| `aide.information_annuelle` | tous les responsables | « Information de l'établissement » | guide, dates, liens |
| `aide.accuse` | demandeur | « Nouveau document » | accusé |
| `aide.pieces_manquantes` | demandeur | « Une démarche vous attend » | liste des pièces |
| `aide.decision` | bénéficiaire | « Nouveau document » | décision, voies de recours |
| `aide.procuration_a_signer` | bénéficiaire | « Un document à signer » | procuration |
| `aide.reversement` | bénéficiaire | « Un virement a été effectué » | montant, détail |
| `aide.retenue` | bénéficiaire | « Nouveau document » | décision motivée |
| `aide.besoin_recu` | assistante sociale, chef | « Nouveau signalement » | — |
| `aide.commission_convocation` | membres | « Convocation » | date, nombre de dossiers |
Plage calme du socle 29 (20 h – 7 h) respectée pour tout sauf les urgences de l'assistante sociale [R].

---

## 21. Écrans (`DESIGN_SYSTEM.md`, `FRONTEND_CIBLE.md`) [R]

1. **Aides — accueil de l'instructeur** (tuiles Bento) : campagne en cours (déposées, complètes, transmises, décidées), pièces manquantes, doubles demandes, attributions sans procuration, versements attendus, enveloppe et reliquat.
2. **Liste des demandes** (tableau dense) : numéro, élève (visible de l'instructeur), dispositif, canal, état, pièces, ancienneté ; filtres ; aucune colonne de revenus.
3. **Fiche demande** : étapes, pièces (vignettes, lecture journalisée), avis social codé, alternatives, historique, décision.
4. **Registre des dépôts** (secrétariat) : saisie rapide au guichet, impression de l'accusé, cases de pièces ; rien d'autre.
5. **Commission** : préparation (ordre du jour pseudonymisé), séance plein écran projetable (une fiche à la fois, avis saisis en direct), clôture, compte rendu.
6. **Exécution** (comptabilité) : ventilation d'un versement, déductions prévues et constatées, reliquats à reverser, état de réalisation.
7. **Chef** : décisions à arrêter (lot confirmé un à un, passkey), retenues proposées, urgences, tableau de bord agrégé.
8. **Famille — « Mes aides »** (web et appli) : ce que je peux demander (guide), mes demandes et leurs étapes, mes décisions, mon échéancier net, mes virements ; un bouton **« J'ai besoin d'aide »** discret, aussi sur la page d'un séjour et dans le compte financier.
9. **Élève** : « J'ai besoin d'aide » → assistante sociale (rendez-vous du socle 19).
10. **Paramétrage** : dispositifs (modèles), lignes cibles, plafonds, pièces, instructeurs, commission, tranches du quotient.

---

## 22. Modèle de données cible (brique `aides`) [R]

Conventions E1–E14 : `etablissement_id UUID NOT NULL` partout (filles comprises), FK composites `(etablissement_id, x_id)`, UUID générés par l'application, numéros lisibles par compteur d'établissement, `cree_le` / `modifie_le`, aucun `DELETE` d'un objet à historique (états datés), `RESTRICT`, RLS stricte, **aucun JSON, aucun tableau, aucun BYTEA** (fichiers = objets du socle 7). Montants en centimes `BIGINT`, contrôles `CHECK`.

### 22.1 Références globales (portée `globale_reference`, E12)
```
ref_aid_dispositif            code PK ('bourse_college','bourse_lycee','prime_equipement','prime_internat',
                              'bourse_merite','fonds_social_collegien','fonds_social_lyceen','fonds_social_cantines',
                              'bourse_agricole'), libelle, famille ('bourse','prime','fonds_social'), echelons_max SMALLINT NULL
ref_aid_bareme                (dispositif_code, annee_scolaire, echelon SMALLINT NULL) PK, montant_annuel_centimes,
                              montant_trimestriel_centimes, source_url, consulte_le DATE
ref_aid_campagne_nationale    (dispositif_code, annee_scolaire, type ('rentree','printemps')) PK, ouverture DATE, cloture DATE, source_url
```

### 22.2 Paramétrage d'établissement
```
ent_aid_dispositif            id, etablissement_id, code, libelle, ref_code NULL (dispositif national) , financeur_organisme_id NULL,
                              decideur ('autorite_nationale','recteur','chef_etablissement','commission','financeur','ogec','apel'),
                              mode_versement ('etablissement_deduit','etablissement_verse','direct_etat','direct_financeur','nature'),
                              plafond_annuel_centimes NULL, plafond_bps NULL, avec_commission BOOL, actif, cree_le, modifie_le
ent_aid_dispositif_cible      (etablissement_id, dispositif_id, categorie) PK  -- catégories du 24 ; fonds cantines : 'restauration' seule (CHECK par déclencheur)
ent_aid_dispositif_piece      (etablissement_id, dispositif_id, nature_piece) PK, obligatoire BOOL
ent_aid_reglement             id, etablissement_id, dispositif_id, version, valide_du, objet_id (socle 7), publie_le
ent_aid_instructeur           (etablissement_id, dispositif_id, personnel_id) PK, depuis, jusqua NULL
ent_aid_campagne              id, etablissement_id, dispositif_id, annee_scolaire, ouverture, cloture_interne, cloture_nationale, statut
ent_aid_compteur              (etablissement_id, serie ('D','A'), annee) PK, dernier BIGINT   -- numéros D-2026-0014, A-2026-0014
```

### 22.3 Demandes, pièces, historique
```
ent_aid_demande               id, etablissement_id, numero TEXT (unique par étab.), eleve_id, demandeur_personne_id, dispositif_id,
                              campagne_id NULL, annee_scolaire, canal ('etude_automatique','teleservice_national','papier','scolia',
                              'assistante_sociale','sejour','compte_financier'), objet_code NULL, objet_texte VARCHAR(200) NULL,
                              montant_demande_centimes NULL, sortie_id NULL (FK composite socle 42), echeance_id NULL (24),
                              urgence BOOL, hors_delai BOOL, etat ('deposee','incomplete','complete','transmise','en_commission',
                              'decidee','retiree','close'), depose_le, cree_le, modifie_le
ent_aid_demande_etat          id, etablissement_id, demande_id, etat, motif_code NULL, le, par_personne_id  -- ajout seul
ent_aid_demande_piece         id, etablissement_id, demande_id, nature_piece ('avis_imposition','attestation_caf','rib',
                              'procuration','notification','devis','justificatif','autre'), objet_id (socle 7), ajoute_le, retire_le NULL
ent_aid_accord_etude_auto     id, etablissement_id, eleve_id, personne_id, annee_scolaire, donne_le, retire_le NULL, transmis_le NULL
ent_aid_besoin                id, etablissement_id, eleve_id, origine ('eleve','personnel','famille'), signale_par_personne_id,
                              categorie_code, pris_en_charge_par NULL, pris_en_charge_le NULL, clos_le NULL  -- sans texte libre
```

### 22.4 Instruction, commission, enveloppes
```
ent_aid_avis_social           id, etablissement_id, demande_id, assistante_personne_id, avis ('favorable','reserve','defavorable','sans_avis'),
                              priorite SMALLINT NULL, situation_connue BOOL, le
ent_aid_alternative_verifiee  (etablissement_id, demande_id, alternative_code) PK, verifiee_le, par
ent_aid_enveloppe             id, etablissement_id, dispositif_id, periode_debut, periode_fin, montant_notifie_centimes, objet_id NULL
ent_aid_enveloppe_mouvement   id, etablissement_id, enveloppe_id, nature ('notification','engagement','consommation','annulation','reversement'),
                              montant_centimes, attribution_id NULL, le   -- ajout seul ; reliquat calculé
ent_aid_commission            id, etablissement_id, dispositif_famille, date_seance, statut ('convoquee','ouverte','close'), cr_objet_id NULL
ent_aid_commission_membre     id, etablissement_id, commission_id, personne_id, qualite, present BOOL
ent_aid_discretion            id, etablissement_id, personne_id, annee_scolaire, signee_le, objet_id
ent_aid_commission_dossier    id, etablissement_id, commission_id, demande_id, pseudonyme TEXT, avis ('favorable','defavorable','ajourne') NULL,
                              montant_propose_centimes NULL, forme_proposee NULL
ent_aid_conflit               (etablissement_id, demande_id, personne_id) PK, declare_le
ent_aid_lien_seance           id, etablissement_id, commission_id, personne_id, empreinte_jeton, expire_le, utilise_le NULL  -- E9 : neutralisé
```

### 22.5 Décisions, attributions, versements
```
ent_aid_decision              id, etablissement_id, demande_id, version SMALLINT, decideur, decision ('accord','refus','ajournement'),
                              echelon SMALLINT NULL, montant_annuel_centimes NULL, motif_code NULL, reference TEXT NULL, decide_le DATE,
                              recours_delai_jours SMALLINT NULL, recours_autorite TEXT NULL, objet_id NULL, saisie_par, cree_le
                              -- UNIQUE (etablissement_id, demande_id, version) ; jamais d'UPDATE (déclencheur)
ent_aid_decision_prime        id, etablissement_id, decision_id, dispositif_prime ('prime_equipement','prime_internat','bourse_merite'),
                              montant_annuel_centimes, deductible BOOL
ent_aid_attribution           id, etablissement_id, numero, decision_id, eleve_id, beneficiaire_personne_id, dispositif_id,
                              annee_scolaire, du DATE, au DATE, montant_annuel_centimes, mode_versement, etat ('active','suspendue',
                              'retiree','transferee','close'), cree_le, modifie_le
ent_aid_attribution_etat      id, etablissement_id, attribution_id, etat, motif_code, le, par  -- ajout seul
ent_aid_procuration           id, etablissement_id, beneficiaire_personne_id, eleve_id, annee_debut, annee_fin, signee_le, objet_id,
                              revoquee_le NULL
ent_aid_echeance              id, etablissement_id, attribution_id, rang SMALLINT (1-3), periode_du, periode_au, montant_prevu_centimes
ent_aid_reception             id, etablissement_id, financeur_organisme_id NULL, dispositif_id, encaissement_ref (25), montant_centimes, recu_le,
                              ventilee_le NULL, ventilee_par NULL
ent_aid_reception_ligne       id, etablissement_id, reception_id, echeance_id, montant_centimes
ent_aid_mouvement             id, etablissement_id, attribution_id, echeance_id NULL, nature ('deduction_prevue','deduction_constatee',
                              'reversement','restitution','regularisation','nature','paiement_tiers'), montant_centimes, source_cle TEXT
                              (unique par étab.), ref_externe (ligne 24 ou opération 25), le   -- ajout seul
ent_aid_retenue               id, etablissement_id, attribution_id, journees_injustifiees SMALLINT, montant_centimes, motif TEXT,
                              decidee_par, decidee_le, annulee_le NULL, objet_id
ent_aid_recours               id, etablissement_id, decision_id, depose_le, autorite, issue ('accueilli','rejete','en_cours'), decision_suite_id NULL
ent_aid_proposition_rectorat  id, etablissement_id, dispositif_id, transmise_le, signee_par, objet_id ;  ent_aid_proposition_ligne (..., demande_id, montant_propose)
ent_aid_etat_realisation      id, etablissement_id, reception_id, objet_id, etabli_le
```

### 22.6 Quotient familial (remplace `ent_foyer_quotient`)
```
ent_aid_qf_tranche            id, etablissement_id, annee_scolaire, version, code, borne_min_centimes, borne_max_centimes NULL
ent_aid_quotient              id, etablissement_id, personne_id (payeur), co_payeur_personne_id NULL, annee_scolaire, source ('rfr','caf',
                              'honneur'), annee_revenus SMALLINT, ressources_centimes NULL (effacé à échéance), parts_x100 NULL (effacé),
                              quotient_centimes, tranche_id, valide_du, valide_au NULL, objet_id NULL, valide_par, cree_le
```

### 22.7 Registre (E12), D-036, fichiers
- Toutes les tables `ent_aid_*` : portée `etablissement`, domaine `aides`, catégorie d'export **« double »** (données sociales de mineurs : export complet sous double validation, D-041) ; `ref_aid_*` : `globale_reference`.
- E9 : `ent_aid_lien_seance` (jetons) **neutralisés** à la restauration ; aucune file propre (avis et reversements passent par 8 et 25, déjà déclarés).
- **D-036** : **aucune exception** ; contrôle `grep -i "json\|bytea\|\[\]"` sur 2520–2539.
- Fichiers : pièces et documents = objets du socle 7, domaine `aides` restreint (E5).

---

## 23. Conservation (durées **à valider par le DPO**, Q-BA1) [R]
| Données | Durée proposée | Après |
|---|---|---|
| Pièces justificatives de ressources (avis d'imposition, attestation CAF) | fin de l'année d'attribution + 3 mois (délai de contestation) | supprimées |
| Revenu et parts du quotient | idem | effacés, tranche conservée |
| Demandes refusées ou retirées | 1 an après la fin de l'année | supprimées |
| Décisions, attributions, mouvements (preuve comptable) | durée comptable (10 ans [NV pour un OGEC], M9.6 pour un EPLE [NV]) | anonymisées au-delà |
| Comptes rendus de commission (anonymisés) | 5 ans [R] | archivés |
| Table de correspondance pseudonyme ↔ élève | fin de l'année + 1 an | supprimée |
| Besoins signalés | 1 an après clôture | supprimés |
| Journal des consultations | socle 27 | — |
Purge planifiée (BRS-271), jamais hors procédure (E7).

---

## 24. Reprise des données (existant Rust, Charlemagne, Next.js)
1. **Existant Rust** (migration 2533) : `ent_bourse_type` → `ent_aid_dispositif` (rapprochement du code national par libellé, à confirmer ligne à ligne) ; `ent_eleve_bourse` → demande + décision version 1 + attribution, **bénéficiaire à désigner** (aucun n'est connu) : tâche par attribution, **aucune déduction** tant qu'il n'est pas désigné ; `avec_procuration = true` → procuration **sans pièce** marquée « à régulariser » (pas de déduction après la date de reprise sans pièce, R-BA12) ; `ent_fin_bourse_bareme` → comparaison au barème national (écarts listés) ; `ent_foyer_quotient` → `ent_aid_quotient` par **payeur** du foyer (si un seul payeur) sinon tâche.
2. **Charlemagne** : bourses et échelons de l'année (F-01 à F-04), **en doublon** (§25), sans pièces ; aucune reprise des années closes.
3. **Next.js** : aides individuelles des séjours (VD-37) → attributions de dispositifs locaux (APEL, caisse, fonds social), sur export fourni par Florian (D-035 point 4).
4. Rapport de reprise : nombre, écarts, tâches créées ; rejouable (E8).

---

## 25. Année de doublon 2027-2028 (D-034, D-035) [R]
- **Bourses** : Charlemagne reste officiel pour la facturation (D-034) ; ScolIA **calcule en parallèle** déductions et reliquats, sans reverser ni émettre (mode **suivi**, comme 42 §32.5) ; la recette compare les deux.
- **Fonds sociaux et caisse de solidarité** : Charlemagne ne gère pas la commission (FIN-15 « partiel » de la matrice [F]) ; **recommandation** : usage réel possible de la **commission** et des **demandes en ligne** dès la rentrée 2027 (gain immédiat, aucun argent bougé par ScolIA), l'exécution comptable restant dans Charlemagne ; **décision de Florian** (Q-BA5).
- **Demandes des familles en ligne** : seulement après l'accord écrit de l'établissement et l'avis du DPO (données réelles).

---

## 26. Lots et migrations

### 26.1 Lots (équipe Sonnet)
| Lot | Contenu | Dépend de | Migrations | Critères de fin |
|---|---|---|---|---|
| **BA0** | correctifs BD-01 à BD-04, BD-09 (§2.3) | — | **aucune** | 4 tests de non-régression ; revue sécurité |
| **BA1** | référentiel national, dispositifs, campagnes, demandes, pièces (socle 7), registre des dépôts, accusés, historique, double demande, information de toutes les familles, droits fins, journal des consultations | socles 4, 7, 8 | **2520** (références globales), **2521** (dispositifs, campagnes, demandes, pièces, accords, compteurs) | R-BA1, 6, 17, 18, 20, 34, 37, 39 testés |
| **BA2** | décisions versionnées, attributions, primes, procurations, notifications au bénéficiaire | BA1 | **2522** | R-BA4, 7, 12, 16, 28, 31, 35 |
| **BA3** | échéances, réceptions, ventilation, déduction (24 F8), reliquat et reversement (25 P7), restitution, régularisation | BA2 ; 24 F8 ; 25 P7 | **2523** | R-BA3, 24, 25, 32, 38 ; scénario Haddad (24 P08) |
| **BA4** | changements (départ, arrivée, régime, charge, décès, majorité), retenues, recours, clôture d'année | BA3 ; 12 | **2524** | R-BA5, 14 |
| **BA5** | fonds sociaux public : enveloppes, besoins, avis social, commission pseudonymisée, urgence, décision, nature, fournisseur | BA2 | **2525** (instruction, enveloppes), **2526** (commission) | R-BA8, 9, 10, 13, 22, 29 |
| **BA6** | privé : propositions au recteur, fonds cantines, état de réalisation, reliquat | BA5 | **2527** | R-BA11, 13 |
| **BA7** | aides locales, caisse de solidarité, APEL, financeurs, double financement | BA3 | **2528** | R-BA15 |
| **BA8** | quotient familial par payeur, tranches versionnées, effacement | BA1 ; 24 | **2529** | R-BA19 |
| **BA9** | voyages : demande depuis le séjour, agrégats, désistement, synthèse APEL | BA7 ; 42 V-lots | **2530** | R-BA24, R-SV18 |
| **BA10** | espace famille et élève (web, appli), guide, simulateur | BA2, BA3 | aucune | parcours mobiles |
| **BA11** | bilans (année scolaire et civile), recensement, rapport CA, imports nationaux | BA5 | **2531** | R-BA26 |
| **BA12** | IA : capacités du §16.3 | BA1–BA5 | aucune | R-BA30 ; tests de refus |
| **BA13** | EPLE : agent comptable, mandats 6576, écritures, Op@le | BA3 ; 24 F13 ; 48 | **2532** | revue avec un gestionnaire public |
| **BA14** | conservation, purge, reprise de l'existant et des séjours Next.js, retraits | BA1–BA9 | **2533** (reprise), **2539** (retraits, sur décision, E8) | rapport de reprise ; purge testée |
| **BA15** (personas) | signalements de besoin, conflits d'intérêts, comptes protégés, contrôles internes | BA1 | **2534** | R-BA21, 23 |
| — | réserve | — | 2535–2537 | — |
| — | RLS stricte sur `ent_aid_*` | BA1 | **2538** | test strict |

**Parcours minimal pour le jalon V1 (fin janvier – février 2027)** : BA0, BA1, BA2, BA3 (bourses de bout en bout au privé, scénario Haddad), BA5 (commission), BA10 partiel ; le reste en épaississement.

### 26.2 Migrations (registre `MIGRATIONS.md`)
| N° | Lot | Contenu |
|---|---|---|
| 2520 | BA1 | `ref_aid_dispositif`, `ref_aid_bareme`, `ref_aid_campagne_nationale` (données 2026-2027 avec source) |
| 2521 | BA1 | `ent_aid_dispositif`, `_cible`, `_piece`, `ent_aid_reglement`, `ent_aid_instructeur`, `ent_aid_campagne`, `ent_aid_compteur`, `ent_aid_demande`, `_etat`, `_piece`, `ent_aid_accord_etude_auto` |
| 2522 | BA2 | `ent_aid_decision`, `_prime`, `ent_aid_attribution`, `_etat`, `ent_aid_procuration` ; déclencheur d'interdiction d'`UPDATE` des décisions |
| 2523 | BA3 | `ent_aid_echeance`, `ent_aid_reception`, `_ligne`, `ent_aid_mouvement` |
| 2524 | BA4 | `ent_aid_retenue`, `ent_aid_recours` |
| 2525 | BA5 | `ent_aid_besoin`, `ent_aid_avis_social`, `ent_aid_alternative_verifiee`, `ent_aid_enveloppe`, `_mouvement` |
| 2526 | BA5 | `ent_aid_commission`, `_membre`, `_dossier`, `ent_aid_discretion`, `ent_aid_conflit`, `ent_aid_lien_seance` |
| 2527 | BA6 | `ent_aid_proposition_rectorat`, `_ligne`, `ent_aid_etat_realisation` |
| 2528 | BA7 | financeurs et plafonds complémentaires (colonnes de `ent_aid_dispositif`, contrôle de cumul) |
| 2529 | BA8 | `ent_aid_qf_tranche`, `ent_aid_quotient` |
| 2530 | BA9 | clés composites vers les séjours (42), vue d'agrégats |
| 2531 | BA11 | vues de bilans (année civile) |
| 2532 | BA13 | correspondances comptables publiques |
| 2533 | BA14 | reprise de `ent_bourse_type`, `ent_eleve_bourse`, `ent_fin_bourse_bareme`, `ent_foyer_quotient` |
| 2534 | BA15 | comptes protégés, déclarations de proches (si non portés par le 24 R-F35) |
| 2535–2537 | — | réserve |
| 2538 | — | RLS stricte |
| 2539 | BA14 | retrait des tables héritées (sur décision, E8) |

---

## 27. Tests et recette
- **Unitaires** : prévision trimestrielle, ventilation d'un versement global, déduction plafonnée au payeur bénéficiaire, reliquat, prime non déductible, cumul ≤ 100 %, tranches, petits effectifs (R-BA8).
- **Postgres** : versions de décision non modifiables, idempotence des mouvements, RLS stricte, FK composites, purge.
- **Scénarios** : famille Haddad (lycée privé, procuration, excédent) ; parents séparés en résidence alternée (double demande, non-payeur bénéficiaire) ; collège public (déduction par défaut, retenue) ; commission avec membres externes ; fonds cantines privé et état de réalisation ; voyage de Rome avec 5 aides ; élève majeur ; départ en février.
- **Séparation** (générés depuis les routes, D-026) : secrétariat, professeur, CPE, parent non bénéficiaire, élève, autre établissement, membre externe hors séance, assistante sociale d'un autre établissement.
- **Recette par personas** (D-021 étape 4) : `PERSONAS_26.md`, avec les attaques de P14.

---

## 28. Réglages signalés (D-023) — S-2600 à S-2615 (S-2616, S-2617 : §30)
| # | Réglage | Défaut | Commentaire |
|---|---|---|---|
| S-2600 | Date limite interne de dépôt papier | date nationale − 5 jours ouvrés | jamais après la date nationale |
| S-2601 | Lecture des pièces par l'instructeur seulement | oui | — |
| S-2602 | Enregistrer les bourses `direct_etat` (privé sans procuration) | seulement sur pièce fournie par la famille | minimisation |
| S-2603 | Durée de la procuration | l'année scolaire | pluriannuelle possible |
| S-2604 | Pièces facultatives pour les fonds sociaux | oui | circulaire : dossier simple |
| S-2605 | Nombre maximal de pièces exigibles | 3 | usine à gaz évitée |
| S-2606 | Instructeurs | nommés par dispositif | — |
| S-2607 | Niveau plutôt que classe sur la fiche de commission | oui | identifiabilité |
| S-2608 | Vote en commission | non (avis consensuel) | — |
| S-2609 | Source du quotient | revenu fiscal de référence | ou CAF |
| S-2610 | Effacement des ressources du quotient | fin d'année + 3 mois | DPO |
| S-2611 | Déclaration sur l'honneur admise | non | si oui, contrôle par sondage |
| S-2612 | Seuil de signalement d'un reliquat d'enveloppe | 30 % en avril | — |
| S-2613 | Décompte de la retenue | journées | ou demi-journées (Q-BA4) |
| S-2614 | Agrégat minimal affiché | 3 | — |
| S-2615 | Délai de reversement d'un reliquat | 15 jours après validation de la ventilation | — |
| S-2616 | (personas) Délai de l'avis social | 7 jours | puis « sans avis » |
| S-2617 | (personas) Alerte de consultations inhabituelles | 10 dossiers d'aide par heure hors demandes traitées | alerte DPO |

---

## 29. Décisions appliquées par défaut (D-031) et questions

### 29.1 Appliqué par défaut (recommandations, Florian peut revenir dessus)
1. **Lot BA0 sans migration** avant tout autre lot du domaine (fermeture des sections aides au secrétariat et aux parents, suppression des envois automatiques, pas de déduction en multi-payeurs) — Q-BA-A.
2. **Bénéficiaire = personne** ; déduction sur le seul payeur bénéficiaire (corrige BD-04 et le résidu de FD-12) — Q-BA-B.
3. **Remplacement** de `bourses.gerer` par les droits fins du §19.1, nominatifs par dispositif ; alias de transition — Q-BA-C.
4. **Pseudonymisation de la commission par construction**, règle des petits effectifs à 3 — Q-BA-D.
5. **Aucune évaluation par l'IA** (ni score, ni proposition de montant) — Q-BA3 côté produit.
6. **Pas de revenu stocké** pour les bourses nationales ; quotient : effacement à échéance, tranche conservée.
7. **Retenue jamais automatique** ; proposée au chef.
8. **Signalement de besoin** par les personnels au lieu de demandes en leur nom.
9. **Privé** : fonds collégien/lycéen tracés sans imputation ; fonds cantines affecté et justifié.
10. **Plage 2520–2539** (imposée) ; abandon de la réservation 5600–5699 / 560–579.
11. Ajouts de la passe personas (§30) : tous appliqués.

### 29.2 Ce qui engage Florian (à trancher par lui)
| # | Question | Recommandation |
|---|---|---|
| Q-BA1 | Durées de conservation (§23) et contenu du droit d'accès (CBA-49) | **DPO** ; propositions du §23 |
| Q-BA2 | AIPD des données d'aides (sociales, mineurs) | à conduire avant toute donnée réelle, avec l'AIPD finances (24 Q-F29) |
| Q-BA3 | Étude « règlement européen sur l'IA » pour les capacités d'aide | à joindre à l'étude de D-032 ; aucune capacité d'évaluation prévue |
| Q-BA4 | Décompte de l'absentéisme pour la retenue (journées / demi-journées) | relire l'article R. 531-31 en version consolidée ; défaut journées |
| Q-BA5 | Commission et demandes en ligne **en usage réel** dès 2027-2028 ? | oui pour la commission (pas d'argent bougé), après accord écrit et DPO |
| Q-BA6 | Interfaces nationales (SIECLE Bourses, Scolarité services, Orquestra) : spécifications éditeur | à inclure dans la démarche éditeur (F-06, mai 2027) ; d'ici là exports manuels |
| Q-BA7 | Format des propositions attendu par les services académiques (privé) | demander à la DSDEN / au rectorat de Florian (pièce réelle) |
| Q-BA8 | Restitution des versements reçus pour un élève parti (privé) | procédure à confirmer avec les services académiques |
| Q-BA9 | Modèle de procuration, voies de recours de la caisse de solidarité, reliquat d'un bénéficiaire décédé, compte joint | **juriste** |
| Q-BA10 | Délégués élèves mineurs en commission ; besoin d'un mineur sans information des parents | avis de l'assistante sociale et du juriste ; défaut prudent du §13 |
| Q-BA11 | Existence et règlement d'une caisse de solidarité de l'OGEC ; aides APEL individuelles et convention | à fournir par Florian (pièces réelles) |
| Q-BA12 | Public (D-029) : écritures des bourses en M9.6 / Op@le | revue avec un gestionnaire d'EPLE avant BA13 |

---

## 30. Ajouts de la passe personas (appliqués par défaut, D-031 ; l'emportent en cas de contradiction)
Détail : `PERSONAS_26.md` (17 personas, 58 manques, 7 bloquants).
1. **Bloquants appliqués** : R-BA41 (registre des dépôts sans recherche par élève, rien au téléphone) ; R-BA42 (tâches et compteurs du 29 adressés au seul bénéficiaire) ; R-BA43 (état civil du concubin effacé après transmission) ; R-BA44 (membre de commission : dossiers de sa classe et de son entourage masqués) ; R-BA45 (aucun indice de suivi social sur la fiche de commission ; le champ `situation_connue` de `ent_aid_avis_social` n'est lisible que du chef) ; R-BA46 (documents de contrôle nominatifs produits par la direction, journalisés, voie sûre) ; BRS-334 (boursier venu du public : aucune déduction avant procuration).
2. **Actions** BRS-300 à BRS-337 (`PERSONAS_26.md` §3) ; **cas limites** CBA-61 à CBA-74 (§5) ; **réglages** S-2616 (délai de l'avis social, 7 jours) et S-2617 (alerte de consultations inhabituelles).
3. **Règles précisées** : R-BA14 — la retenue n'apparaît jamais dans les écrans de justification des absences ; R-BA24 — l'accès au service (repas, séjour) est maintenu pendant l'instruction ; R-BA31 — chaque décision porte un auteur humain, les propositions automatiques sont marquées comme telles (RGPD art. 22).
4. **Modèle** : `ent_aid_commission_membre` reçoit le lien avec ses élèves proches (classe du délégué, enfants du parent élu, entourage déclaré) calculé à la séance, sans stockage supplémentaire ; `ent_aid_accord_etude_auto` : colonnes d'état civil du concubin **nullables et effacées** (R-BA43) ; `ent_aid_import_etat_paiement` (BRS-304) rattaché à `ent_aid_reception` (2523).
5. **Lots** : BA15 porte BRS-315, BRS-326 ; BA5 porte BRS-300, 303, 312, 313, 319 à 321 ; BA4 porte BRS-302, 333, 334 ; BA1 porte BRS-301, 308 à 311, 318, 327.

---

## 31. Faits, hypothèses, contradictions
**Faits [F]** (`ent-rust/`, `afeb8c4`) : tables `ent_bourse_type`, `ent_eleve_bourse` (017), `ent_fin_bourse_bareme` (200), `ent_fac_echeance.bourse_centimes` (033), `ent_foyer_quotient` (075) ; RLS (582) ; déduction répartie (`finances.rs:1084-1104`) ; sections du dossier (`service.rs:4680-4781`) ; notifications à tous les parents (`service.rs:10742-10785`) ; `bourses.gerer` à Admin, Direction, Compta (`roles.rs:216`) ; aucun module Next.js d'aides (§2.4).
**Hypothèses [H]** : calendrier du §4 ; virements globaux de l'État sans liste ; assistante sociale partagée ; prime d'internat déductible si la procuration le prévoit ; transfert de bourse géré par les services académiques ; pratique APEL.
**Non vérifié [NV]** : répartition exacte de la décision de bourse de collège au public ; articles L. 411-3 CASF et 226-13 CP (connus, non relus) ; écritures des bourses en M9.6 ; durées comptables ; bourse au mérite (montants relus sur une copie d'établissement) ; aides de la voie professionnelle ; aides des collectivités.
**Contradictions** : voir `PERSONAS_26.md` §4 (24 §5.7 précisé, quotient par payeur, plage de migrations remplacée).
**Sources consultées le 2026-11-11** : service-public.gouv.fr F984 (https://www.service-public.gouv.fr/particuliers/vosdroits/F984) et F616 (https://www.service-public.gouv.fr/particuliers/vosdroits/F616), vérifiées par l'administration le 30/04/2026 ; circulaire n° 2017-122, BO n° 28 du 31/08/2017 (https://www.education.gouv.fr/bo/17/Hebdo28/MENE1718891C.htm ; PDF https://cache.media.education.gouv.fr//file/28/41/3/BO-MEN-28-OK_803413.pdf, pages 33-36) ; barème lycée 2026-2027 (annexe 7, https://www.education.gouv.fr/sites/default/files/document/annexe-7-bareme-des-bourses-nationales-d-etudes-du-second-degre-de-lycee-annee-scolaire-2026-2027.pdf, repéré, non relu ; copie d'établissement pour la bourse au mérite). Sources du 2026-11-10 : `PERSONAS_24_25.md` §5.
