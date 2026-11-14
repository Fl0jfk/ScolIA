# Socle 3 — Inscriptions et réinscriptions

**Date :** 2026-11-06 — **Statut :** conception, **non décidée** (à valider par Florian). Aucun code écrit, rien compilé ni testé pour ce document. Aucun accès à la base ni aux secrets de production ; l'application Next.js a été lue en lecture seule ciblée, sans copie de code.
**Branche :** `ent-socle-inscriptions-conception`.
**À lire avec :** `CLAUDE.md`, `docs/VISION.md`, `docs/PRODUCT_SPEC.md`, `docs/DECISIONS.md` (D-013, D-014, D-017, D-019, D-020, D-021), `docs/ROLES_ET_DROITS.md`, `docs/SOCLES.md` (§3, socles 3 et 4), **`docs/socles/02_ELEVE.md`** (sur lequel ce document s'appuie), `docs/SOCLE2_DOMAINES.md` (D3, actions `INS-xx`), `docs/IDENTITE_ET_PROFILS_FAMILLES.md`, `docs/INTEROPERABILITE_NATIONALE.md`.

**Conventions.** **[F]** fait vérifié dans le dépôt (chemin cité ; les chemins `lot5:` renvoient à la branche `origin/ent-integration-lot5`, la plus avancée, qui contient tout `ent-socle2-inscriptions`) ; **[F Next]** fait lu dans l'application Next.js ; **[S]** fait sourcé sur le web (source citée au §19) ; **[H]** hypothèse (mémoire ou déduction) ; **[NV]** règle réglementaire non vérifiée sur un texte officiel, **à vérifier avant de coder** ; **[R]** recommandation (non décidée). Les identifiants d'actions métier `INS-xx` existants sont conservés ; les nouveaux prennent la suite (`INS-16`…), sans renumérotation.

**[personas] Passe « personas » du 2026-11-06.** Ajouts marqués **[personas]** : actions INS-53 à INS-84 (§11), règles R-I1 à R-I27 et cas limites (§17 bis), questions Q-I21 à Q-I26 (§18). Origine de chaque ajout : `docs/socles/PERSONAS_02-04.md`. **À signaler au lot I0 en cours** : un responsable marqué « à ne pas solliciter » ne reçoit ni demande d'accord, ni invitation, ni relance (INS-57, R-I8) ; une adresse déclarée « à protéger » n'est jamais transmise en clair au socle 4 (INS-58, R-I9 ; lot A0 du socle 4).

Sommaire : §1 Thèse · §2 Évaluation de l'existant · §3 Acteurs et ce qu'ils vivent · §4 Public, privé sous contrat, hors contrat · §5 Frontières (socles 2 et 4) · §6 Concepts et vocabulaire · §7 Cycles de vie · §8 Modèle de données cible · §9 Logiciel ou paramétrage · §10 Échanges avec l'extérieur · §11 Actions métier · §12 Droits · §13 Événements · §14 Écrans · §15 Données personnelles, sécurité, équité · §16 Plan de migration · §17 Lots de code · §18 Questions à trancher · §19 Récapitulatif faits / hypothèses / sources.

---

## 1. Thèse : ce que veut dire « bien gérer les inscriptions »

Une inscription n'est pas un formulaire : c'est **une saison** que chaque établissement revit tous les ans, avec des pics brutaux (décembre–mars dans le privé, fin juin–début juillet dans le public après l'affectation, dernière semaine d'août pour tout le monde), des familles inquiètes, un secrétariat débordé, une direction qui doit remplir ses classes sans les dépasser et préparer l'année suivante (moyens, budget, structure) **avant** de connaître ses effectifs. Le socle 3 est le domaine qui **fait entrer un enfant, le fait rester d'une année sur l'autre, et le fait sortir proprement**, et qui dit à tout moment **combien d'élèves l'établissement aura à la rentrée**.

Huit principes [R] :

1. **Trois portes, un seul domaine.** Entrer (admission, ou inscription d'un élève affecté par l'administration dans le public), rester (réinscription), sortir (départ). Mêmes campagnes, mêmes dossiers, mêmes pièces, mêmes écrans, mêmes règles d'historique. Aujourd'hui l'existant a **deux** mécanismes de réinscription et **aucun** pour le départ [F, §2].
2. **Le processus appartient au socle 3, le fait aux socles 2 et 4.** Le socle 3 conduit la démarche (dossier, étapes, décision, confirmation) ; quand l'enfant est inscrit, il **appelle les commandes** du socle 2 (élève, scolarisation, inscription annuelle) et du socle 4 (personnes, liens, foyers). Il n'écrit jamais dans leurs tables. L'existant fait l'inverse (INSERT directs, §2.2) : c'est le premier chantier.
3. **On collecte au fil de l'eau, jamais tout au début.** Un contact ne donne que son e-mail et le niveau visé ; une candidature, ce qui sert à décider ; une admission confirmée, ce qui sert à inscrire ; la rentrée, ce qui sert à vivre au quotidien (santé, autorisations, contacts d'urgence : socles 34, 18, 4). Demander le carnet de santé d'un enfant qui ne sera peut-être pas admis est à la fois une perte de temps et une faute de minimisation.
4. **Chaque place est comptée en temps réel.** Capacité par niveau, par parcours (option, section, filière) et par régime (internat surtout) ; places prises par les réinscrits, offertes, acceptées, libérées. La direction voit à tout instant ce qui reste, et la liste d'attente sait qui appeler quand une place se libère.
5. **La famille sait toujours où elle en est, et ce qu'on attend d'elle.** Une page « Mes démarches » par enfant, avec l'étape, la prochaine échéance, les pièces manquantes et l'historique des échanges ; plus jamais d'appel au secrétariat pour « savoir si le dossier est arrivé ». Les relances partent seules, mais sobrement et avec un plafond.
6. **Les deux parents comptent.** L'inscription dans un établissement privé est un acte non usuel de l'autorité parentale, qui exige l'accord exprès des deux parents **[V]** (réponse du ministère de l'éducation nationale publiée au JO Sénat le 25-4-2024, https://www.senat.fr/questions/base/2024/qSEQ240310668.html : « Parmi les actes non usuels figurent le choix de l'orientation, le redoublement, l'inscription dans un établissement d'enseignement privé : ces actes nécessitent que les deux parents se prononcent et s'accordent expressément. Parmi les actes usuels figure l'inscription dans un établissement public. » ; VERIF R10, §4.4). Le logiciel sait demander, recueillir et tracer cet accord, et sait **bloquer** sans trancher en cas de désaccord. *Nuance* : le guide eduscol du premier degré (2017) range « le changement d'établissement » parmi les actes usuels ; la réponse de 2024, plus récente et explicite, vise le privé ; la qualification reste jurisprudentielle (cas par cas). Les parents séparés sont un cas normal, pas une exception.
7. **Personne ne décide à la place des humains.** Critères affichés, aide au classement, aucun tri automatique ayant un effet juridique (pas de refus calculé, pas de note de candidat par IA) ; chaque décision a un auteur, un motif interne et, si l'établissement le choisit, un motif communiqué.
8. **Tout se prévoit.** Les effectifs prévisionnels de la rentrée (montée, redoublements, départs annoncés, admissions, liste d'attente, taux de réinscription) sont un **écran de pilotage** disponible dès décembre, avec comparaison à la même date l'année précédente. C'est ce qui sert à demander des moyens (public) ou à bâtir le budget (privé), puis à ouvrir les bonnes classes (socle 5).

Ce qui fera la différence avec les logiciels du marché [H — comparaison de mémoire, non vérifiée] : un entonnoir de recrutement intégré (contact → visite → candidature → inscrit) au lieu d'un tableur à côté ; une fratrie inscrite en une seule démarche ; une réinscription en trois gestes depuis le téléphone ; l'accord des deux parents géré nativement ; une liste d'attente qui propose la place libérée au bon candidat en un clic ; un départ qui produit seul le certificat, ferme les services au bon jour et signale les soldes ; des prévisions d'effectifs fiables dès l'hiver.

---

## 2. Évaluation franche de l'existant

Trois générations coexistent : (a) sur `ENT`, la **réinscription ancienne** (migrations 018, 028, 029), le **suivi d'inscription** par étapes (078), les **vœux** (077) et l'**historique d'IBAN** (078) ; (b) sur `ent-socle2-inscriptions`, intégrée dans `ent-integration-lot5` et pas encore dans `ENT`, le **lot S2-3** (migrations 140–145, 17 routes personnel + 13 routes famille ou publiques, 4 écrans Leptos, 10 tests d'intégration) ; (c) l'application **Next.js** en production (préinscription, rendez-vous d'inscription, portes ouvertes).

### 2.1 Ce qui tient (à garder)

| Élément | Pourquoi c'est bon | Preuve [F] |
|---|---|---|
| Préinscription **vérifiée avant tout affichage** : code à 6 chiffres stocké en empreinte, 5 essais, 15 min ; jeton de préinscription en empreinte, 2 h ; même réponse que le compte existe ou non | Applique D-014 et l'anti-énumération ; testé | `lot5:migrations/142` ; `lot5:ent-api/src/tests_inscriptions.rs:67` |
| Limitation de débit sur les routes publiques (démarrer, vérifier, accepter l'invitation) | Bonne base anti-abus | `lot5:ent-api/src/anti_abus.rs:106-123` |
| Le dossier est **la copie propre à l'établissement** de ce que la famille a saisi ; l'origine d'un IBAN repris n'est jamais écrite chez le nouvel établissement | Applique D-013/D-014 à la lettre | `lot5:migrations/141:1-8`, `143:1-10` |
| Contrainte « parents séparés ⇒ e-mail obligatoire » en base | Règle D-014 garantie par la base, pas par l'écran | `lot5:migrations/141` (`ent_inscription_responsable_separes_chk`) |
| Campagne paramétrée (nature, année, niveaux, capacité, pièces, message), une seule ouverte par nature et année | Bonne forme de départ | `lot5:migrations/140` |
| Suivi du dossier avec `visible_famille` (note interne ou message à la famille) | Distinction juste ; à compléter (§15.3) | `lot5:migrations/141` (`ent_inscription_suivi_dossier`) |
| Validation **en une transaction** avec événement `inscription.validee` à charge utile minimale (identifiants seulement) | Le fait et l'événement partent ensemble | `lot5:ent-db/src/inscriptions.rs:1606-1748` |
| Réactions idempotentes à `inscription.validee` (affectation, invitations, message d'accueil), testées en double distribution | Bon usage de l'outbox S2-0 | `lot5:ent-db/src/inscriptions_reactions.rs:26-30`, test `reactions_idempotentes` |
| Invitation du second parent à la validation, jeton en empreinte, rattachement d'une identité existante (D-013) | Applique D-014 | `lot5:migrations/145` |
| Nouveau mandat SEPA à chaque établissement, IBAN repris **masqué**, seulement si la famille a accepté de réutiliser ses informations | Applique D-014 | `lot5:migrations/143`, test `iban_masque_et_nouveau_mandat` |
| Séparation instruction / décision : le secrétariat instruit, la direction décide (403 testé) | Bonne séparation des pouvoirs | `lot5:ent-auth/src/actions.rs:113-119, 586-592` |
| Isolation entre établissements testée (dossiers, jetons, invitations, lots d'import) | Indispensable | test `isolation_etablissements` (`lot5:ent-db/src/tests_inscriptions.rs:1317`) |
| Moteur d'import avec prévisualisation, rejets ligne à ligne, « jamais d'écrasement silencieux » | Base de l'IOP-L1 (socle 9) | `lot5:migrations/144`, `lot5:ent-db/src/imports.rs` |
| Ancienne réinscription : exigences de pièces par campagne, **vanne d'acompte**, confirmation par le parent, vœux classés (077) | Fonctionnalités réelles à ne pas perdre | `migrations/018, 028, 029, 077` ; `ent-db/src/service.rs:9847-10664` |
| Next.js : rendez-vous d'inscription avec double opt-in, réservation provisoire qui expire (2 h), **reconfirmation à J-7**, lien de changement de créneau ; portes ouvertes avec pointage à l'accueil et message de suivi différé | Parcours aboutis, appréciés en production [H] | [F Next] `db/schema-rdv-inscription.ts`, `db/schema-portes-ouvertes.ts`, `app/lib/rdv-inscription-mail.ts` |

### 2.2 Ce qui est à revoir

Classé par gravité. Les points 1 à 4 sont à corriger **avant** toute fusion de S2-3 dans `ENT` (lot I0, §17).

| # | Problème | Conséquence | Preuve [F] | Correction [R] |
|---|---|---|---|---|
| 1 | **Rapprochement silencieux d'un responsable par e-mail à la validation.** Pour chaque responsable déclaré, y compris le **second parent dont l'e-mail n'a jamais été vérifié**, la validation cherche une `ent_personne` de même e-mail dans l'établissement et, si elle existe, **la lie à l'enfant** avec `peut_voir_notes`, `peut_voir_vs`, `peut_justifier_absence` = vrai. Si cette personne a **déjà un compte**, aucune invitation ne part (la réaction ne vise que les personnes sans compte) | Une famille qui inscrit un enfant en indiquant l'adresse d'un parent déjà connu de l'établissement (erreur de frappe, ex-conjoint, malveillance) donne à ce parent, **sans qu'il en soit informé ni qu'il l'ait accepté**, un accès aux notes et à la vie scolaire d'un enfant qui n'est peut-être pas le sien. La direction ne le voit pas à l'écran | `lot5:ent-db/src/inscriptions.rs:1826-1911` (`rattacher_responsables_tx`, « `lower(email) = lower($2)` », puis `INSERT INTO ent_eleve_lien … true,true,true`) ; `lot5:inscriptions_reactions.rs:213-214` | Jamais de rapprochement implicite : la validation présente les **correspondances possibles** (même e-mail, même nom) et une personne les confirme ; un lien vers une personne qui a un compte n'est actif qu'après **confirmation par cette personne** (invitation) ; droits du lien issus du paramétrage, pas codés en dur |
| 2 | **Écritures directes dans les tables des socles 2 et 4** : `INSERT INTO ent_eleve`, `ent_personne`, `ent_foyer`, `ent_eleve_lien` (validation et import), `ent_affectation` (réaction et import) | Contredit la règle D-021 (« le socle 3 n'écrit les personnes que par les commandes du domaine personnes ») et `02_ELEVE.md` §4 ; contourne la future identité versionnée, le numéro d'élève, la détection de doublons, les clés composées ; tout sera à refaire | `lot5:ent-db/src/inscriptions.rs:1784,1845,1877,1894` ; `inscriptions_reactions.rs:144` ; `imports.rs:508,526,545,573,590` | Ports de commandes (§5.3), lot I1 |
| 3 | **Un nouveau foyer à chaque inscription.** Le responsable de rang 1 reçoit systématiquement un nouveau `ent_foyer` (« Foyer NOM »), même s'il est déjà connu avec un autre enfant | Une fratrie inscrite en deux fois = deux foyers pour les mêmes parents ; factures, envois et payeurs dédoublés | `lot5:ent-db/src/inscriptions.rs` (`rattacher_responsables_tx`, `if r.rang == 1 \|\| …`) | Foyer choisi ou confirmé à la validation (socle 4) ; fratrie native (§8.3) |
| 4 | **Doublon d'élève = refus.** Un homonyme exact ou un INE déjà présent bloque la validation par une erreur | Un **ancien élève qui revient** ne peut pas être réinscrit sur sa fiche ; la seule issue est de contourner | `lot5:ent-db/src/inscriptions.rs` (`creer_eleve_tx`, « un élève de même identité … existe déjà ») | Détection de doublons du socle 2 (ELV-01, ELV-12 réintégration) avec choix humain |
| 5 | **Pièces sans fichier** : la famille « déclare » une pièce par son nom de fichier, le secrétariat accepte ou refuse **sans document** | Le cœur d'un dossier d'inscription n'existe pas | `lot5:web/.../inscriptions/famille.rs:438-452`, `lot5:ent-db/src/inscriptions.rs:2488` | Pièces réelles via le socle 7 (lot I3) |
| 6 | **Affectation en classe à la validation** (réaction `d1.affectation_inscription`, identifiant de classe saisi en texte libre ; une saisie invalide devient silencieusement « sans classe ») | En février on ne sait pas encore dans quelle classe sera l'élève : les classes se composent en juin–juillet. Mélange inscription et constitution des classes | `lot5:inscriptions_reactions.rs:144` ; `lot5:web/.../inscriptions/mod.rs:389` (`parse::<Uuid>().ok()`) | La validation crée une inscription annuelle **prévue** (niveau, parcours, régime) ; l'affectation relève de la constitution des classes (socle 5) ; exception : arrivée en cours d'année (classe choisie dans une liste) |
| 7 | **Année et niveau en texte** (`annee_cible '2026-2027'`, niveau `'6EME'`) alors que le référentiel S2-2 a `ent_annee_scolaire` et `ent_niveau` | Deux façons de désigner l'année ; capacités non reliées aux niveaux et MEF réels | `lot5:migrations/140` | `annee_id`, `niveau_id`, `mef_id`, parcours (§8.2) |
| 8 | **Deux responsables au plus** (rang 1 et 2), liens `mere/pere/tuteur/autre`, droits du lien codés en dur | Tuteur + deux parents, famille recomposée, ASE : impossibles à déclarer | `lot5:migrations/141` (`rang IN (1, 2)`) | N déclarations de responsables, vocabulaire du socle 4 |
| 9 | **Deux mécanismes de réinscription** (ancien : `ent_reinscription_campagne`, exigences, acompte ; nouveau : `ent_inscription_campagne` nature `reinscription`) et **les mêmes événements émis par les deux** (`reinscription.campagne_ouverte`, `eleve.reinscription_confirmee`) ; deux « suivis » de noms voisins (`ent_inscription_suivi` 078 et `ent_inscription_suivi_dossier` 141) | Deux campagnes ouvertes la même année possibles ; consommateurs exposés à des doublons | `migrations/018:4`, `lot5:migrations/140:6-7` ; `ent-db/src/service.rs:9873, 10589` | Un seul modèle, reprise de l'ancien (§16) |
| 10 | **Départ non modélisé** : `annoncer_depart_famille` existe pour la réinscription, mais aucune instruction de départ, aucun certificat, aucune fermeture des services | Les départs se font « à la main » dans d'autres écrans | `lot5:ent-db/src/inscriptions.rs:3177` ; `SOCLE2_DOMAINES.md` D3 prévoit `ent_depart`, absente | Départ comme démarche (§7.4) |
| 11 | **Actions sensibles sans confirmation** à l'écran (ouvrir et fermer une campagne, décider, valider) ; **un seul champ « motif »** partagé entre refus de pièce, dossier incomplet, décision et départ | Erreur de manipulation lourde (élève créé, invitations envoyées) ; motif d'une action envoyé avec une autre | `lot5:web/.../inscriptions/mod.rs:102-115, 269, 372-397` | Récapitulatif avant confirmation ; un motif par action |
| 12 | **Routes à jeton famille sans limitation de débit** (`/preinscription/moi`, `dossiers`, `pieces`, `banque`, `mandat`) ; `/preinscription/session` donne un jeton famille à **tout** compte connecté (professeur, élève) | Surface d'abus ; confusion des rôles | `lot5:ent-api/src/anti_abus.rs:52-127` ; `lot5:ent-db/src/inscriptions.rs:2054-2064` | Règles anti-abus ; jeton par session réservé aux responsables et aux majeurs |
| 13 | **Tests verts sans base** : sans `ENT_DATABASE_URL`, les tests d'intégration reviennent sans rien vérifier ; l'isolation du jeton entre établissements annoncée dans un commentaire n'est pas testée en HTTP | Fausse assurance | `lot5:ent-api/src/tests_inscriptions.rs:16-24, 307-310` | Échec explicite si la base manque en CI ; vrai test multi-hôtes |
| 14 | **Pas d'action de lecture** (`inscriptions.lire`) : lire les dossiers exige `gerer` ou `decider` ; l'import exige `imports.appliquer` même pour lire | Un rôle « accueil » ou « comptabilité » ne peut pas consulter sans pouvoir modifier | `lot5:ent-auth/src/actions.rs:113-119` ; `lot5:ent-db/src/inscriptions.rs:517-533` | Catalogue §12 |
| 15 | **Clés étrangères simples** (`eleve_id REFERENCES ent_eleve (id)`), pas composées avec `etablissement_id` ; pas de RLS sur les tables d'inscription | Même faiblesse que celle relevée pour l'élève (`02_ELEVE.md` §2.2) | `lot5:migrations/141, 144, 145` | Clés composées et RLS (§15.1) |
| 16 | **La famille n'est prévenue ni de la décision ni des étapes** : aucun consommateur de `admission.decidee`, `admission.etape_changee`, `admission.deposee` ; une décision ne peut pas être revue (`admis` et `refuse` sont des états sans sortie) ; aucun désistement possible après une admission | La famille découvre la décision par téléphone ; une erreur de décision est irrattrapable | `lot5:ent-db/src/inscriptions.rs:273-278, 3192-3194` ; recherche de consommateurs | Décision versionnée, notifications (§7.1, §13) |
| 17 | **E-mails envoyés à l'intérieur de la transaction des réactions, avant validation de celle-ci** (invitations, accueil) ; échec d'envoi avalé (`let _ =` sur la relance, invitation passée en `non_envoyee` sans trace) | Un message peut partir pour un fait annulé ; un échec passe inaperçu | `lot5:inscriptions_reactions.rs:89, 252-263, 375-382` ; `lot5:inscriptions.rs:1477` | Envois par le socle 8 (file d'envoi, après validation de la transaction, avec reprise) |
| 18 | **Garde d'état manquante** : une campagne fermée peut être rouverte (événement republié) ; un commentaire peut être ajouté à un dossier validé | Incohérences et doublons d'événements | `lot5:inscriptions.rs:808-810, 987, 1370` | Transitions contrôlées (§7) |
| 19 | **Import** : reclassement des lignes hors de la transaction d'application (deux lignes identiques corrigées peuvent créer deux élèves) ; aucun événement pour les personnes et liens créés ; le second responsable devient payeur si le premier est absent ; ligne brute (avec e-mails et téléphones) gardée en JSON sans purge ; pas d'annulation de lot | Doublons, données sans durée de conservation | `lot5:imports.rs:554, 588, 660` ; `migrations/144:32` | Le cadre d'import passe au socle 9 ; le socle 3 ne consomme que des listes d'affectés (§10) |
| 20 | **Acceptation d'invitation** : compte créé hors transaction, rattachement sans garde « personne sans compte » ; code de vérification haché sans secret serveur (10⁶ valeurs, épuisables si la base fuit) | Compte orphelin possible ; résistance faible en cas de fuite de la base | `lot5:inscriptions.rs:145-154, 2914-2958` | Empreinte avec secret serveur (HMAC) ; une transaction |
| 21 | **Élève « préinscrit » pour l'année suivante** : aucun chemin ne le rend actif quand l'année devient courante | Élèves validés en mars invisibles ou mal classés en septembre | `lot5:inscriptions.rs:1782` ; `lot5:inscriptions_reactions.rs:158-170` | Inscription annuelle `prevue` du socle 2, statut dérivé (`02_ELEVE.md` §7.4) |
| 22 | **Numéros de migration en collision avec la conception du socle 2** : `02_ELEVE.md` §7 réserve le bloc 140–149 aux tables élève, déjà pris par S2-3 (140–145) ; 150 (cahier), 170–172 (EDT), 180–182 (évaluations), 190–193 (communication) sont pris sur d'autres branches | Conflits à la fusion | `git ls-tree` des branches au 2026-11-06 | Socle 2 → 160–169 ; socle 3 → 200–219 (Q-I1) |

### 2.3 Ce qui manque

- **Entonnoir de recrutement** : contacts (prospects), demandes d'information, visites, portes ouvertes, origine du contact, taux de transformation. Next.js a les portes ouvertes [F Next], le Rust rien.
- **Rendez-vous et entretiens** d'inscription (Next.js en a un parcours abouti, mais adossé à Google Agenda, ce qui pose une question de souveraineté, §10.3).
- **Commission d'admission**, avis des membres, critères affichés, classement.
- **Liste d'attente** ordonnée, **proposition d'une place libérée** avec délai de réponse.
- **Réponse de la famille** à la décision (accepter, se désister), **conditions** de l'admission (contrat signé, frais payés, passage en classe supérieure).
- **Accord des deux parents** et blocage en cas de désaccord.
- **Fratries** (une démarche pour plusieurs enfants ; réinscrire tous ses enfants d'un geste).
- **Contrat de scolarisation**, frais de dossier, frais d'inscription, acompte relié à la facturation (la vanne d'acompte existe dans l'ancien code seulement).
- **Inscription des élèves affectés** dans le public (Affelnet, SIECLE) : aucun parcours.
- **Non-réinscription décidée par l'établissement** (privé), avec procédure.
- **Départ** complet : certificat de radiation (exeat), fermeture des services, soldes, prêts, destination, « élèves non présentés à la rentrée ».
- **Effectifs prévisionnels** et indicateurs de campagne.
- **Arrivée en cours d'année** (procédure express).
- **Liste des élèves aux mairies** dans les huit jours de la rentrée et état mensuel des mutations [S, §4.5].
- **Modèles de courriers** paramétrables (accusé de réception, pièces manquantes, convocation, décision, confirmation).

### 2.4 Ce que Next.js fait et qu'il ne faut pas perdre (lecture seule)

| Fonction [F Next] | Où | Reprise [R] |
|---|---|---|
| Préinscription publique (identité, naissance, site, niveau, demi-pension, établissement précédent), acceptée ou refusée par le secrétariat | `app/preinscription/page.tsx`, `app/api/public/preinscriptions/route.ts`, `app/api/eleves/preinscriptions/route.ts` | Remplacée par la préinscription vérifiée S2-3. **Défauts à ne pas reproduire** : tout le corps de la requête gardé en JSON (`payload`), refus sans motif, aucun contrôle de doublon à l'acceptation, pas de limitation de débit visible |
| Rendez-vous d'inscription : vérification de l'e-mail, créneaux, rapprochement avec les enfants du foyer, réservation provisoire 2 h, confirmation par lien, **reconfirmation J-7**, changement de créneau (lien valable 14 j), dépôt du PAP avant le rendez-vous | `app/(public-rentree)/rdv-inscription/`, `app/lib/rdv-inscription-*.ts` | Repris dans l'étape « entretien » (§7.1, INS-24) **sans dépendance à Google** par défaut ; le PAP n'est **pas** demandé avant l'admission (minimisation, §15.2) sauf choix explicite de l'établissement |
| Portes ouvertes : créneaux, inscription des visiteurs (public ou accueil), pointage, message de suivi différé | `db/schema-portes-ouvertes.ts`, `app/(admin)/accueil/portes-ouvertes/` | Repris dans l'entonnoir (INS-16 à INS-19) |
| 17 types de pièces d'inscription (fiche, certificat de scolarité, certificat de radiation, livret de famille, PAP, PAI, PPS, GEVA-Sco, jugement…) et classement assisté | `app/lib/inscription-doc-kinds.ts`, `inscription-ingest/route.ts` | Catalogue de types de pièces du logiciel (§8.6) ; le classement assisté par IA relève du socle 30 et ne part jamais chez un tiers non autorisé |
| Certificat de scolarité par modèle | `app/lib/document-templates/catalog.ts` | Socle 7 (libre-service famille) ; le certificat de radiation est produit ici (§7.4) |
| Statuts `preinscrit / inscrit / ancien / archive` | `app/lib/eleves-config.ts:88` | Statut dérivé du socle 2 ; « préinscrit » n'est plus un statut d'élève : un candidat n'est pas un élève (§6) |
| Suppression définitive d'un élève | `EleveDossierClient.tsx:1798` | Non reprise (`02_ELEVE.md` Q-E8) ; un dossier refusé se purge selon la durée de conservation (§15.4) |

---

## 3. Acteurs et ce qu'ils vivent

La méthode du projet demande de partir de ce que chacun perd comme temps, redoute et aimerait. Ce tableau est le cahier des charges réel du socle.

| Acteur | Ce qu'il vit aujourd'hui [H] | Ce qu'il redoute | Ce qui le ravirait | Fréquence |
|---|---|---|---|---|
| **Famille candidate** (privé) | Formulaires papier ou PDF, pièces envoyées trois fois, silence pendant des semaines, incertitude sur la place, dossiers en parallèle dans plusieurs établissements | Perdre la place faute d'avoir vu un message ; ne pas comprendre un refus ; payer des frais pour rien | Savoir à chaque instant où en est le dossier ; tout faire du téléphone (photo d'une pièce) ; inscrire les deux enfants en une fois ; une réponse claire et datée | 1 à 3 fois dans la vie scolaire |
| **Famille d'un élève affecté** (public) | Courrier d'affectation, puis course à l'inscription dans un délai court, en juillet, parfois en vacances | Rater le délai et perdre la place ; ressaisir ce que l'administration sait déjà | Un lien reçu, une fiche pré-remplie, cinq minutes en ligne | 1 fois par cycle |
| **Famille déjà inscrite** (réinscription) | Re-remplir chaque année la même fiche ; payer l'acompte ; choisir les options sans savoir qu'elles sont pleines | Oublier la date limite et perdre la place (privé) | « Rien n'a changé ? Confirmer » en trois gestes, pour tous ses enfants | 1 fois par an |
| **Parent séparé, non déposant** | Découvre l'inscription après coup, ou jamais ; reçoit les factures sans avoir signé | Être mis devant le fait accompli ; ou au contraire être tenu pour responsable sans avoir été consulté | Être informé, donner son accord en ligne, recevoir ses propres envois | 1 à 3 fois |
| **Élève majeur** (lycée, post-bac) | Ses parents font la démarche à sa place, ou il ne peut pas la faire seul | Ne pas être reconnu comme l'interlocuteur | S'inscrire lui-même, désigner un payeur | rare |
| **Secrétariat de scolarité** | 300 dossiers en six semaines, pièces dans les mails, relances au téléphone, ressaisie dans le logiciel de gestion, puis dans SIECLE (privé) | L'erreur de saisie qui suivra l'élève des années ; la famille mécontente au guichet ; le dossier perdu | Une file de travail triée (« à traiter aujourd'hui »), des pièces lisibles et classées, zéro ressaisie, des relances qui partent seules, des courriers générés | quotidienne en saison |
| **Accueil** | Répond aux appels « mon dossier est-il complet ? », reçoit les visiteurs des portes ouvertes | Ne pas savoir répondre ; donner une information confidentielle | Retrouver un dossier en deux secondes et voir l'étape, sans voir les avis internes | quotidienne en saison |
| **Chef d'établissement / direction** | Remplit ses classes à l'intuition, découvre les désistements fin août, prépare la dotation horaire ou le budget sur des effectifs devinés | La classe à 36 ou à 18 ; le recours d'une famille refusée ; l'image de l'établissement | Le remplissage en temps réel par niveau et par option ; la liste d'attente qui se gère seule ; des prévisions fiables en janvier ; des décisions motivées et traçables | hebdomadaire, quotidienne en saison |
| **Membre de commission / enseignant qui fait passer un entretien** | Fiches papier, avis oraux | Que son avis soit lu par la famille hors contexte | Un écran d'entretien simple, ses rendez-vous dans son agenda | quelques jours par an |
| **CPE, vie scolaire** | Découvre à la rentrée un élève à besoins particuliers ou un interne sans fiche | Les « non présentés » de la rentrée sans nouvelles | Être prévenu des arrivées, voir les départs datés | rentrée, au fil de l'eau |
| **Comptabilité** (privé) | Encaisse les acomptes sans savoir à quel dossier ils correspondent ; rembourse à la main les désistements | Les impayés hérités ; les frais non facturés | Frais et acomptes générés par le dossier, rapprochés seuls ; le solde d'un partant calculé | saison et départs |
| **Professeurs principaux** | Apprennent tard qui arrive | — | La liste des arrivées de leur future classe et les informations utiles (pas le dossier) | rentrée |
| **Infirmerie, enseignants référents (socles 34, 35)** | Découvrent PAI et PPS en septembre | Un élève allergique sans protocole le jour 1 | Être prévenus **après admission** qu'un protocole est annoncé, pour l'organiser avant la rentrée | rentrée |
| **Administration publique** (DSDEN, rectorat) | Affecte, reçoit les remontées | — | Remontées justes et à temps (socle 9) | annuelle |
| **Mairie** | Doit connaître les enfants soumis à l'obligation scolaire | Ne pas recevoir les listes des établissements [S] | Liste reçue dans les huit jours de la rentrée, mutations mensuelles | annuelle et mensuelle |
| **Ancien et nouvel établissement** | Échangent certificat de radiation et dossier scolaire par courrier ou par la famille | — | Un certificat fiable et daté, émis sans délai | au départ |
| **Domaines internes** | Facturation, restauration, internat, comptes, documents, référentiel attendent le signal « inscrit » | Un signal en double ou trop tôt | Des événements fiables, sans données personnelles, au bon moment | permanent |

**Cas limites à traiter dès la conception** [R] (chacun renvoie à une règle du §7 ou du §8) : fratrie (jumeaux compris) ; famille recomposée ; enfant confié à l'aide sociale à l'enfance, dont l'adresse ne doit pas être connue (confidentialité renforcée, `02_ELEVE.md` §7.6) ; parent seul titulaire de l'autorité parentale (pièce) ; parents en désaccord ; tuteur ; élève majeur, émancipé, majeur protégé ; enfant venant de l'étranger sans INE ni bulletins ; enfant allophone ; enfant en instruction en famille qui revient ; enfant venant du hors contrat ; ancien élève qui revient ; double inscription (place gardée dans le privé et affectation dans le public) ; désistement le 31 août ; élève inscrit mais jamais venu ; départ en cours de trimestre ; exclusion définitive (procédure disciplinaire, socle 13) ; décès d'un élève (procédure humaine, aucun envoi automatique) ; changement d'établissement à l'intérieur du même tenant (école → collège : ce n'est pas un départ, `02_ELEVE.md` §7.4) ; même enfant dans deux établissements ScolIA (deux élèves sans lien visible, D-013).

**[personas] Cas limites ajoutés** (règles au §17 bis) : mère protégée par une ordonnance qui inscrit en ligne un dimanche soir ; jumeaux à admettre ensemble ; famille sans domicile stable ou en hébergement d'urgence ; famille sans titre de séjour ; famille non francophone ; enfant arrivé de l'étranger sans certificat de radiation possible ; départ annoncé trois jours avant ; arrivée en juillet après la clôture des admissions ; élève en mobilité un an à l'étranger ; décès d'un candidat ou d'un élève pendant une démarche ; validation faite par erreur ; recours contre un refus ; panne le dernier jour de la réinscription ; secrétariat fermé en août ; membre de commission parent d'un candidat ; reprise des dossiers en cours lors d'un changement de logiciel en pleine saison.

---

## 4. Public, privé sous contrat, hors contrat

D-019 impose les deux d'emblée. Le domaine est **le même** ; ce qui change, c'est **qui décide de l'admission**, ce qui est contractuel, ce qui part vers l'extérieur. Tout ce qui suit est à vérifier sur les textes avant codage (D-019) ; les points sourcés sont marqués [S].

### 4.1 Public (second degré)

- **L'établissement n'admet pas, il inscrit.** L'entrée en 6ᵉ suit la sectorisation et, pour les demandes hors secteur, la décision du directeur académique (DASEN) ; l'entrée en seconde générale, technologique ou professionnelle passe par l'affectation **Affelnet** [H, mécanisme connu ; détails NV]. Les dérogations sont instruites par la DSDEN, pas par l'établissement [S : service-public, §19]. L'inscription au collège ou au lycée « dépend de la capacité d'accueil » [S].
- **Conséquence pour le logiciel** : un type de campagne **« inscription des élèves affectés »** sans étape de décision ; les dossiers sont **pré-créés** à partir d'une liste (import SIECLE ou Affelnet, socle 9 ; fichier fourni par l'administration ; ou saisie), la famille reçoit un lien, complète une fiche pré-remplie, choisit options et régime dans l'offre, dépose les pièces ; le secrétariat valide. Un établissement public **ne peut pas refuser** un élève affecté [H] : l'action « refuser » est absente de ce type de campagne ; seule existe « non présenté / désistement constaté ».
- **Téléservice national d'inscription** : certaines académies proposent l'inscription en lycée par un téléservice de Scolarité Services [H, NV]. Si l'établissement l'utilise, ScolIA ne doit **pas** faire doublon : il **importe** ce qui en sort (socle 9) et ne propose que ce qui manque (options locales, pièces, régime). Paramétrage : `inscription_en_ligne = scolia | teleservice_national | aucune`.
- **Réinscription** : dans le public, l'élève **reste** sauf départ ; la réinscription est **tacite** et sert à mettre à jour la fiche (socle 4), choisir options et régime [H]. Le passage dans la classe supérieure est une décision du conseil de classe et de la procédure d'orientation (socles 23, 37).
- **Arrivée en cours d'année** : affectation par la DSDEN (déménagement), certificat de radiation de l'ancien établissement exigé [S, pratique décrite par service-public et justice.fr ; base réglementaire précise NV].
- **Élèves à besoins particuliers** : ULIS (notification MDPH, affectation par le DASEN), UPE2A pour les allophones (évaluation par le CASNAV) [H] : le dossier porte la pièce de décision, l'inscription suit l'affectation.
- **Ne jamais bloquer la scolarisation** pour une pièce manquante dans le public [R ; **[V]** pour le premier degré : C. éduc. D131-3-1, seules l'identité de l'enfant, celle des responsables et le domicile peuvent être exigées, le domicile se prouve « par tous moyens, y compris une attestation sur l'honneur », et sa vérification « ne peut faire obstacle à l'inscription » ; second degré : avis du juriste, VERIF R48] : l'inscription peut être validée **avec pièces en attente**, suivies jusqu'à régularisation.

### 4.2 Privé sous contrat

- **L'établissement admet librement** dans la limite de ses places, mais doit accueillir les enfants « sans distinction d'origine, d'opinions ou de croyances » (contrat d'association, loi Debré) [H, article L442-1 du code de l'éducation, NV]. Le logiciel n'offre **aucun champ** religion, origine ou opinion, et signale ces mots dans les questions personnalisées (§15.5).
- **Relation contractuelle** : **contrat de scolarisation** entre la famille et l'organisme de gestion (OGEC), annuel ou pluriannuel ; règlement financier ; frais de dossier, frais d'inscription ou de réinscription, **acompte** ou **arrhes**, contribution des familles [S : modèles de contrats publiés, §19]. Les délais (date limite de réinscription, date d'information d'une non-réinscription, conditions de remboursement des arrhes) sont **contractuels** et varient d'un établissement à l'autre [S] : ce sont des **paramètres**, jamais des constantes du logiciel.
- **Non-réinscription par l'établissement** : prévue par les contrats « pour une cause réelle et sérieuse », souvent annoncée au plus tard après le dernier conseil de classe [S, modèles de contrats] ; pas de règle légale unique trouvée [NV]. Le logiciel fournit une procédure tracée (motif, entretien proposé, courrier, date) sans présumer du droit applicable.
- **Remontées** : l'établissement transmet ses élèves à SIECLE BEE par un logiciel agréé (socle 9 ; `INTEROPERABILITE_NATIONALE.md`) ; l'INE revient par ce canal [H].
- **Affelnet** : un lycée privé sous contrat peut choisir d'y participer [H, NV] ; si oui, même parcours d'élèves affectés qu'au public, **avec** une étape de confirmation de la famille.
- **Pic de saison** : portes ouvertes d'octobre à février, candidatures de décembre à mars, réinscriptions de février à avril, décisions conditionnées par le conseil du troisième trimestre, désistements en juillet et août [H, pratique courante].

### 4.3 Privé hors contrat

- **Totalement libre** d'admettre et de fixer ses conditions ; pas d'accès à SIECLE ni à ONDE [H] ; souvent un INE absent (`02_ELEVE.md` §8.1).
- **Obligations déclaratives** **[V]** (C. éduc. R131-3 II ; VERIF R47) : les directeurs d'école et chefs d'établissement, « publics ou privés », déclarent **au maire et au directeur académique** les enfants qui fréquentent l'établissement « dans les huit jours qui suivent la rentrée des classes » ; « l'état des mutations est fourni à la fin de chaque mois » ; seuls les enfants soumis à l'obligation scolaire (3 à 16 ans, L131-1) sont visés (§4.5) ; contrôles de l'administration (registre des élèves, à vérifier [NV]).
- Mêmes besoins commerciaux que le privé sous contrat (entonnoir, contrat, frais), souvent avec une équipe plus petite : la réinscription automatisée et les relances y valent encore plus.

### 4.4 L'accord des deux parents

- **Fait vérifié** **[V]** (lu le 2026-11-09, VERIF R10) : une réponse ministérielle publiée par le Sénat (25-4-2024) range l'inscription dans un établissement privé parmi les actes non usuels de l'autorité parentale, qui exigent l'accord exprès des deux parents ; Eduscol classe le passage public/privé parmi les décisions qui exigent cet accord ; un document de l'enseignement catholique cite la **première inscription** dans un établissement privé. En revanche, l'inscription ou la radiation dans le public a pu être traitée comme un acte usuel, pour lequel un parent est présumé agir avec l'accord de l'autre envers un tiers de bonne foi (code civil, art. 372-2), présomption qui tombe si l'autre parent a fait connaître son désaccord [S]. La qualification se fait au cas par cas [S].
- **Conséquence [R]** : le logiciel ne qualifie pas l'acte ; il applique une **règle paramétrée par type de campagne** :
  - `accord_responsables = un_seul | tous_les_titulaires` (préréglage : `tous_les_titulaires` pour l'admission dans le privé ; `un_seul` pour la réinscription et pour le public) ;
  - **désaccord déclaré** par un titulaire de l'autorité parentale (à tout moment, y compris un parent qui n'est pas déposant et qui écrit à l'établissement) ⇒ le dossier passe en **« bloqué : désaccord des responsables »**, aucune décision ni validation possible, message neutre aux deux parents (« l'établissement ne peut pas trancher ; seule une décision commune ou du juge aux affaires familiales le permet » [S]) ; levée avec pièce (accord écrit, décision de justice) ;
  - **un seul titulaire** de l'autorité parentale (décès, retrait, jugement) : déclaré avec pièce ; le socle 4 en garde la trace sur le lien ;
  - parent **sans e-mail** : accord recueilli sur papier, scanné et joint (pièce `accord_responsable`), saisi par le secrétariat avec son nom ;
  - **[personas] responsable à ne pas solliciter** (danger, ordonnance de protection, décision de justice) : aucune demande d'accord, invitation, relance ni copie ne lui est envoyée ; l'accord est « dispensé » par décision motivée de la direction, pièce attendue (INS-57, R-I8, R-I10). Sans cette règle, la demande d'accord révèle l'établissement de l'enfant.
- **Écart avec D-014** : D-014 envoie l'invitation du second parent **à la validation**. Pour une admission dans le privé, l'accord doit exister **avant** l'engagement (contrat, acompte). Proposition : une **demande d'accord** (lien, vérification de l'e-mail par code, sans création de compte) part au dépôt ou à l'admission ; l'invitation à créer le compte reste à la validation, comme D-014. Question Q-I5.

### 4.5 Ce que tous les établissements doivent produire

- **Liste des élèves aux mairies** des communes de résidence dans les huit jours suivant la rentrée, puis **état des mutations** (arrivées, départs) chaque mois [S : guide ministériel des établissements hors contrat ; une réponse ministérielle décrit la même règle pour les établissements publics et privés ; base exacte : article L131-6 du code de l'éducation, NV]. Le socle 3 produit ces listes (par commune de résidence lue au socle 4) ; l'envoi est humain et tracé (INS-41).
- **Certificat de radiation (exeat)** au départ, demandé par le nouvel établissement [S, pratique ; obligation précise NV].
- **Signalement d'un élève qui disparaît** sans destination connue alors qu'il est soumis à l'obligation d'instruction [H, NV] : le socle 3 le détecte (non-présentés, départ sans destination), le socle 12 porte la procédure d'assiduité.

### 4.6 Préréglages par profil d'établissement [R]

Le champ `statut_juridique` proposé par `02_ELEVE.md` §9.1 sert ici aussi.

| Réglage | Public collège | Public lycée | Privé sous contrat | Hors contrat |
|---|---|---|---|---|
| Types de campagne actifs | affectés, réinscription tacite | affectés (Affelnet), réinscription tacite | admission, réinscription explicite, affectés si Affelnet | admission, réinscription explicite |
| Étape de décision | non | non | oui | oui |
| Entonnoir (contacts, portes ouvertes) | désactivé | facultatif (journées d'information) | activé | activé |
| Accord des responsables (admission) | un seul | un seul | tous les titulaires | tous les titulaires |
| Contrat de scolarisation, frais, acompte | non | non | oui | oui |
| Pièces bloquantes pour valider | non (suivies) | non (suivies) | selon paramétrage | selon paramétrage |
| Remontée nationale | import SIECLE | import SIECLE | remontée BEE | aucune |
| Listes mairies | oui | oui (élèves soumis à l'obligation) | oui | oui |

---

## 5. Frontières

### 5.1 Tableau

| Sujet | Socle 2 (élève) | **Socle 3 (ce document)** | Socle 4 (dossier administratif) | Autres |
|---|---|---|---|---|
| Contacts, visites, portes ouvertes | — | **possède** | — | 19 (agenda) plus tard pour les créneaux |
| Campagnes, offre, capacités, pièces demandées | — | **possède** | — | 5 fournit niveaux, MEF, groupes |
| Candidat (enfant pas encore élève) | — | **possède** (dans le dossier, comme déclaration) | — | — |
| Responsables **déclarés** dans un dossier | — | **possède la déclaration** (copie de ce que la famille a saisi, D-014) | **possède** les personnes, foyers, liens, coordonnées qui en résultent | — |
| Rapprochement déclaration ↔ personnes existantes | — | présente les correspondances, recueille le choix humain | **possède** la règle et la commande (§5.3) | — |
| Accord des responsables pour une démarche | — | **possède** (demande, réponse, blocage) | possède l'autorité parentale du lien et ses pièces | — |
| Élève, identité, doublons, réintégration | **possède** (ELV-01, ELV-12) | appelle | — | — |
| Scolarisation (entrée, sortie) et inscription annuelle | **possède les données et les commandes** (ELV-08, 09, 11) | **possède les démarches** qui les déclenchent | — | — |
| Affectation en classe et groupes | — | ne fait que proposer (arrivée en cours d'année) | — | **5** (constitution des classes) |
| Vœux d'options, de régime, de filière pour N+1 | — | **possède** (dans le dossier) | — | 5 (offre), 37 (orientation après la 3ᵉ et la 2de) |
| Décision de passage, orientation | — | consomme | — | **23**, 37 |
| Pièces (fichiers) | — | demande, suit le statut, décide de la conformité | possède les pièces des personnes (jugement) une fois inscrites | **7** stockage, antivirus, conservation |
| Contrat de scolarisation (signature) | — | déclenche, suit | — | **7** (gabarit, signature) |
| Frais de dossier, d'inscription, acompte | — | déclenche (commande), lit le statut de paiement | payeurs | **24**, **25** |
| IBAN, mandat SEPA | — | proposition à la validation (D-014) | — | **25** possède |
| Comptes de connexion, invitations | — | déclenche à la validation | rattache le compte à la personne | **1** |
| Fiche de rentrée (coordonnées vérifiées), santé, autorisations | — | **ouvre** les tâches de rentrée après validation | **4** fiche de rentrée | 34, 18 |
| Départ : démarche, certificat, checklist | — | **possède** | — | chaque domaine signale ses soldes |
| Clôture de la scolarisation | **possède** (ELV-11) | appelle | — | — |
| Effectifs prévisionnels | lit statuts et inscriptions | **possède** le calcul | — | 5 (structure), 24 (budget) lisent |
| Imports nationaux en masse | — | consomme les listes d'élèves affectés | — | **9** possède le cadre d'import |

### 5.2 La règle de D-021 rendue concrète

`SOCLES.md` et D-021 signalent le risque : le socle 3 crée des responsables avant que le socle 4 n'approfondisse leur modèle. Règle [R], plus précise que « appeler les commandes existantes » :

1. **Avant la validation, aucune personne n'est créée.** Les responsables n'existent que comme **déclarations** dans le dossier (`ent_inscription_declaration_responsable`, §8.4). C'est ce que dit D-014 (« copie propre à l'établissement de ce qui figure dans le formulaire »).
2. **À la validation**, le socle 3 remet au socle 4 **une seule structure** (type `DeclarationResponsables` dans `ent-types`, versionnée) et les **choix humains** de rapprochement, par **une seule commande** du socle 4 : `integrer_declaration(eleve_id, declaration, resolutions) -> Resultat { personnes, liens, foyers }`. Le socle 3 ne connaît ni `ent_personne`, ni `ent_foyer`, ni `ent_eleve_lien`.
3. **Aujourd'hui**, cette commande est écrite **dans le code du socle 4** (`ent-db/src/dossier_personnes.rs`) en composant les commandes existantes (`creer_personne`, `lier_personne`, `definir_payeurs` [F `dossier_personnes.rs:1041, 1334, 1672`]) dans des variantes qui acceptent la transaction de l'appelant (`*_tx`). Le socle 4 pourra ensuite changer son modèle interne (adresses historisées, garde, droits fins) **sans changer ce contrat**.
4. Même principe vers le socle 2 : `creer_eleve`, `reintegrer`, `ouvrir_scolarisation`, `enregistrer_inscription_annuelle`, `cloturer_scolarisation` (`02_ELEVE.md` §4). Tant que le lot E4 du socle 2 n'est pas livré, ces commandes sont écrites **dans le code du socle 2** sur le schéma actuel ; le socle 3 ne change pas quand E4 arrive.
5. **Test de garde** [R] : un test lit le code SQL du module `inscriptions` et échoue s'il contient `INSERT`, `UPDATE` ou `DELETE` sur une table qui n'appartient pas au socle 3 (même esprit que la garde SQL statique [F `isolation_sql_statique.rs`]).

### 5.3 Contrat des commandes appelées (ports)

| Port | Propriétaire | Appelé par | Sens |
|---|---|---|---|
| `eleve::rechercher_doublons(identite) -> Vec<Candidat>` | socle 2 | instruction, validation, import | lecture, minimisée |
| `eleve::creer(identite, source=admission, source_ref=dossier_id) -> eleve_id` | socle 2 (ELV-01) | validation | écriture, dans la transaction |
| `eleve::reintegrer(eleve_id, …)` | socle 2 (ELV-12) | validation d'un ancien élève | écriture |
| `eleve::ouvrir_scolarisation(eleve_id, date_entree, motif, origine)` | socle 2 (ELV-08) | validation | écriture |
| `eleve::enregistrer_inscription_annuelle(eleve_id, annee_id, niveau_id, mef_id, etat=prevue, source)` | socle 2 (ELV-09) | validation, réinscription | écriture |
| `eleve::confirmer_inscription_annuelle(inscription_id)` / `annuler(…)` | socle 2 | désistement après validation, rentrée | écriture |
| `eleve::declarer_identifiant(eleve_id, INE, provenance=document_famille, statut=declare)` | socle 2 (ELV-07) | validation | écriture |
| `eleve::cloturer_scolarisation(eleve_id, date, motif, destination)` | socle 2 (ELV-11) | départ | écriture |
| `personnes::integrer_declaration(eleve_id, declaration, resolutions)` | socle 4 | validation | écriture |
| `personnes::correspondances(declaration) -> Vec<Correspondance>` | socle 4 | validation (écran de rapprochement) | lecture, minimisée |
| `personnes::responsables_actifs(eleve_id)` | socle 4 | réinscription, départ, accord | lecture |
| `facturation::demander_paiement(objet, montant, echeance, payeurs)` / `annuler_demande` | socle 24 | frais de dossier, acompte | écriture (commande) |
| `documents::demander_signature(gabarit, signataires, objet)` | socle 7 | contrat | écriture |
| `documents::stocker_piece(…)`, `rattacher_a_eleve(…)` | socle 7 | pièces | écriture |

Chaque port est une **fonction Rust qui prend la transaction en cours** (`&mut TenantTx`) : fait du socle 3, faits des autres socles, journal et événements partent ensemble ou pas du tout.

---

## 6. Concepts et vocabulaire

Le mot « inscription » désigne trois choses différentes dans l'existant. Ce document fixe [R] :

| Mot | Sens | Table |
|---|---|---|
| **Contact** (prospect) | Personne qui s'intéresse à l'établissement pour un enfant, avant toute candidature : demande d'information, visite, portes ouvertes | `ent_inscription_contact` |
| **Évènement d'accueil** | Portes ouvertes, visite, réunion d'information, avec créneaux et pointage | `ent_inscription_accueil`, `_creneau`, `_presence` |
| **Campagne** | Période ouverte par l'établissement pour un type de démarche et une année cible | `ent_inscription_campagne` (existante, étendue) |
| **Type de campagne** | `admission`, `affectes` (élèves affectés par l'administration), `reinscription`, `en_cours_annee` (arrivées en cours d'année, campagne permanente) | colonne `nature` |
| **Offre** | Ce que la campagne propose : niveau × parcours (MEF, option, section, filière) × régime, avec capacité | `ent_inscription_offre` |
| **Démarche familiale** | Ce qu'une famille dépose en une fois : un ou plusieurs enfants (fratrie), une seule déclaration des responsables | `ent_inscription_demarche` |
| **Dossier** | Un enfant dans une campagne : candidature, affectation ou réinscription. **Le candidat n'est pas un élève** tant que le dossier n'est pas validé (sauf réinscription : l'élève existe déjà) | `ent_inscription_dossier` (existante) |
| **Vœu** | Choix classé de la famille dans l'offre (niveau, parcours, régime) | `ent_inscription_voeu` (reprend `ent_voeu_inscription` 077) |
| **Déclaration de responsables** | Ce que la famille déclare des responsables (identité, lien, autorité parentale, coordonnées, foyer) ; devient des personnes au socle 4 à la validation | `ent_inscription_declaration_responsable` |
| **Accord** | Accord (ou désaccord) d'un titulaire de l'autorité parentale sur une démarche | `ent_inscription_accord` |
| **Étape** | Étape **paramétrée** par l'établissement à l'intérieur de l'état « en instruction » (pièces, entretien, commission…) | `ent_inscription_etape_param`, colonne `etape_code` |
| **État** | État **fixe du logiciel** du dossier (§7) | colonne `etat` |
| **Pièce demandée** | Pièce attendue pour un dossier, avec conditions et moment | `ent_inscription_piece` (existante, étendue) |
| **Avis** | Avis d'un membre (entretien, commission), interne | `ent_inscription_avis` |
| **Décision** | Admis, admis sous réserve, liste d'attente, refusé, ajourné ; avec auteur, motif interne, motif communiqué | `ent_inscription_decision` |
| **Proposition de place** | Offre faite à une famille (après admission ou depuis la liste d'attente), avec date limite de réponse | `ent_inscription_proposition_place` |
| **Condition** | Ce qui doit être rempli pour confirmer : contrat signé, frais payés, accord des responsables, décision de passage, pièces bloquantes | `ent_inscription_condition` |
| **Validation** | Le moment où le socle 3 appelle les socles 2 et 4 : l'enfant devient élève (ou l'élève a son inscription N+1) | événement `inscription.validee` |
| **Inscription annuelle** | **Fait du socle 2** (`ent_eleve_inscription`, `02_ELEVE.md` §7.4) : l'élève × l'année × le niveau, à l'état `prevue` puis `confirmee` puis `en_cours` | socle 2 |
| **Départ** | Démarche de sortie : annonce, instruction, certificat, clôture | `ent_inscription_depart` |
| **Prévision d'effectifs** | Calcul, et instantanés datés, des effectifs de la rentrée par niveau, parcours et régime | `ent_inscription_prevision_instantane` |

---

## 7. Cycles de vie

### 7.1 Dossier d'admission

États **fixes** (logiciel) ; étapes **paramétrées** à l'intérieur de `en_instruction`.

```
brouillon ──déposer──▶ deposee ──prendre en charge──▶ en_instruction[etape] ◀──▶ attente_famille
                                                          │
                       ┌──────────────────────────────────┼─────────────────────────┐
                       ▼                                  ▼                         ▼
                 liste_attente[rang] ──place libérée──▶ admis[sous_reserve?] ◀── (décision)      refuse
                       │                                  │
                       │                     famille accepte + conditions remplies
                       │                                  ▼
                       │                              confirme ──valider──▶ validee
                       ▼
     (à tout moment avant validee)  retire (par la famille) · desiste (après admission)
                                    sans_suite (délai de réponse dépassé, campagne close)
                                    bloque_desaccord (désaccord des responsables : suspend tout, réversible)
                                    annule (erreur, par l'établissement, motif)
```

Règles [R] :
- `brouillon` n'est visible que de la famille ; il expire (paramètre, ex. 30 jours sans activité) et se purge.
- `attente_famille` remplace `incomplet` : pièce refusée ou manquante, question posée, accord du second parent attendu. Le dossier revient seul en `en_instruction` quand la famille a répondu.
- **Décision** (`admis`, `admis_sous_reserve`, `liste_attente`, `refuse`, `ajourne`) = une ligne de `ent_inscription_decision`, jamais un simple changement de colonne : auteur, date, motif interne, motif communiqué (facultatif selon paramétrage), modèle de courrier. Une décision peut être **revue** (nouvelle ligne qui remplace la précédente, avec motif), jamais effacée.
- `admis_sous_reserve` : condition explicite (`decision_passage` attendue du conseil de classe de l'établissement d'origine, résultats, pièce) ; levée par la famille (pièce) ou par l'établissement.
- **Capacité** : admettre au-delà de la capacité d'une offre est **possible avec confirmation** (« dépassement assumé : 31/30 ») et tracé ; jamais bloqué en silence, jamais dépassé en silence. L'existant bloque [F test `decision_capacite_et_droits`] : à assouplir (Q-I8).
- **Délai de réponse** de la famille (paramètre de campagne, ex. 8 jours) ; rappel à mi-délai ; à expiration, `sans_suite` **proposé** à l'établissement (jamais automatique) et la place redevient disponible.
- `confirme` exige toutes les **conditions** de la campagne : accord des responsables, contrat signé, frais payés (si exigés), pièces bloquantes acceptées. La liste des conditions restantes est visible des deux côtés.
- `validee` : **seule** transition qui appelle les socles 2 et 4 (§5.3). Pour une admission en juillet pour septembre, l'inscription annuelle est créée à l'état `prevue` ; elle devient `en_cours` à la rentrée ou à la première présence (socle 2).
- **Désistement après validation** (la famille renonce le 25 août) : `desiste` sur le dossier + commande du socle 2 qui **annule** l'inscription annuelle prévue et clôt la scolarisation qui n'a jamais commencé (sans « départ » ni certificat) ; remboursement selon le contrat (commande au socle 24).

### 7.2 Dossier d'élève affecté (public, ou privé participant à Affelnet)

```
importe (ou saisi) ──lien envoyé──▶ a_completer ──famille complète──▶ complete ──valider──▶ validee
        └── non_presente (constaté par l'établissement) · desiste (famille) · reaffecte (l'administration change l'affectation)
```
Pas de décision. La validation peut se faire **avec pièces en attente** (§4.1). Le délai d'inscription fixé par l'administration [H] est un paramètre de campagne ; les relances sont plus serrées (ex. J+2, J+4).

### 7.3 Réinscription

Préparée par l'établissement pour **chaque élève** concerné (scolarisé, pas dans un niveau terminal, pas de départ annoncé), ce qui crée un dossier par élève (existant : préparation idempotente [F test `reinscription_campagne`]).

```
a_confirmer ──famille confirme (+ vœux + conditions)──▶ confirmee ──niveau connu (décision de passage)──▶ validee
     │                                                     └── en_attente_passage (si la décision n'est pas encore rendue)
     ├── non_reinscription_famille (départ annoncé : date, destination, motif) ──▶ ouvre un départ (§7.4)
     ├── non_reinscription_etablissement ★ (privé : procédure, §4.2)          ──▶ ouvre un départ
     └── sans_reponse (date limite dépassée) ── relance ── décision humaine : prolonger, considérer comme départ, valider (tacite)
```

Règles [R] :
- **Mode** (paramètre de campagne) : `tacite` (public : tout élève est réinscrit sauf départ annoncé ; la famille ne fait que mettre à jour et choisir) ; `explicite` (privé : la famille doit confirmer avant la date limite) ; `explicite_avec_acompte` (confirmation effective au paiement de l'acompte : la **vanne d'acompte** existante [F `029`, `continuum.rs:377-424`] devient une condition).
- **Le niveau N+1 n'est pas connu en février.** La famille confirme une **intention** (rester, options souhaitées, régime) ; le niveau est **déduit** de la décision de passage (événement du socle 23) ou de la montée par défaut si l'établissement l'a choisi ; un redoublement ou une réorientation **met à jour** l'inscription annuelle prévue sans nouvelle démarche de la famille, avec information.
- **Réinscription « en 3 gestes »** (web et mobile) : (1) « Mes enfants restent ? » — oui pour tous / détail par enfant ; (2) choix de l'offre (options, régime) avec places restantes ; (3) conditions : vérifier la fiche de rentrée (socle 4, lien), signer l'avenant ou le contrat, payer l'acompte. Tout ce qui n'a pas changé est pré-rempli ; l'accord du second parent n'est demandé que si le paramétrage l'exige.
- **Impayés** : dans le privé, conditionner la réinscription au règlement des sommes dues est une pratique contractuelle [H]. Recommandation : le logiciel **signale** l'impayé au secrétariat et à la direction et peut **retenir la validation** (condition « situation financière examinée »), mais ne **refuse jamais automatiquement** ; la décision est humaine et motivée (Q-I9).
- **Fratrie** : un seul écran pour tous les enfants d'une famille ; une seule signature quand le contrat le permet.

### 7.4 Départ

Un départ est une **démarche**, pas un champ `date_sortie`.

```
annonce (famille, établissement, ou constat : non présenté, exclusion définitive, non-réinscription)
   ──▶ en_preparation (checklist calculée) ──▶ pret ──enregistrer ★──▶ enregistre (socle 2 : scolarisation close)
                                                                         └──▶ certificat émis, services fermés, listes mairie mises à jour
```

Règles [R] :
- **Données** : date effective (dernier jour de présence), motif (codes du logiciel, `02_ELEVE.md` §7.4 : fin de cycle, changement d'établissement, déménagement, exclusion définitive, instruction en famille, départ à l'étranger, fin d'obligation, autre) + motif local, destination (UAI si connu, type, libellé), demande de certificat.
- **Checklist calculée** à partir des domaines (chacun répond à une requête « que reste-t-il pour cet élève à cette date ? ») : solde de facturation et prorata (24), prélèvements à arrêter (25), repas réservés après la date (40), chambre d'internat (41), prêts de matériel et manuels (44), badge, documents à remettre (dernier bulletin, LSU, 23 et 22), comptes à fermer (date de fermeture paramétrée, ex. 30 jours pour que la famille récupère ses documents dans le coffre, 7), groupes et affectations à fermer (5), sorties prévues (42). La checklist **informe**, elle ne bloque pas le départ (on ne retient pas un élève pour une facture).
- **Certificat de radiation (exeat)** : document généré (gabarit du socle 7), daté, signé par le chef d'établissement, remis à la famille et, à sa demande, transmis à l'établissement d'accueil ; mentions à vérifier sur un modèle officiel [NV].
- **Destination inconnue** pour un élève soumis à l'obligation d'instruction : alerte à la direction (« sortie sans solution connue ») et lien vers la procédure du socle 12 [H, NV].
- **Non présentés à la rentrée** : à J+N (paramètre, ex. 3 jours de classe) le socle 3 liste les élèves dont l'inscription annuelle est `prevue` et qui n'ont aucune présence (événements du socle 12) ; actions proposées : contacter la famille, enregistrer un désistement (avant tout début de scolarité), ouvrir un départ, ou signaler.
- **Départ en cours d'année puis retour** : réintégration sur la même fiche (`02_ELEVE.md` ELV-12).
- **Exclusion définitive** : le départ est ouvert par l'événement de la décision disciplinaire (socle 13), avec la date d'effet ; le motif détaillé reste au socle 13.

### 7.5 Arrivée en cours d'année

Campagne permanente `en_cours_annee` (une par année) : dossier ouvert par le secrétariat (ou par la famille via le lien de l'établissement), **procédure courte** : identité, responsables, certificat de radiation de l'ancien établissement (pièce attendue, non bloquante dans le public), niveau, **classe choisie dans une liste** (seul cas où le socle 3 propose une affectation, exécutée par la commande du socle 5), date d'entrée ; validation le jour même ; tâches de rentrée ouvertes (fiche de rentrée, santé, autorisations) ; information du professeur principal et de la vie scolaire (événement).

### 7.6 Contact (prospect)

`nouveau → en_relation (visite, rendez-vous, échange) → converti (dossier déposé) | perdu (motif facultatif) | expire (durée de conservation)`. Un contact se convertit **seul** quand la même adresse, vérifiée, dépose un dossier. Un contact ne reçoit de message que s'il l'a demandé (consentement daté, §15.4).

---

## 8. Modèle de données cible

Notation : pseudo-DDL PostgreSQL, à préciser par le lot qui l'implémente. Toutes les tables portent `etablissement_id UUID NOT NULL`, des clés étrangères **composées** `(etablissement_id, x_id)` vers les tables du même établissement (`02_ELEVE.md` §7), la RLS (§15.1), `cree_le`, `cree_par`. Les codes en `CHECK` sont des codes du logiciel ; les libellés affichés sont paramétrables. **Bloc de migrations proposé : 200–219** (Q-I1). Les tables S2-3 existantes sont **conservées et étendues** (pas de renommage gratuit) ; les ajouts sont additifs.

### 8.1 Campagne

```
ent_inscription_campagne                         -- existante (140), étendue
  id, etablissement_id
  nature             TEXT CHECK (nature IN ('admission','affectes','reinscription','en_cours_annee'))  -- étendu
  libelle, message_famille
  annee_id           UUID NOT NULL → ent_annee_scolaire      -- remplace annee_cible TEXT (reprise §16)
  site_id            UUID NULL → ent_site                    -- campagne d'un site (multi-sites)
  statut             TEXT CHECK (statut IN ('brouillon','planifiee','ouverte','suspendue','fermee','archivee'))
  ouverture_le, cloture_le                                    -- TIMESTAMPTZ (heure de Paris affichée)
  cloture_depot_le   TIMESTAMPTZ NULL                         -- fin des dépôts ≠ fin de la campagne (décisions continuent)
  mode_reinscription TEXT NULL CHECK (mode_reinscription IN ('tacite','explicite','explicite_avec_acompte'))
  date_limite_famille DATE NULL                               -- réinscription : date contractuelle
  delai_reponse_jours SMALLINT NULL                           -- réponse à une proposition de place
  accord_responsables TEXT NOT NULL CHECK (accord_responsables IN ('un_seul','tous_les_titulaires'))
  decision_requise   BOOLEAN NOT NULL                         -- faux pour 'affectes'
  motif_communique   TEXT NOT NULL CHECK (motif_communique IN ('jamais','facultatif','toujours'))
  contrat_gabarit_id UUID NULL                                -- socle 7
  regle_relance      JSONB NOT NULL                           -- {intervalle_jours, max, heures_calmes} dans les bornes du logiciel
  page_publique      BOOLEAN NOT NULL DEFAULT false           -- visible sur la page d'inscription de l'établissement
  version            INTEGER NOT NULL                         -- toute modification après ouverture crée une nouvelle version (journal)
```
Règles : une seule campagne `ouverte` par (établissement, nature, année, site) [F index existant, étendu au site] ; une campagne ouverte ne peut plus changer ses règles **au détriment** des dossiers déposés (ex. ajouter une pièce obligatoire ne s'applique qu'aux nouveaux dossiers, sauf choix explicite « demander aussi aux dossiers en cours », qui relance les familles concernées).

### 8.2 Offre et capacités

```
ent_inscription_offre
  id, etablissement_id, campagne_id
  niveau_id          UUID NOT NULL → ent_niveau
  mef_id             UUID NULL → ent_mef
  parcours_groupe_id UUID NULL → ent_groupe        -- option, section, LV, filière (groupe du référentiel : 02_ELEVE.md §7.4)
  regime             TEXT NULL                     -- code régime (interne…) quand la capacité est par régime
  libelle_public     TEXT NOT NULL
  capacite           INTEGER NULL CHECK (capacite >= 0)   -- NULL = non limitée
  places_reservees_reinscription BOOLEAN NOT NULL DEFAULT true  -- les réinscrits du niveau inférieur consomment la capacité
  ouverte_candidature BOOLEAN NOT NULL DEFAULT true
  conditions_acces   TEXT NULL                     -- texte affiché (ex. « sur dossier »)
```

**Compteur de places** (vue `ent_inscription_offre_occupation`, recalculée à chaque fait, jamais saisie) : `capacite` ; `reserve_reinscription` (élèves du niveau inférieur réinscrits ou en mode tacite, pondérés par la décision de passage quand elle est connue) ; `proposees` (admis en attente de réponse) ; `confirmees` ; `validees` ; `liste_attente` ; `disponibles = capacite − reserve − proposees − confirmees − validees`. Un **dossier compte une seule fois** (son vœu retenu). La capacité d'internat est une offre par régime, indépendante des niveaux si besoin.

`ent_inscription_campagne_niveau` (140) est repris comme offres sans parcours (§16).

### 8.3 Démarche familiale et dossier

```
ent_inscription_demarche                          -- nouvelle : ce qu'une famille dépose en une fois
  id, etablissement_id
  deposant_email     TEXT NOT NULL                -- vérifié (code ou passkey) — D-014
  deposant_user_id   UUID NULL
  deposant_role      TEXT NOT NULL CHECK (deposant_role IN ('responsable','eleve_majeur','etablissement'))
  reutilise_identite BOOLEAN NOT NULL DEFAULT false   -- D-014 (existant sur le dossier, déplacé ici)
  canal              TEXT NOT NULL CHECK (canal IN ('en_ligne','guichet','import','reinscription'))
  contact_id         UUID NULL → ent_inscription_contact

ent_inscription_dossier                            -- existant (141), étendu
  id, etablissement_id, campagne_id, demarche_id (nouveau)
  nature                                           -- reprise de la campagne
  etat               TEXT NOT NULL                 -- §7, CHECK par nature ; remplace statut (reprise §16)
  etape_code         TEXT NULL                     -- étape paramétrée courante (§8.8)
  eleve_id           UUID NULL                     -- réinscription : dès la création ; admission : à la validation
  -- le candidat (déclaration de la famille, copie propre à l'établissement) :
  cand_nom_naissance, cand_nom_usage, cand_prenoms TEXT[], cand_sexe, cand_date_naissance,
  cand_naissance_commune, cand_naissance_pays, cand_ine_declare
  scolarite_actuelle JSONB NOT NULL               -- {etablissement_libelle, uai, type, niveau, annee} — affichée, jamais requêtée
  besoins_annonces   TEXT[] NOT NULL DEFAULT '{}' -- après admission seulement (§15.2) : 'pai','pap','pps','ulis','upe2a', sans détail
  voeu_retenu_id     UUID NULL → ent_inscription_voeu
  rang_attente       INTEGER NULL                  -- liste d'attente, par offre
  priorite_codes     TEXT[] NOT NULL DEFAULT '{}'  -- critères déclarés et vérifiés : fratrie, personnel, ancien_eleve…
  assigne_a          UUID NULL                      -- personne qui instruit
  reference          TEXT NOT NULL                  -- numéro lisible donné à la famille (ex. ADM-2027-0142), unique par établissement
  echeance_famille   DATE NULL                      -- prochaine échéance attendue de la famille (calculée)
  depose_le, valide_le, valide_par, clos_le, motif_cloture
  CHECK (etat <> 'validee' OR eleve_id IS NOT NULL)        -- existant
  UNIQUE partiel (campagne_id, eleve_id) WHERE eleve_id IS NOT NULL AND etat NOT IN ('annule')   -- existant, resserré
```

Les colonnes `eleve_nom`, `eleve_prenom`, `eleve_date_naissance`, `eleve_sexe`, `eleve_ine` (141) deviennent les colonnes `cand_*` (reprise) ; `classe_cible_id` disparaît (§2.2 n° 6) ; `decision_*` passent dans `ent_inscription_decision`.

**Détection de candidatures en double** [R] : même candidat (nom, prénoms, date de naissance) dans deux dossiers ouverts de la même campagne → alerte à l'instruction (souvent deux parents séparés qui déposent chacun) ; fusion de dossiers par le secrétariat (le second devient `annule`, motif « doublon », ses pièces et déclarations sont rattachées au premier).

### 8.4 Déclarations de responsables et accords

```
ent_inscription_declaration_responsable           -- remplace ent_inscription_responsable (rang 1/2)
  id, etablissement_id, demarche_id
  ordre              SMALLINT NOT NULL
  est_deposant       BOOLEAN NOT NULL
  civilite, nom, prenom, email, telephone, adresse JSONB (structurée : voie, CP, commune COG, pays)
  lien_code          TEXT NOT NULL                 -- vocabulaire du socle 4 (mere, pere, parent, tuteur, ase, autre)
  autorite_parentale TEXT NOT NULL CHECK (autorite_parentale IN ('oui','non','inconnue'))
  autorite_exclusive_declaree BOOLEAN NOT NULL DEFAULT false   -- exige une pièce (jugement…)
  foyer_cle          TEXT NOT NULL                 -- regroupe les responsables qui vivent ensemble ('A', 'B'…)
  payeur_propose     BOOLEAN NOT NULL DEFAULT false
  pour_enfants       UUID[] NOT NULL               -- dossiers concernés (fratrie recomposée : un beau-parent pour un seul enfant)
  personne_id        UUID NULL                     -- écrit par le RETOUR de la commande du socle 4 à la validation
  CHECK : email obligatoire si le foyer est différent de celui du déposant (D-014, existant)

ent_inscription_accord
  id, etablissement_id, dossier_id, declaration_id
  objet              TEXT NOT NULL CHECK (objet IN ('candidature','admission','contrat','reinscription','depart'))
  reponse            TEXT NOT NULL CHECK (reponse IN ('attendu','donne','refuse','dispense'))
  moyen              TEXT NOT NULL CHECK (moyen IN ('en_ligne_code','en_ligne_compte','papier','depot_conjoint'))
  piece_id           UUID NULL                     -- accord papier, jugement (dispense)
  repondu_le, saisi_par
```
Une personne qui a déjà un compte et un lien actif avec un **autre enfant** de l'établissement retrouve la démarche dans son espace et répond depuis son compte ; sinon un lien à usage unique (empreinte, durée courte) et un code de vérification suffisent ; **aucun compte n'est créé** avant la validation (D-014).

### 8.5 Vœux

```
ent_inscription_voeu
  id, etablissement_id, dossier_id
  rang SMALLINT NOT NULL, offre_id UUID NOT NULL → ent_inscription_offre
  statut TEXT CHECK (statut IN ('demande','retenu','non_retenu','retire'))
  UNIQUE (dossier_id, rang), UNIQUE (dossier_id, offre_id)
```
Reprend `ent_voeu_inscription` (077, rang 1 à 5) pour les vœux internes ; les vœux d'orientation nationaux (Affelnet, Parcoursup) restent au socle 37.

### 8.6 Pièces

```
ent_inscription_type_piece                         -- catalogue du LOGICIEL
  code TEXT PK            -- livret_famille, acte_naissance, justificatif_domicile, jugement, certificat_radiation,
                          -- bulletins, decision_orientation, notification_affectation, notification_mdph,
                          -- attestation_assurance, photo, piece_identite_responsable, accord_responsable, autre
  categorie_conservation TEXT NOT NULL            -- durée fixée avec le DPO (§15.4)
  rattachement_final TEXT NOT NULL CHECK (rattachement_final IN ('eleve','personne','dossier_seulement'))
  sensible BOOLEAN NOT NULL                         -- jugement, MDPH : accès restreint
  formats_acceptes TEXT[] NOT NULL

ent_inscription_piece_regle                         -- PARAMÉTRAGE par campagne (remplace ent_inscription_campagne_piece 140 et ent_reinscription_exigence_doc 028)
  campagne_id, code_type, libelle, consigne
  moment       TEXT CHECK (moment IN ('depot','admission','confirmation','rentree'))
  bloquante    BOOLEAN                              -- empêche la confirmation ou la validation
  condition    JSONB NULL                           -- ex. {"responsables_foyers_distincts": true}, {"niveau_in": [...]}, {"regime": "interne"}, {"vient_d_un_autre_etablissement": true}

ent_inscription_piece                               -- existant (141), étendu : une pièce attendue d'un dossier (ou d'une démarche)
  id, etablissement_id, dossier_id NULL, demarche_id NULL, code_type
  statut TEXT CHECK (statut IN ('attendue','deposee','acceptee','refusee','dispensee','reprise'))
  document_id UUID NULL                             -- socle 7 : fichier stocké, analysé, URL courte
  motif_refus, dispense_motif, maj_par, maj_le
  reprise_de_piece_id UUID NULL                     -- « déjà fournie pour le frère » (même démarche, même établissement)
```
Règles [R] : fichier stocké par le socle 7 (stockage objet privé, contrôle du type réel, taille, analyse antivirale, URL pré-signées courtes) ; **photo depuis le téléphone** avec recadrage et conversion en PDF côté serveur (moteur de conversion maison, D-005) ; une pièce de niveau « famille » (livret de famille, jugement, justificatif de domicile) se dépose **une fois par démarche** et vaut pour tous les enfants ; à la validation, chaque pièce est **rattachée** à l'élève ou à la personne (socle 7, catégorie et durée de conservation) ou **reste au dossier** puis se purge ; les pièces des dossiers non aboutis se purgent selon la durée validée par le DPO. Pas de pièce de santé avant l'admission (§15.2).

### 8.7 Instruction, avis, décisions, propositions, conditions

```
ent_inscription_etape_param            -- PARAMÉTRAGE : étapes de l'état en_instruction, par campagne
  campagne_id, code, libelle, ordre, obligatoire BOOLEAN, type TEXT CHECK (type IN ('controle_pieces','entretien','test','commission','autre'))

ent_inscription_rdv                     -- entretien, visite individuelle (créneaux propres au socle 3, contrat compatible socle 19)
  id, etablissement_id, dossier_id NULL, contact_id NULL, creneau_id, statut ('provisoire','confirme','reconfirme','annule','honore','absent'),
  provisoire_expire_le, reconfirmation_demandee_le, jeton_gestion_hash, intervenants UUID[]

ent_inscription_avis                    -- interne
  id, dossier_id, auteur_id, contexte ('entretien','commission','pedagogique'), sens ('favorable','reserve','defavorable','sans_avis'), texte (≤ 2000), cree_le

ent_inscription_decision
  id, etablissement_id, dossier_id
  sens          TEXT CHECK (sens IN ('admis','admis_sous_reserve','liste_attente','refuse','ajourne'))
  offre_id      UUID NULL                       -- vœu retenu
  reserve       TEXT NULL                       -- condition de la réserve
  motif_interne TEXT NOT NULL CHECK (length BETWEEN 3 AND 1000)
  motif_communique TEXT NULL
  depassement_capacite BOOLEAN NOT NULL DEFAULT false
  remplace_decision_id UUID NULL
  decide_par, decide_le, courrier_document_id UUID NULL
  commission_id UUID NULL

ent_inscription_commission              -- séance : date, membres, dossiers examinés, procès-verbal (document)
ent_inscription_proposition_place
  id, dossier_id, offre_id, propose_le, repondre_avant, reponse ('acceptee','declinee','expiree'), repondu_le, propose_par
ent_inscription_condition
  id, dossier_id, code ('accord_responsables','contrat_signe','frais_payes','acompte_paye','pieces_bloquantes','passage','situation_financiere_examinee'),
  remplie BOOLEAN, remplie_le, source_ref UUID NULL                -- événement qui l'a remplie (paiement, signature…)
```

### 8.8 Suivi, échanges, relances

`ent_inscription_suivi_dossier` (141) est conservé comme **journal du dossier** (changements d'état, d'étape, pièces, messages, relances), en ajout seul, avec `visible_famille`. Les **messages** avec la famille passent par la messagerie (socle 16) quand elle est branchée, en conversation « à propos du dossier » ; avant cela, par le suivi existant. Les relances sont des tâches planifiées, plafonnées par `regle_relance`, jamais envoyées à une personne dont le lien est suspendu, jamais la nuit ni le dimanche (paramètre du logiciel, heures calmes du socle 8).

### 8.9 Contacts et accueil

```
ent_inscription_contact
  id, etablissement_id, email (vérifié ou non), nom, prenom, telephone NULL
  enfants JSONB NOT NULL               -- [{prenom, annee_naissance, niveau_vise, annee_visee}] : minimum utile
  source  TEXT                          -- site, portes_ouvertes, salon, recommandation, fratrie, autre (paramétrable)
  consentement_messages_le TIMESTAMPTZ NULL, consentement_texte_version TEXT NULL
  etat ('nouveau','en_relation','converti','perdu','expire'), dossier_id NULL, expire_le DATE NOT NULL
ent_inscription_accueil, ent_inscription_accueil_creneau, ent_inscription_accueil_presence   -- portes ouvertes, visites (cf. Next.js)
ent_inscription_contact_trace          -- appels, échanges, visites (auteur, date, résumé court) : interne
```

### 8.10 Départ

```
ent_inscription_depart
  id, etablissement_id, eleve_id, scolarisation_id NULL
  origine TEXT CHECK (origine IN ('famille','etablissement','non_reinscription','non_presente','discipline','fin_cycle'))
  etat ('annonce','en_preparation','pret','enregistre','annule')
  date_effet DATE NOT NULL, motif_code TEXT NOT NULL, motif_local UUID NULL
  destination_uai, destination_libelle, destination_type
  certificat_demande BOOLEAN, certificat_document_id UUID NULL
  checklist JSONB NOT NULL                -- instantané des points remontés par les domaines, avec leur état
  enregistre_par, enregistre_le
```
Les départs de fin d'année (fin de cycle, 3ᵉ → lycée, terminale) sont créés **en lot** à la clôture de l'année (socle 5) à partir des inscriptions annuelles sans suite, avec une seule confirmation.

### 8.11 Prévisions

```
ent_inscription_prevision_instantane      -- photographie datée (chaque semaine pendant la campagne, et à la demande)
  id, etablissement_id, annee_id, pris_le, scenario TEXT ('constate','hypothese_<nom>')
  lignes JSONB NOT NULL                   -- par niveau × parcours × régime : montée, redoublants, départs annoncés,
                                          -- admis, confirmés, validés, attente, capacité, prévision
  hypotheses JSONB NULL                   -- taux de réinscription, taux de désistement… saisis par la direction
```
Les chiffres « en direct » sont calculés par une vue ; les instantanés servent à comparer avec **la même date de l'année précédente** et à expliquer un écart.

### 8.12 Tables reprises ou retirées

| Existant | Devient |
|---|---|
| `ent_inscription_campagne` (140) | conservée, étendue (§8.1) |
| `ent_inscription_campagne_niveau` (140) | offres sans parcours (§8.2), puis retirée |
| `ent_inscription_campagne_piece` (140), `ent_reinscription_exigence_doc` (028) | `ent_inscription_piece_regle` |
| `ent_inscription_dossier` (141) | conservée, étendue (§8.3) |
| `ent_inscription_responsable` (141) | `ent_inscription_declaration_responsable` |
| `ent_inscription_piece` (141) | conservée, étendue (fichier réel) |
| `ent_inscription_suivi_dossier` (141) | conservée (journal du dossier) |
| `ent_preinscription_verification`, `ent_profil_personnel`, `ent_identite_journal` (142) | conservées telles quelles (niveau ENT, D-013) |
| `ent_mandat_sepa_demande` (143) | conservée, **propriété du socle 25** à terme |
| `ent_import_lot`, `ent_import_ligne` (144) | **socle 9** (cadre d'import) ; le socle 3 les consomme |
| `ent_inscription_invitation` (145) | conservée, propriété du socle 1/4 à terme (invitation de compte) |
| `ent_reinscription_campagne` (018) + `acompte_centimes` (029) | reprises en campagnes `reinscription` (mode `explicite_avec_acompte`), puis retirées |
| `ent_eleve_scolarite_annee` (018) statut `reinscription`/`projetee` | inscriptions annuelles `prevue` du socle 2 (`02_ELEVE.md` M5) |
| `ent_voeu_inscription` (077), `ent_filiere` (077) | `ent_inscription_voeu` + offres ; filières → groupes ou MEF du référentiel |
| `ent_inscription_suivi` (078) | reprise dans `etat`/`etape_code` des dossiers, puis retirée |
| `ent_foyer_iban_histo` (078) | socle 25 (à vérifier : chiffrement de la colonne `iban`, Q-I16) |

---

## 9. Ce qui relève du logiciel, ce qui relève du paramétrage

| Sujet | Logiciel (identique partout) | Paramétrage de l'établissement |
|---|---|---|
| Types de campagne, états des dossiers, transitions | ✔ | activation des types selon le profil |
| Étapes d'instruction | types d'étape | liste, ordre, libellés, caractère obligatoire, par campagne |
| Offre | structure (niveau × parcours × régime) | offres, capacités, ouverture aux candidatures, textes publics |
| Dépassement de capacité | toujours possible avec confirmation et trace | — |
| Pièces | catalogue de types, formats, rattachement, conservation | pièces demandées, consignes, moment, conditions, caractère bloquant |
| Questions complémentaires du formulaire | types de champs, **liste noire** (religion, origine, opinions, santé avant admission) | questions propres (rebranchées plus tard sur le moteur de formulaires du socle 18) |
| Accord des responsables | mécanisme, blocage en cas de désaccord | règle par type de campagne (`un_seul` / `tous_les_titulaires`) |
| Décision | sens possibles, motif interne obligatoire | motif communiqué (jamais / facultatif / toujours), modèles de courrier |
| Critères de priorité | liste de codes (fratrie, enfant du personnel, ancien élève, proximité, date de dépôt) **sans pondération automatique** | critères affichés, ordre de présentation |
| Liste d'attente | rang, proposition de place sur confirmation humaine | délai de réponse, rappel |
| Délais | bornes (ex. délai de réponse entre 2 et 30 jours) | valeurs (dates limites contractuelles) |
| Relances | plafond absolu, heures calmes, jamais vers un lien suspendu | intervalle, nombre, canal |
| Frais et acompte | commandes vers la facturation, conditions | montants, échéances, règles de remboursement (texte du contrat), réduction fratrie (socle 24) |
| Contrat de scolarisation | signature et preuve (socle 7) | gabarit, version annuelle, signataires requis |
| Réinscription | modes, intention puis niveau par décision de passage | mode, date limite, condition « situation financière examinée » |
| Impayés à la réinscription | signalement, jamais de refus automatique | retenir ou non la validation |
| Départ | checklist, certificat, clôture via le socle 2 | motifs locaux, délai de fermeture des comptes (dans les bornes), gabarit du certificat |
| Non-présentés | détection | délai J+N |
| Entonnoir | structure, conservation maximale (DPO) | sources, accueils, créneaux, messages de suivi |
| Rendez-vous | créneaux, réservation provisoire, reconfirmation | plages, durée, intervenants, reconfirmation (oui/non, J-N) |
| Page publique | sécurité, accessibilité, aucun traceur tiers | textes, offres affichées, documents (règlement, tarifs) |
| Numéro de dossier | unicité | préfixe et format |
| Rôles | catalogue d'actions (§12), garde-fous | rôles, actions cochées, portées (D-017) |
| Listes mairies | format, regroupement par commune | — |
| Durées de conservation | bornes validées par le DPO | — (aucun réglage local des durées légales) |

---

## 10. Échanges avec l'extérieur

### 10.1 Tableau des flux

| Flux | Sens | Ce qu'il apporte ou emporte | Comment [R] | Qui | Socle porteur |
|---|---|---|---|---|---|
| **Affelnet** (6ᵉ, 2de, voie professionnelle) [H] | entrant | élèves affectés dans l'établissement | import par le cadre du socle 9 → dossiers `affectes` pré-créés ; format exact NV | public ; privé participant | 9 → 3 |
| **SIECLE BEE** (public) | entrant | élèves inscrits, INE, structures | AAF / exports (`INTEROPERABILITE_NATIONALE.md`) → propositions du socle 2 | public | 9 → 2 |
| **Remontée BEE** (privé sous contrat) | sortant | élèves inscrits, scolarité, sorties | après validation et départs ; rejets → propositions | privé sous contrat | 9 |
| **Téléservice national d'inscription** [H, NV] | entrant | inscriptions faites en ligne par les familles | import, pas de doublon de saisie | public (selon académie) | 9 → 3 |
| **ONDE** (premier degré) | entrant/sortant | admissions, radiations [H] | hors périmètre initial (socle 50) ; modèle compatible | écoles | 50 |
| **Parcoursup** | — | post-bac | hors socle (51) | lycées avec BTS, CPGE | 51 |
| **Mairie et DASEN** | sortant | liste des élèves soumis à l'obligation scolaire (3-16 ans), dans les 8 jours de la rentrée, au maire **et** au directeur académique ; mutations à la fin de chaque mois **[V]** (R131-3 II, VERIF R47) ; pour une école : certificat d'inscription délivré par la mairie (pièce) [H] | export PDF ou tableur par commune, envoi humain tracé | tous | 3 |
| **Ancien établissement** | entrant | certificat de radiation, bulletins, décision d'orientation, livret (LSU) | pièces déposées par la famille ; transmission directe entre établissements selon les canaux officiels [H] | tous | 3, 7 |
| **Nouvel établissement** (au départ) | sortant | certificat de radiation ; dossier scolaire à sa demande | document généré ; envoi tracé | tous | 3, 7 |
| **Autre établissement ScolIA** | — | **rien** entre établissements (D-013) ; seule la famille réutilise **ses** informations (pré-remplissage D-014) | profil personnel de la famille, jamais de lecture d'un établissement par un autre | tous | 1, 3 |
| **MDPH, CASNAV, DSDEN** (affectations spécifiques) [H] | entrant | notification, décision d'affectation | pièce sensible du dossier | public surtout | 3, 35 |
| **Banque** (SEPA) | sortant | mandat, prélèvements de frais | socle 25 | privé | 25 |
| **Paiement en ligne** (carte) | sortant | frais de dossier, acompte | prestataire à choisir (Q5 de `SOCLE2_DOMAINES.md`) | privé | 25 |
| **Signature électronique** | — | contrat | socle 7 (niveau de signature à décider [H]) | privé | 7 |
| **E-mail, SMS, push** | sortant | codes, accusés, relances, décisions | socle 8 (SMS : OVH, D-004) ; aucun contenu sensible dans un SMS | tous | 8 |
| **Site web de l'établissement** | sortant | lien vers la page publique d'inscription et le formulaire de contact | lien ou bouton ; **pas** d'iframe tierce par défaut (sécurité, CSP) | tous | 3 |
| **Agenda de la direction** | sortant | rendez-vous d'entretien | **iCal** (lecture) ; synchronisation Google ou Microsoft seulement en option, désactivée par défaut (souveraineté, §10.3) | privé surtout | 3, 19 |

### 10.2 Règle commune

Rien de ce qui vient de l'extérieur n'écrit en silence : une liste d'affectés crée des **dossiers** à compléter, pas des élèves ; un INE donné par la famille est **déclaré** (`02_ELEVE.md` §8.1) ; une pièce venue d'un autre établissement est **vérifiée** par une personne.

### 10.3 Rendez-vous : ne pas dépendre de Google

Next.js adosse les rendez-vous d'inscription à Google Agenda, un agenda par direction, avec un jeton OAuth conservé en base [F Next `db/schema-rdv-inscription.ts`]. Ce choix fait de Google un sous-traitant de fait pour des données de familles, et contredit l'orientation « pas d'API cloud comme solution finale par défaut » de `CLAUDE.md`. Recommandation [R] : créneaux et réservations **dans ScolIA** (réutiliser le modèle de créneaux de la branche `ent-socle2-communication`, migration 192 [F], qui préfigure le socle 19), **export iCal** pour que la direction les voie dans son agenda, synchronisation avec un agenda tiers **en option**, désactivée par défaut, avec mention au registre des traitements. Question Q-I12.

---

## 11. Actions métier

Notation des acteurs comme `02_ELEVE.md` §11 : ADF administratif (secrétariat), ACC accueil, DIR direction, CPT comptabilité, MBR membre de commission ou enseignant qui fait passer un entretien, PAR responsable, ELV élève (majeur), CTC contact, SYS système. ★ = action sensible (récapitulatif avant / après, confirmation ; ré-authentification quand elle sera branchée). Les `INS-01` à `INS-15` de `SOCLE2_DOMAINES.md` sont conservés, précisés ; les suivants sont nouveaux.

### 11.1 Campagnes et offre

| ID | Action | Acteur · droit | Règles clés | Événements |
|---|---|---|---|---|
| INS-01 | Créer, planifier, ouvrir une campagne (tout type) | DIR · `inscriptions.campagnes.gerer` ★ à l'ouverture | aperçu avant ouverture (offres, pièces, étapes, délais, textes, frais) ; une ouverte par nature/année/site | `admission.campagne_ouverte` |
| INS-06 | Préparer une campagne de réinscription | DIR · idem | crée un dossier par élève concerné (idempotent [F]) ; aperçu du nombre et des exclusions (niveau terminal, départ annoncé) | `reinscription.campagne_ouverte` [F] |
| INS-16 | Modifier une campagne ouverte | DIR · idem ★ | nouvelle version ; changements défavorables non rétroactifs sauf choix explicite (relance des familles) | `inscription.campagne_modifiee` |
| INS-17 | Suspendre, fermer, archiver | DIR · idem ★ | fermeture des dépôts ≠ fin des décisions | `admission.campagne_fermee` [F] |
| INS-18 | Définir l'offre et les capacités | DIR · idem | lien au référentiel ; capacité modifiable en cours (trace) | `inscription.offre_modifiee` |
| INS-19 | Copier la campagne de l'an dernier | DIR · idem | dates décalées d'un an, pièces, étapes, textes ; tout reste à relire avant ouverture | — |

### 11.2 Entonnoir

| ID | Action | Acteur · droit | Règles clés | Événements |
|---|---|---|---|---|
| INS-20 | Demander des informations (formulaire de contact public) | CTC (public, limité) | minimum de champs ; consentement aux messages explicite et facultatif ; anti-abus | `inscription.contact_cree` |
| INS-21 | S'inscrire à une porte ouverte ou une visite | CTC | créneau, places, rappel la veille si consenti | `inscription.accueil_reserve` |
| INS-22 | Pointer la présence, enregistrer un contact à l'accueil | ACC · `inscriptions.contacts.gerer` | tablette ou téléphone (QR) | `inscription.accueil_presence` |
| INS-23 | Suivre un contact (trace d'échange, relance, conversion, perte) | ADF, DIR · idem | traces internes courtes | `inscription.contact_converti` |
| INS-24 | Proposer et gérer des rendez-vous (visite, entretien) | ADF · idem ; PAR, CTC (réservation) | réservation provisoire qui expire, confirmation, reconfirmation J-N, changement de créneau par lien (cf. Next.js) | `inscription.rdv_*` |
| INS-53 [personas] | Organiser une **session de tests d'admission** (convocations, salles, surveillants, présence, résultats saisis par les correcteurs) | ADF · `inscriptions.instruire` ; MBR · `inscriptions.avis` | les résultats sont des **avis** internes, jamais une décision ; aménagements annoncés (sans détail médical) pris en compte ; convocation dans la langue de la famille si disponible (R-I4) | `inscription.session_tests_*` |
| INS-54 [personas] | Recueillir un **avis externe** par lien à usage unique (club sportif, conservatoire, section internationale) | ADF · `inscriptions.instruire` ; tiers sans compte | lien unique, 7 jours ; le tiers ne voit que nom, prénom, niveau visé et la question posée ; la famille est informée qu'un avis extérieur est demandé ; la réponse devient un avis interne (R-I23) | `inscription.avis_externe_*` |

### 11.3 Candidature et instruction

| ID | Action | Acteur · droit | Règles clés | Événements |
|---|---|---|---|---|
| INS-02 | Déposer une démarche (un ou plusieurs enfants) | PAR, ELV majeur (e-mail vérifié avant tout affichage, D-014) | brouillon enregistré à chaque étape ; « utiliser mes informations ? » ; fratrie ; vœux classés ; pièces du moment `depot` ; récapitulatif ; accusé de réception avec numéro | `admission.deposee` [F] |
| INS-25 | Retirer une candidature | PAR (déposant ou titulaire de l'autorité parentale) | avant décision, sans motif obligatoire | `admission.retiree` |
| INS-26 | Saisir une démarche au guichet | ADF · `inscriptions.instruire` | même parcours, canal `guichet` ; e-mail facultatif (parent sans e-mail) | `admission.deposee` [F] |
| INS-03 | Instruire : prendre en charge, passer d'étape, assigner | ADF · `inscriptions.instruire` | étapes paramétrées ; file « à traiter aujourd'hui » | `admission.etape_changee` [F] |
| INS-27 | Contrôler une pièce (accepter, refuser avec motif, dispenser) | ADF · idem | le refus renvoie le dossier en `attente_famille` avec le motif visible | `inscription.piece_controlee` |
| INS-15 | Relancer (pièces, réponse, accord) | SYS planifié ; ADF | plafond, heures calmes ; jamais vers un lien suspendu | `admission.relance_envoyee` [F] |
| INS-28 | Demander l'accord d'un responsable, enregistrer un accord papier, déclarer un désaccord | SYS ; ADF ; PAR | §4.4 ; désaccord ⇒ `bloque_desaccord` | `inscription.accord_*`, `inscription.bloque_desaccord` |
| INS-29 | Échanger avec la famille sur le dossier | ADF ↔ PAR | messages « à propos du dossier » (socle 16 quand disponible) | — |
| INS-30 | Détecter et fusionner des candidatures en double | ADF · idem | même candidat dans deux dossiers ouverts | `inscription.dossiers_fusionnes` |
| INS-55 [personas] | **Modifier une démarche déposée** (vœux, informations, pièce oubliée) tant qu'aucune décision n'est rendue | PAR (déposant), ELV majeur | chaque modification est versionnée et visible du secrétariat (« modifié après dépôt ») ; la date de dépôt est conservée ; vœux gelés à l'ouverture de la commission (paramètre) | `admission.modifiee` |
| INS-56 [personas] | **Changer l'e-mail du déposant**, ou transférer la démarche à l'autre responsable | PAR ; ADF au guichet (identité vérifiée) | nouvelle adresse vérifiée par code ; l'ancienne est prévenue ; transfert seulement vers un responsable déclaré titulaire de l'autorité, avec son accord ; jamais vers une personne « à ne pas solliciter » | `inscription.deposant_change` |
| INS-57 [personas] | **Marquer un responsable « à ne pas solliciter »** ★ (danger, ordonnance de protection, décision de justice) | DIR · `inscriptions.decider` | supprime demandes d'accord, invitations, relances et copies vers cette personne, y compris celles déjà programmées ; motif scellé ; pièce attendue ; règle d'accord recalculée (« dispensé : décision de la direction ») ; le déposant voit seulement « la direction prend contact avec vous » ; transmis au socle 4 à la validation (R-I8) | `inscription.sollicitation_suspendue` (sans motif ni personne) |
| INS-58 [personas] | Déclarer, **dès la préinscription**, une situation qui exige la confidentialité (adresse à protéger, personne à ne pas contacter) | PAR, ELV majeur | case dédiée avec un texte court, scellé ; dossier marqué pour la direction **avant** toute demande d'accord ; l'adresse marquée n'est jamais transmise en clair au socle 4 (R-I9) | `inscription.confidentialite_demandee` |
| INS-59 [personas] | **Numériser un dossier papier** reçu par courrier (lot de pages → pièces typées) | ADF · `inscriptions.instruire` | découpage assisté (pages, types), aperçu, contrôle du type réel (socle 7) ; sort de l'original selon le paramétrage (rendu, archivé, détruit) | `inscription.piece_controlee` |
| INS-60 [personas] | **Préparer la démarche par l'élève**, vérifiée et déposée par le responsable | ELV (âge minimal paramétré, désactivé par défaut) ; PAR | l'élève ne voit ni ne remplit les champs financiers ni les coordonnées des responsables ; seul le responsable dépose (R-I22) | — |
| INS-61 [personas] | Demander un **interprète** ou une traduction (rendez-vous, courrier de décision) | PAR ; ADF | marque sur le dossier et le rendez-vous ; rappel « pas d'enfant interprète pour une décision » ; la langue est gardée pour les envois (socle 8) | — |

### 11.4 Évaluation et décision

| ID | Action | Acteur · droit | Règles clés | Événements |
|---|---|---|---|---|
| INS-31 | Donner un avis (entretien, commission) | MBR · `inscriptions.avis` (dossiers confiés) | interne ; rappel « rédigez comme si la famille pouvait le lire » (§15.3) | — |
| INS-32 | Tenir une commission | DIR · `inscriptions.decider` | liste des dossiers, critères affichés, avis, places restantes en direct ; décisions en lot avec récapitulatif | `inscription.commission_tenue` |
| INS-04 | Décider (admis, sous réserve, attente, refus, ajourné) | DIR · `inscriptions.decider` ★ | motif interne obligatoire ; motif communiqué selon paramétrage ; courrier généré ; dépassement de capacité confirmé | `admission.decidee` [F] |
| INS-33 | Revoir une décision | DIR · idem ★ | nouvelle décision qui remplace, motif | `admission.decidee` [F] |
| INS-34 | Gérer la liste d'attente (rang, proposer une place libérée) | DIR · idem | le système **désigne** le suivant ; une personne confirme l'envoi | `inscription.place_proposee` |
| INS-62 [personas] | **Programmer la publication des décisions** (une date et une heure pour une commission ou une offre) | DIR · `inscriptions.decider` ★ | décisions prises mais invisibles jusqu'à l'heure ; envois groupés ; confirmation demandée pour une heure entre 20 h et 8 h, un dimanche ou un jour férié (R-I7) | `admission.decisions_publiees` |
| INS-63 [personas] | **Se déporter** d'un dossier (conflit d'intérêts) | MBR, DIR | détection proposée : le membre est responsable, contact ou personnel apparenté déclaré du candidat (socles 4 et 6) ; il ne voit plus le dossier ni ses avis ; tracé (R-I13) | `inscription.deport` |
| INS-64 [personas] | **Lier les décisions** de plusieurs dossiers (jumeaux, fratrie : « ensemble ou pas du tout ») | DIR ; PAR (souhait déclaré au dépôt) | la commission voit le groupe ; décisions différentes possibles seulement avec confirmation explicite (R-I14) | — |
| INS-65 [personas] | **Proposer une offre alternative** (« pas de place en bilangue, une place en classe standard ») | DIR · `inscriptions.decider` | décision `admis` sur une autre offre ; la famille accepte ou décline ; le vœu initial peut rester en liste d'attente | `admission.decidee` |
| INS-66 [personas] | **Enregistrer et instruire un recours** contre une décision (refus, liste d'attente, non-réinscription) | ADF, DIR · `inscriptions.decider` | date de réception, auteur, objet ; échéance de réponse paramétrée ; réponse motivée et courrier ; nouvelle décision éventuelle (INS-33) ; public : orientation vers l'autorité compétente [NV, à vérifier] (R-I12) | `inscription.recours_recu` / `_repondu` |
| INS-67 [personas] | Produire le **dossier de justification** d'une décision (critères affichés, avis, pièces, chronologie, auteurs) | DIR · `inscriptions.avis.lire` + `inscriptions.exporter` | pour un recours, le Défenseur des droits, un contrôle ; filigrane ; journalisé | `export.realise` |

### 11.5 Confirmation et validation

| ID | Action | Acteur · droit | Règles clés | Événements |
|---|---|---|---|---|
| INS-35 | Répondre à une proposition (accepter, décliner) | PAR | dans le délai ; accepter ouvre les conditions | `inscription.reponse_famille` |
| INS-36 | Signer le contrat de scolarisation | PAR (signataires requis) | gabarit de la campagne, socle 7 | (socle 7) `document.signe` → condition remplie |
| INS-37 | Payer les frais ou l'acompte | PAR, payeur | commande au socle 24 ; paiement au socle 25 | (socle 25) `paiement.recu` → condition remplie |
| INS-05 | **Valider l'inscription** | ADF ou DIR · `inscriptions.valider` ★ | conditions remplies (ou levées avec motif) ; écran de **rapprochement** de l'élève (doublons, ancien élève) et des responsables (correspondances) ; appels des ports §5.3 dans **une** transaction | `inscription.validee` [F] |
| INS-13 | Inviter les responsables à créer ou rattacher leur compte | SYS sur `inscription.validee` [F] | D-014 ; seulement les personnes dont l'adresse est confirmée par l'accord ou l'invitation | `eleve.compte_invite` |
| INS-14 | Reprendre l'IBAN masqué et demander un nouveau mandat | PAR | D-014 [F] | (socle 25) |
| INS-38 | Ouvrir les tâches de rentrée | SYS sur `inscription.validee` | fiche de rentrée (socle 4), fiche santé (34), autorisations (18), photo (2) apparaissent dans l'à-faire de la famille | — |
| INS-39 | Se désister après admission ou validation | PAR ; ADF (constat) | §7.1 ; remboursement selon contrat (commande) | `inscription.desistement` |
| INS-68 [personas] | **Annuler une validation erronée** ★ (mauvais enfant, mauvaise année, doublon) | DIR · `inscriptions.valider` | délai court paramétré (défaut 72 h) ; refusée si un autre socle a produit des données depuis (liste montrée : utiliser alors le désistement ou ELV-02) ; appelle les commandes inverses des socles 2 et 4 ; invitations révoquées ; facture annulée par commande (R-I11) | `inscription.validation_annulee` |
| INS-69 [personas] | Délivrer une **attestation d'inscription pour l'année à venir** | PAR (libre-service) ; ADF | dès la validation ; année et niveau prévu (« sous réserve de la décision de passage » si inconnu) ; vérifiable par code (socle 7) ; bilingue possible (R-I19) | `document.emis` |
| INS-70 [personas] | Demander un **changement de vœu ou d'option après la validation**, avant la rentrée | PAR (titulaire) | selon places et date limite (socle 5) ; décision humaine ; frais recalculés par commande ; refus motivé | `inscription.voeu_change` |

### 11.6 Réinscription

| ID | Action | Acteur · droit | Règles clés | Événements |
|---|---|---|---|---|
| INS-07 | Confirmer la réinscription (3 gestes, tous ses enfants) | PAR · lien actif (autorité parentale) ; ELV majeur | mode de campagne ; vœux dans l'offre ; conditions | `eleve.reinscription_confirmee` [F] |
| INS-08 | Annoncer un départ / ne pas se réinscrire | PAR · lien actif | date, destination, motif ; ouvre un départ | `scolarite.depart_annonce` |
| INS-40 | Décider une non-réinscription (établissement) | DIR · `inscriptions.decider` ★ | motif, entretien proposé, courrier, date (paramètre contractuel) | `reinscription.non_reinscription_decidee` |
| INS-09 | Appliquer la décision de passage au dossier de réinscription | SYS sur événement du socle 23 ; ADF | niveau, redoublement, réorientation ; famille informée | `inscription.niveau_determine` |
| INS-41 | Traiter les sans-réponse à la date limite | ADF, DIR | prolonger, valider (tacite), ouvrir un départ | — |
| INS-10, INS-11 | Changer de régime, d'options (en cours d'année) | — | **déplacées au socle 2** (ELV-10) ; le socle 3 ne les porte que comme vœux N+1 | — |
| INS-71 [personas] | **Réserver la place d'un élève en mobilité** (année à l'étranger, `02_ELEVE.md` ELV-35) | ADF · `inscriptions.instruire` | dossier de réinscription `confirmee` sans conditions de rentrée ; relance pour confirmer le retour à une date ; place comptée dans les effectifs prévisionnels | `eleve.reinscription_confirmee` |
| INS-72 [personas] | **Relancer de façon ciblée** (pas ouvert ; commencé sans finir ; un seul parent a répondu ; conditions manquantes) | ADF | compteurs par cible ; canal choisi ; plafond et heures calmes inchangés | `admission.relance_envoyee` [F] |

### 11.7 Départ et rentrée

| ID | Action | Acteur · droit | Règles clés | Événements |
|---|---|---|---|---|
| INS-12 | Préparer et enregistrer un départ | ADF · `inscriptions.departs.gerer` ; clôture : `eleves.sortie` (socle 2) ★ | checklist ; certificat ; clôture via ELV-11 | `scolarite.depart_enregistre` ; (socle 2) `scolarite.closed` |
| INS-42 | Émettre un certificat de radiation | ADF · idem | gabarit, signature du chef d'établissement | `document.emis` |
| INS-43 | Traiter les non-présentés de la rentrée | ADF, CPE · `inscriptions.departs.gerer` | liste J+N ; contacter, désistement, départ, signalement | — |
| INS-44 | Enregistrer une arrivée en cours d'année | ADF · `inscriptions.instruire` + `inscriptions.valider` | procédure courte (§7.5) ; classe choisie dans une liste | `inscription.validee` |
| INS-45 | Produire les listes mairies (rentrée, mutations mensuelles) | ADF · `inscriptions.exporter` | par commune de résidence (socle 4) ; envoi humain tracé | `export.realise` |
| INS-46 | Créer les départs de fin d'année en lot | ADF · `inscriptions.departs.gerer` ★ | fin de cycle, élèves sans inscription N+1 | `scolarite.depart_enregistre` |
| INS-73 [personas] | Émettre les **certificats de radiation en lot** | ADF · `inscriptions.departs.gerer` ★ | signature du chef d'établissement en une fois ; dépôt dans le coffre de chaque famille (socle 7) ; transmission sur demande (INS-74) | `document.emis` |
| INS-74 [personas] | **Transmettre le dossier scolaire** à l'établissement d'accueil | ADF · `inscriptions.departs.gerer` | à la demande de la famille ou de l'établissement d'accueil ; contenu choisi et listé (bulletins, livret, décisions d'orientation, certificat) ; **jamais** santé ni suivi social hors de leurs circuits ; famille informée ; bordereau d'envoi conservé (R-I20) | `scolarite.dossier_transmis` |
| INS-75 [personas] | **Répondre à une demande** de dossier ou de certificat reçue d'un autre établissement | ADF | vérification de la demande (UAI, interlocuteur, élève effectivement parti vers cet établissement) ; réponse tracée ; refus motivé si la demande ne correspond à rien | `scolarite.dossier_transmis` |
| INS-76 [personas] | Enregistrer un **départ vers l'instruction en famille ou l'étranger** avec ses références | ADF · `inscriptions.departs.gerer` | référence d'autorisation exigée pour l'instruction en famille **[V]** (C. éduc. L131-5 : autorisation, motifs limitatifs ; VERIF R49), référence de déclaration pour un départ à l'étranger ; sans référence : alerte direction et lien vers la procédure du socle 12 | `scolarite.depart_enregistre` |
| INS-77 [personas] | **Signaler à l'administration** les places vacantes et les affectés non inscrits | ADF, DIR · `inscriptions.exporter` | public, Affelnet ; liste datée ; envoi humain tracé [NV : circuit exact] | `export.realise` |
| INS-78 [personas] | Suivre la **complétude de rentrée** par classe (fiche de rentrée, santé, assurance, photo, autorisations, comptes activés) | ADF ; PP (lecture de sa classe) | « fait / pas fait » seulement, jamais le contenu (santé surtout) ; relance groupée | — |

### 11.8 Pilotage, paramétrage, imports

| ID | Action | Acteur · droit | Règles clés | Événements |
|---|---|---|---|---|
| INS-47 | Consulter le tableau de bord de campagne (entonnoir, places, délais) | DIR, ADF · `inscriptions.lire` | chiffres explicables (clic → liste) | — |
| INS-48 | Consulter et simuler les effectifs prévisionnels | DIR · `inscriptions.effectifs.lire` | hypothèses nommées ; comparaison N-1 à la même date | — |
| INS-49 | Exporter (listes, tableaux) | `inscriptions.exporter` | colonnes limitées aux droits ; journalisé | `export.realise` |
| INS-50 | Paramétrer (profil, types de campagne actifs, modèles de courriers, motifs, critères, page publique) | ADM · `inscriptions.parametrer` | dans les bornes du logiciel | `parametrage.inscriptions_modifie` |
| INS-51 | Importer une liste d'élèves affectés | ADF · `imports.appliquer` [F] | cadre du socle 9 ; crée des dossiers `affectes`, jamais des élèves | `import.lot_applique` [F] |
| INS-52 | Purger les données arrivées à échéance | SYS | durées DPO ; journal de purge | `inscription.purge` |
| INS-79 [personas] | **Prolonger ou décaler une échéance** pour toute une campagne ou une sélection (panne, intempérie, grève postale) ★ | DIR · `inscriptions.campagnes.gerer` | familles prévenues ; relances recalées ; motif public court ; jamais rétroactif au détriment d'une famille | `inscription.echeance_prolongee` |
| INS-80 [personas] | Déclarer une **fermeture du secrétariat** (vacances) : délais suspendus, message aux familles | DIR, ADF · `inscriptions.campagnes.gerer` | délais de réponse des familles et échéances internes recalculés (R-I6) ; message sur « Mes démarches » ; urgences redirigées vers un contact | `inscription.fermeture_declaree` |
| INS-81 [personas] | **Reprendre des dossiers en cours** depuis un ancien logiciel ou un tableur (mise en service en pleine saison) | ADF · `imports.appliquer` | dossiers créés dans leur état réel (déposé, admis, sur liste d'attente) avec leur date d'origine ; pièces jointes ; **aucun** envoi rétroactif (R-I26) | `import.lot_applique` [F] |
| INS-82 [personas] | **Vérifier une priorité déclarée** (fratrie, enfant du personnel, ancien élève) | SYS propose ; ADF confirme | lecture minimisée aux socles 2, 4 (port `fratrie`), 6 ; réponse « vérifiée / non trouvée », sans exposer d'autre donnée | — |
| INS-83 [personas] | Signaler une **suspicion de fraude** (justificatif de domicile, bulletin falsifié) | ADF → DIR | note interne scellée ; aucune conséquence automatique ; décision humaine (R-I21) | — |
| INS-84 [personas] | Imprimer la **fiche d'inscription** d'un candidat ou d'un élève (dossier complet, une page par section) | ADF · `inscriptions.lire` | trame prête (R-I27) ; avis internes exclus sauf droit `inscriptions.avis.lire` ; filigrane — présente dans Next.js et sur le marché (D-024) | — |

---

## 12. Droits (catalogue `ent_auth::Action`)

### 12.1 Existant [F]

`inscriptions.gerer` (instruire, créer les campagnes ; Admin, Direction, Administratif), `inscriptions.decider` (décider et valider ; Admin, Direction), `imports.appliquer` (Admin, Direction, Administratif), toutes à portée « établissement » seulement [F `lot5:ent-auth/src/actions.rs:113-119, 363-366, 586-592`]. Aucune action de lecture. Les familles sont hors rôles libres : leurs droits viennent de la vérification de l'e-mail (jeton de préinscription) et du lien actif (réinscription, départ) [F `lot5:ent-db/src/inscriptions.rs:3035-3052`].

### 12.2 Actions proposées [R]

| Action | Libellé | Portées admises | Modèle par défaut (= comportement actuel quand il existe) |
|---|---|---|---|
| `inscriptions.lire` | Consulter campagnes, dossiers (hors avis), tableaux de bord | établissement, sites désignés | Admin, Direction, Administratif ; **Accueil** (modèle nouveau) ; Compta (lecture des conditions financières) |
| `inscriptions.contacts.gerer` | Contacts, portes ouvertes, rendez-vous | établissement, sites désignés | Admin, Direction, Administratif ; Accueil |
| `inscriptions.instruire` | Instruire, contrôler les pièces, relancer, saisir au guichet | établissement, sites désignés | = `inscriptions.gerer` actuel (Admin, Direction, Administratif). `inscriptions.gerer` est **conservé** comme alias le temps de la bascule (§16) |
| `inscriptions.avis` | Donner un avis sur les dossiers **qui lui sont confiés** | établissement (la restriction aux dossiers confiés est une règle du service, pas une portée) | aucun par défaut ; attribué par rôle (« Commission d'admission ») |
| `inscriptions.avis.lire` | Lire les avis internes | établissement | Admin, Direction |
| `inscriptions.decider` | Décider, revoir, liste d'attente, non-réinscription, commission | établissement | Admin, Direction [F] |
| `inscriptions.valider` | Valider une inscription (création de l'élève, appels aux socles 2 et 4) | établissement, sites désignés | Admin, Direction (= actuel, la validation exige `decider` [F]) ; l'établissement peut l'accorder au secrétariat |
| `inscriptions.campagnes.gerer` | Créer, ouvrir, modifier, fermer les campagnes et l'offre | établissement | Admin, Direction, Administratif (= actuel via `inscriptions.gerer`) |
| `inscriptions.departs.gerer` | Préparer les départs, certificats, non-présentés | établissement, sites désignés | Admin, Direction, Administratif |
| `inscriptions.effectifs.lire` | Prévisions d'effectifs, simulations | établissement | Admin, Direction, Administratif, Compta |
| `inscriptions.exporter` | Exports nominatifs, listes mairies | établissement | Admin, Direction, Administratif |
| `inscriptions.parametrer` | Paramétrage du domaine | établissement | Admin |
| `imports.appliquer` [F] | Appliquer un import | établissement | inchangé (rejoindra le socle 9) |

Règles [R] :
- **Compatibilité** : les modèles par défaut reproduisent exactement les droits actuels (test oracle comme `r2_non_regression_profil_par_action` [F]) ; `inscriptions.gerer` reste dans le catalogue et vaut `instruire` + `campagnes.gerer` + `lire` tant que des rôles l'utilisent.
- **Inclusions explicites** : `inscriptions.instruire`, `inscriptions.decider`, `inscriptions.valider` incluent `inscriptions.lire` ; `inscriptions.decider` inclut `inscriptions.avis.lire`. Aucune autre inclusion implicite.
- **Portée « sites désignés »** : réutilise l'extension de la portée « désignés » aux sites proposée par `02_ELEVE.md` Q-E9 (secrétariat d'un site).
- **Clôture d'une scolarisation** : le départ appelle ELV-11, qui exige `eleves.sortie` (socle 2). Un acteur qui a `inscriptions.departs.gerer` sans `eleves.sortie` **prépare** le départ ; un autre l'enregistre.
- **Familles et élèves** (hors rôles libres, D-017) :
  - avant validation : le **déposant vérifié** voit et modifie ses démarches ; un **responsable déclaré** ayant répondu à une demande d'accord voit la démarche (lecture, accord ou désaccord, pièces le concernant) ; personne d'autre ;
  - après validation et pour la réinscription : droits issus du **lien actif** (`ent_eleve_lien_actif` [F]) : confirmer la réinscription, annoncer un départ, signer exigent l'**autorité parentale** ; payer est ouvert au **payeur** ;
  - **élève majeur** : il est le titulaire de ses démarches ; l'accès de ses parents suit la règle encore ouverte (Q10 de `SOCLE2_DOMAINES.md`, `02_ELEVE.md` Q-E14) ;
  - **contact** (prospect) : aucun accès hors du lien de gestion de son rendez-vous.
- **Anti-escalade** inchangée (D-017).
- Les routes ne font qu'un pré-filtre ; le service refait le contrôle précis, comme en R2 [F].

---

## 13. Événements (outbox, dans la transaction du fait)

**Charge utile : identifiants, codes, dates ; jamais de nom, d'adresse, d'e-mail, de motif rédigé, d'IBAN** (règle déjà respectée par S2-3 [F]). Les noms **existants sont conservés** : `admission.campagne_ouverte`, `admission.campagne_fermee`, `admission.deposee`, `admission.etape_changee`, `admission.decidee`, `admission.relance_envoyee`, `inscription.validee`, `inscription.invitation_acceptee`, `scolarite.depart_annonce`, `import.lot_applique` [F `lot5:ent-events/src/lib.rs:197-211`]. Les nouveaux suivent le même préfixe (`admission.*` pour une candidature, `inscription.*` pour ce qui vaut pour tous les types, `reinscription.*`, `scolarite.depart_*`).

### 13.1 Émis

| Événement | Charge utile | Consommateurs [R] |
|---|---|---|
| `admission.campagne_ouverte` [F] / `inscription.campagne_modifiee` / `admission.campagne_fermee` [F] | campagne, nature, année | page publique, notifications (8), pilotage |
| `reinscription.campagne_ouverte` [F] | campagne, nombre de dossiers | notifications aux familles (8), à-faire (29) |
| `inscription.contact_cree`, `inscription.contact_converti` | contact, source | pilotage (entonnoir) |
| `inscription.accueil_reserve`, `inscription.rdv_*` | créneau, statut | rappels (8), agenda (19, iCal) |
| `admission.deposee` [F] / `admission.retiree` | dossier, campagne, nombre d'enfants de la démarche | accusé de réception (8), file de travail |
| `admission.etape_changee` [F], `inscription.piece_controlee` (aucun événement aujourd'hui [F]) | dossier, étape ou type de pièce, statut | notification famille si visible (8) |
| `inscription.accord_demande` / `_donne` / `_refuse`, `inscription.bloque_desaccord` | dossier, déclaration, objet | notifications (8), file de travail |
| `admission.decidee` [F] (désormais aussi à la révision) | dossier, sens, offre, décision remplacée | courrier (7), notification (8), compteur de places, pilotage |
| `inscription.place_proposee`, `inscription.reponse_famille` | dossier, offre, échéance / réponse | rappels (8), compteur de places |
| `inscription.validee` [F] | dossier, élève, nature, année, offre, régime, payeurs (identifiants) | **facturation** (lignes annuelles au prorata [F branche `ent-socle2-finances`, `finances_reactions.rs:5`]), invitations [F], message d'accueil [F], tâches de rentrée, restauration, internat, pilotage. **Plus d'affectation** (§2.2 n° 6) |
| `inscription.desistement` | dossier, élève éventuel, date | facturation (remboursement selon contrat), restauration, internat, compteur |
| `eleve.reinscription_confirmee` [F] | dossier, élève, année | facturation (acompte, lignes N+1), pilotage |
| `inscription.niveau_determine` | dossier, élève, niveau, redoublement | famille informée (8), prévisions |
| `reinscription.non_reinscription_decidee` | dossier, élève, date | courrier (7), départ |
| `scolarite.depart_annonce` | élève, date prévue, origine | domaines pour leur checklist, pilotage |
| `scolarite.depart_enregistre` | élève, départ, date, motif codé | facturation (prorata, solde), restauration, internat, comptes (1), prêts (44), listes mairies, pilotage |
| `inscription.purge` | catégorie, nombre | journal RGPD (27) |

Les doublons actuels (`reinscription.campagne_ouverte`, `eleve.reinscription_confirmee` émis par l'ancien et le nouveau code [F]) disparaissent avec la reprise (§16) ; jusque-là, les consommateurs dédupliquent par `dossier_id` ou par élève et année.

### 13.2 Consommés

| Événement (socle) | Effet |
|---|---|
| `conseil.decision_passage` ou équivalent (23) | INS-09 : niveau du dossier de réinscription et de l'inscription annuelle prévue |
| `document.signe` (7) | condition « contrat signé » |
| `paiement.recu`, `paiement.rembourse` (25) | conditions « frais payés », « acompte payé » |
| présences de la rentrée (12) | liste des non-présentés (INS-43) |
| `sanction.exclusion_definitive` (13) | ouvre un départ |
| `scolarite.closed` (2) | ferme le dossier de réinscription en cours |
| `eleve.fusionne` / `personne.fusionnee` (2, 4) | redirige les références |
| `lien.suspendu` (4) | retire l'accès famille aux démarches et coupe les relances |
| `invitation.acceptee` (1) | met à jour l'accord et le suivi |

---

## 14. Écrans clés

Leptos 0.8 (`docs/FRONTEND_LEPTOS.md`), design system maison, données et **actions proposées par le serveur** (corriger l'écart actuel des boutons non conditionnés [F §2.2 n° 11]). Toute action ★ passe par un **récapitulatif avant / après** et une confirmation. Aucun écran ne demande un identifiant technique (UUID) : on choisit dans une liste.

### 14.1 Personnel

| Écran | Route [R] | Contenu | Pour qui |
|---|---|---|---|
| **Tableau de bord des inscriptions** | `/web/inscriptions` | par campagne : entonnoir (contacts → dossiers → admis → confirmés → validés), **places par offre en direct** (jauges), délais moyens, dossiers en retard, comparaison avec la même date l'an dernier ; « à traiter aujourd'hui » | direction, secrétariat |
| **File de travail** | `/web/inscriptions/a-traiter` | dossiers triés par urgence (échéance, ancienneté, pièces déposées à contrôler, accords reçus, réponses de familles) ; actions groupées (relancer, assigner) | secrétariat |
| **Liste des dossiers** | `/web/inscriptions/dossiers` | tableau dense virtualisé ; filtres (campagne, état, étape, offre, vœu, assigné, priorité, retard) ; colonnes au choix ; vues enregistrées ; adresse directe vers chaque dossier | secrétariat, direction, accueil (lecture) |
| **Dossier** | `/web/inscriptions/dossiers/{id}` | en-tête (candidat, référence, état, étape, prochaine échéance, conditions restantes) ; onglets : **Candidat** · **Responsables et accords** · **Vœux** · **Pièces** (aperçu du document à côté de la liste, accepter / refuser / dispenser) · **Rendez-vous et avis** · **Décision** · **Échanges** (suivi, notes internes distinguées visuellement) · **Frais et contrat** ; fratrie : liens vers les dossiers frères | selon droits |
| **Validation** | dans le dossier | assistant en trois étapes : (1) élève : doublons et anciens élèves proposés par le socle 2 (« créer » ou « réintégrer X ») ; (2) responsables : correspondances proposées par le socle 4 (« même personne » / « nouvelle personne »), foyers, payeurs ; (3) récapitulatif de ce qui sera créé et envoyé (invitations, facture, tâches de rentrée) → confirmation | secrétariat ou direction |
| **Commission** | `/web/inscriptions/commissions/{id}` | grille des dossiers avec critères, avis, places restantes en direct ; décision par ligne ou en lot, avec récapitulatif ; procès-verbal généré | direction |
| **Liste d'attente** | `/web/inscriptions/attente` | par offre, rang modifiable par glisser ; quand une place se libère : bandeau « proposer à X » → confirmation | direction |
| **Campagnes** | `/web/inscriptions/campagnes` | assistant de création (type → offre → pièces → étapes → délais → frais et contrat → textes et courriers → aperçu de la page publique) ; copie de l'an dernier ; versions | direction |
| **Entonnoir et accueil** | `/web/inscriptions/contacts`, `/accueil` | contacts, portes ouvertes (créneaux, inscrits, pointage par QR), rendez-vous (agenda), relances | accueil, secrétariat |
| **Réinscriptions** | `/web/inscriptions/reinscriptions` | par classe : chaque élève avec son état (à confirmer, confirmé, en attente de passage, départ annoncé, sans réponse, retenu : situation financière), vœux, conditions ; relance groupée ; traitement des sans-réponse | secrétariat, direction, professeur principal (lecture de sa classe, Q-I14) |
| **Départs** | `/web/inscriptions/departs` | départs annoncés et en préparation, checklist par élève (points remontés par les domaines), certificat, enregistrement ; non-présentés de la rentrée ; départs de fin d'année en lot | secrétariat, direction, vie scolaire |
| **Prévisions d'effectifs** | `/web/inscriptions/effectifs` | tableau niveau × parcours × régime : montée, redoublants, départs, admissions, attente, capacité, prévision ; hypothèses nommées ; courbe de l'année comparée à N-1 ; export pour la préparation de la rentrée (socle 5) et le budget | direction |
| **Imports d'élèves affectés** | `/web/inscriptions/imports` | dépôt, prévisualisation, rejets ligne à ligne, création des dossiers ; écran manquant aujourd'hui [F] | secrétariat |
| **Paramétrage** | `/web/direction/parametrage/inscriptions` | profil, types de campagne, motifs, critères, modèles de courriers, page publique, relances, délais | administrateur |

### 14.2 Familles, élèves, public

| Écran | Route [R] | Contenu |
|---|---|---|
| **Page publique d'inscription** de l'établissement | `/inscription` (sous-domaine de l'établissement, D-012) | offre ouverte, dates, portes ouvertes, documents (règlement, tarifs), « demander des informations », « déposer une candidature » ; aucun traceur tiers |
| **Préinscription** | `/web/preinscription` (existant [F], refondu) | e-mail → code → « utiliser mes informations ? » → **enfants (un ou plusieurs)** → responsables (le second parent : e-mail, foyer, autorité parentale) → scolarité actuelle → vœux (places restantes affichées si l'établissement le choisit) → pièces du moment → récapitulatif → dépôt ; brouillon repris à tout moment |
| **Mes démarches** | `/web/demarches` | une carte par enfant : étape, prochaine échéance, ce qu'on attend de moi, pièces, échanges ; réponse à une décision ; contrat ; paiement ; historique daté |
| **Accord du second parent** | `/web/accord?jeton=…` | e-mail vérifié par code ; démarche en lecture ; « je donne mon accord » / « je ne donne pas mon accord » (avec texte d'information neutre) ; sans compte |
| **Réinscription** | `/web/reinscription` (existant [F], refondu) | les 3 gestes, tous les enfants sur un écran |
| **Annoncer un départ** | dans Mes démarches | date, destination, motif, demande de certificat |
| **Mobile** (appli unique, D-003) | — | familles : Mes démarches, **photo d'une pièce** (caméra, recadrage), réponse à une décision, réinscription en 3 gestes, signature, rappels push ; personnel : accueil des portes ouvertes (pointage QR), lecture d'un dossier, **pas** d'instruction ni de décision sur mobile (tâches complexes au web, D-003) |

Critères d'ergonomie [R] : 375 px, clavier complet, lecteur d'écran, contraste (RGAA à auditer, socle 29) ; préinscription faisable en moins de 10 minutes pour un enfant avec ses pièces prêtes ; réinscription en moins d'une minute quand rien n'a changé ; deux clics au plus d'une liste au dossier ; chaque masquage dit pourquoi.

---

## 15. Données personnelles, sécurité, équité

### 15.1 Cloisonnement

- `etablissement_id NOT NULL` partout, déduit de l'hôte et de la session (D-012), jamais du client ; clés étrangères composées ; **RLS** sur toutes les tables `ent_inscription_*` (mode permissif puis strict, `RLS_ISOLATION.md`) ; dépôts en `TenantTx`.
- Tests de séparation étendus à **toutes** les routes (y compris publiques et à jeton) et à **plusieurs hôtes** en HTTP (manque actuel [F §2.2 n° 13]).
- **Aucune** information sur l'existence d'une famille ou d'un enfant dans un autre établissement (D-013) : la détection de doublons et de correspondances ne regarde **que** l'établissement.

### 15.2 Minimisation par étape

| Moment | Ce qu'on demande | Ce qu'on ne demande pas |
|---|---|---|
| Contact | e-mail, prénom de l'enfant, année de naissance, niveau visé | identité complète, adresse |
| Candidature | identité de l'enfant, scolarité actuelle, responsables (identité, lien, autorité parentale, contacts), vœux, pièces utiles à la décision (bulletins si l'établissement sélectionne) | santé, handicap, situation sociale, religion, nationalité, IBAN, revenus |
| Admission confirmée | pièces administratives, contrat, frais ; **annonce** d'un besoin (PAI, PAP, PPS) sans détail, pour préparer l'accueil | contenu médical (socle 34 après validation) |
| Validation | IBAN (D-014) si prélèvement | — |
| Rentrée | fiche de rentrée (4), santé (34), autorisations (18) | — |

Le **quotient familial** ou les revenus ne sont demandés qu'au socle 26 (bourses, tarification), jamais pour décider d'une admission.

### 15.3 Notes internes et avis

Les avis et notes internes ne sont **pas** montrés à la famille dans l'interface, mais ils font partie de ses données personnelles et peuvent lui être communiqués au titre du droit d'accès [H : RGPD art. 15, à confirmer par le DPO]. L'écran le rappelle à la saisie ; les avis sont courts, factuels, datés et signés. Aucun avis ne porte sur la santé, la religion, l'origine ou la situation familiale.

### 15.4 Conservation et purge

Durées **à fixer avec le DPO** (aucune durée codée avant) pour : contacts non convertis ; brouillons ; dossiers refusés, retirés, sans suite (et leurs pièces) ; avis ; journaux de vérification d'e-mail ; dossiers validés (rattachés au dossier de l'élève) ; listes mairies. Purge planifiée, journalisée (`inscription.purge`), avec export préalable possible pour l'établissement. Proposition à soumettre [R] : contacts non convertis et dossiers non aboutis supprimés à la fin de la campagne suivante.

### 15.5 Équité et non-discrimination

- Aucun champ du logiciel ne recueille religion, origine, opinions ; les **questions personnalisées** sont contrôlées par une liste de termes signalés (avertissement, pas blocage) et chaque question est affichée avec sa finalité.
- **Pas de décision automatisée** produisant un effet juridique (refus ou admission calculés) [H : RGPD art. 22] ; les critères aident à classer, une personne décide et motive ; aucune note de candidat produite par une IA. L'assistant (socle 30) peut résumer un dossier, jamais proposer une décision d'admission.
- Le motif communiqué est relu avant envoi (pas de motif interne envoyé par erreur : champs distincts, §2.2 n° 11).

### 15.6 Sécurité des parcours publics

- Anti-énumération et limitation de débit sur **toutes** les routes publiques et à jeton (manque actuel sur les routes à jeton [F]) ; jetons en empreinte, courts, liés à l'établissement ; aucune donnée affichée avant vérification [F, à garder].
- Dépôt de fichiers : contrôle du type réel, taille, antivirus, stockage privé, jamais d'URL publique (socle 7).
- Formulaire de contact : protection anti-robots sans service tiers par défaut (preuve de travail, limitation, champ piège) [R].
- Les liens envoyés par e-mail ne contiennent aucune donnée personnelle ; les SMS ne contiennent ni nom d'enfant ni décision.
- Journal de consultation des dossiers par le personnel (acteur, dossier) visible de la direction (`audit.lire`).

---

## 16. Plan de migration depuis l'existant

Règles : migrations **additives** d'abord ; reprise idempotente ; aucune migration ne doit échouer sur des données imprévues (elle les range en « à traiter » et le signale) ; répétition sur base de labo et jeux anonymisés, **jamais** sur la base de production (interdite) ; ancienne interface `ui.rs` figée (F-001) ; anciennes routes conservées puis dépréciées ; Next.js intouché.

| Étape | Contenu | Reprise / compatibilité | Vérification |
|---|---|---|---|
| **M0 Correctifs avant fusion de S2-3** | supprimer le rapprochement implicite par e-mail (écran de correspondances, lien vers une personne ayant un compte seulement après sa confirmation) ; plus de nouveau foyer systématique ; limitation de débit des routes à jeton ; jeton par session réservé aux responsables et majeurs ; motif par action ; confirmations à l'écran ; classe choisie dans une liste ; échec explicite des tests sans base en CI ; test d'isolation multi-hôtes | aucun changement de schéma | tests ajoutés : lien non créé vers une personne existante sans confirmation ; fratrie = un foyer |
| **M1 Numérotation** | décider les blocs (Q-I1) ; renuméroter le bloc élève de `02_ELEVE.md` ; vérifier qu'aucune autre branche n'utilise 200–219 | — | `git ls-tree` de toutes les branches |
| **M2 Ports** | ports §5.3 écrits dans le code des socles 2 et 4 sur le schéma actuel ; la validation et l'import passent par eux ; test de garde « aucune écriture hors socle 3 » | même résultat fonctionnel qu'aujourd'hui (moins le rapprochement implicite) | tests S2-3 existants verts ; test de garde |
| **M3 Campagnes et offre** | `annee_id`, `site_id`, nouveaux états et types ; `ent_inscription_offre` ; reprise de `ent_inscription_campagne_niveau` (code → `ent_niveau` par code ; non trouvés → rapport) | `annee_cible` gardé en lecture le temps de migrer les lecteurs | aucune campagne sans année ; rapport des niveaux non rapprochés |
| **M4 Dossiers** | `ent_inscription_demarche` (une par dossier existant) ; `etat` depuis `statut` (`incomplet` → `attente_famille`) ; `cand_*` depuis `eleve_*` ; `ent_inscription_decision` depuis `decision_*` ; `reference` attribuée | vue de compatibilité pour l'ancien DTO | égalité des comptes par état avant / après |
| **M5 Responsables** | `ent_inscription_declaration_responsable` depuis `ent_inscription_responsable` (rang → ordre ; `meme_foyer` → `foyer_cle`) | — | une déclaration par ligne reprise |
| **M6 Réinscription unifiée** | `ent_reinscription_campagne` (018) → campagnes `reinscription` (`acompte_centimes` > 0 → mode `explicite_avec_acompte`) ; exigences (028) → règles de pièces ; documents déjà déposés (`reinscription/…`) rattachés comme pièces ; confirmations (`ent_eleve_scolarite_annee` statut `reinscription`) → dossiers `confirmee` ; vœux (077) → `ent_inscription_voeu` ; `ent_inscription_suivi` (078) → états et étapes | anciennes routes `/reinscription/*` et `/inscriptions/par-etape` redirigées vers le nouveau service, puis retirées ; un seul émetteur par événement | une campagne par année et par établissement ; comptes de confirmations identiques |
| **M7 Pièces réelles** | `document_id` (socle 7) ; types de pièces ; reprise des noms de fichiers déclarés en « déclarée sans fichier » à faire redéposer | — | aucune pièce « acceptée » sans document pour les nouveaux dossiers |
| **M8 Droits** | nouvelles actions, modèles par défaut équivalents, alias `inscriptions.gerer` | oracle profil × action | tests de non-régression |
| **M9 RLS et clés composées** | sur toutes les tables du socle | — | tests sous rôle non propriétaire |
| **M10 Affectation retirée de la validation** | désactivation de `d1.affectation_inscription` pour les admissions (gardée pour `en_cours_annee` avec classe choisie) | les affectations déjà créées restent | test : valider en février ne crée aucune affectation |
| **M11 Écrans** | nouveaux écrans ; écrans S2-3 refondus ; `ui.rs` intouché | — | tests de rendu, navigateur, accessibilité |
| **M12 Retraits** | après bascule : `ent_reinscription_campagne`, `ent_reinscription_exigence_doc`, `ent_inscription_suivi`, `ent_voeu_inscription`, `ent_inscription_campagne_niveau`, colonnes `annee_cible`, `statut`, `eleve_*`, `classe_cible_id` | migration dédiée, après validation | grep des lectures à zéro |

---

## 16 bis. État du lot I0 (2026-11-06, branche `ent-socle-inscriptions-i0`)

**Implémenté et testé en local (PostgreSQL local, voir `PROJECT_STATUS.md`) ; non validé par Florian, non prêt pilote.** Base : `ent-integration-lot5` (contient S2-3) + `ent-mc0hoe`. Migration `250_inscriptions_i0.sql`.

| §2.2 | Fait | Test |
|---|---|---|
| 1 | Plus aucun rapprochement par e-mail. `GET /inscriptions/dossiers/{id}/correspondances` présente les fiches possibles (même e-mail, même nom et prénom, adresse masquée) ; `valider` refuse (409) sans choix humain explicite par responsable. Lien vers une personne ayant un compte : ligne `ent_inscription_lien_attente`, **aucun** `ent_eleve_lien` tant que le titulaire n'a pas confirmé par le lien reçu à l'adresse **du compte** (ou refusé : `POST /inscriptions/invitations/refuser`). Droits du lien : table `ent_inscription_parametre_lien` (`GET/PUT /inscriptions/parametres/liens`), à défaut notes/vie scolaire/justification = autorité parentale, sorties = non | `attaque_email_parent_existant_aucun_acces`, `refus_du_parent_existant_aucun_lien` |
| 2 | Foyer existant à confirmer (409 sinon) ; aucun nouveau foyer quand il est confirmé | `fratrie_foyer_existant_confirme` |
| 3 | Homonyme/INE : choix humain `ChoixEleve::Existant` (rattachement) ou `Nouveau` ; INE identique ⇒ rattachement obligatoire | `ancien_eleve_rattache_a_la_fiche_existante` |
| 4 | Un seul seau par IP pour toutes les routes à jeton famille ; jeton par session réservé aux parents et élèves majeurs | `jeton_famille_limite_de_debit_et_roles` |
| 5 | `ent_db::url_base_de_test()` : échec explicite en CI (`CI` ou `ENT_EXIGER_BASE=1`) ; test HTTP multi-hôtes | `jeton_famille_isole_entre_etablissements_http` |
| 6, 17 | File `ent_inscription_envoi` écrite dans la transaction, expédiée après commit (`expedier_envois`, tâche de fond de `ent-api`), reprise à délai croissant, 5 essais puis `echec` / invitation `non_envoyee` ; relance : échec tracé au suivi du dossier | `echec_d_envoi_trace_et_code_hmac`, `reactions_idempotentes`, `attaque_…` |
| 7 (pt 20) | Acceptation d'invitation en une transaction, fiche déjà rattachée jamais réassignée ; code de vérification en HMAC-SHA-256 (`ENT_SECRETS_KEY`, usage dédié) ; jeton d'invitation dérivé par HMAC (non stocké en clair) | idem + test unitaire `hmac_depend_du_secret_et_de_l_usage` |

**Limites connues (hors I0).** (a) Écritures directes dans `ent_eleve`/`ent_personne`/`ent_foyer`/`ent_eleve_lien` conservées dans le module (pt 2 du §2.2 : lot I1) — I0 n'en ajoute aucune nouvelle hors de ces tables déjà écrites. (b) La réintégration d'un ancien élève (statut, scolarisation) relève du socle 2 : I0 se contente de rattacher le dossier à la fiche. (c) **Écran de validation du frontend** : sans interface de rapprochement, la validation reçoit un 409 dès qu'une correspondance existe ; écran à faire. (d) L'import (`imports.rs`) crée toujours ses liens directement (pt 19, socle 9). (e) Invitation d'un responsable sans compte rapproché sur choix humain : lien direct (sans compte, inutilisable avant l'acceptation par l'adresse du dossier). (f) Interface d'administration du paramétrage des droits : API seulement. (g) Rattachement à un élève déjà suivi : refusé sans `confirme_responsables_existants` (anti-détournement par INE copié). (h) Points ouverts de la revue de sécurité : voir `PROJECT_STATUS.md`.

## 16 ter. État du lot I1 « ports et frontières » (2026-11-07, branche `ent-socle-inscriptions-i1`)

**Implémenté et testé en local (PostgreSQL local, résultats chiffrés dans `PROJECT_STATUS.md`) ; non validé par Florian, non prêt pilote.** Base : lots A0 et A0 bis du socle 4 (qui contiennent I0) + lots E0 à E2 du socle 2 (fusion `ent-socle-eleve-e2`). Migrations `300_inscription_annuelle_prevue.sql` et `301_inscriptions_i1_dossier.sql` ; `MIGRATION_TAG` = `301_inscriptions_i1_dossier` ; plage 300–304 du socle 3 (`MIGRATIONS.md`, 302–304 libres).

| Critère §16 (M1, M2) | Fait | Test |
|---|---|---|
| **M1 Numérotation** | Plage 300–329 du socle 3 réservée dans `MIGRATIONS.md` ; 300–301 utilisées ; aucune collision avec les branches 240, 250, 255, 260–269, 280 | `migration_tag_suit_la_derniere_migration` |
| **M2 Ports — socle 2** | Module `ent-db::eleve_ports` : `creer_eleve_tx` (ELV-01 : fiche + version d'identité initiale par la base, source `admission` ou `import`, auteur posé ; journal et événement `eleve.created` **sans état civil**), `enregistrer_inscription_annuelle_tx` (ELV-09 : inscription annuelle **prévue** = ligne `projetee` de `ent_eleve_scolarite_annee` avec niveau, parcours, régime, source `admission` / `reinscription`, idempotente, une année `courante` ou `historique` jamais réécrite), `affecter_arrivee_en_cours_d_annee_tx` (affectation initiale, réservée aux classes de l'année **courante**) | `i1_validation_cree_une_inscription_annuelle_prevue_sans_affectation`, `i1_arrivee_en_cours_d_annee_affecte_en_classe`, `i1_ancien_eleve_inscription_prevue_sur_la_meme_fiche`, `i1_port_inscription_annuelle_idempotent_et_borne` |
| **M2 Ports — socle 4** | `integrer_declaration_tx` (déjà A0) devient l'**unique** chemin de la validation **et** de l'import ; nouvelle commande `rattacher_compte_tx` (compte ↔ personne sans compte, jamais de réassignation) utilisée par l'acceptation d'invitation ; `SourceLien::Import` | `i1_import_par_les_ports_sans_lien_vers_un_compte_existant`, tests A0 / A0 bis inchangés |
| **Validation = inscription prévue, pas affectation** | La validation enregistre l'inscription annuelle prévue (niveau et parcours du corps de la requête, à défaut ceux du dossier ; année = année cible de la campagne) ; la classe choisie n'est retenue **que** si elle est de l'année courante (arrivée en cours d'année) ; l'événement `inscription.validee` ne porte `classe_id` que dans ce cas ; le dossier garde `inscription_annuelle_id` et `parcours_demande` | idem |
| **Test de garde** | `ent-db/tests/garde_inscriptions_ecritures.rs` : lit les sources (`ent-db/src` et `ent-api/src`, fichiers `*inscription*` et `*import*`, hors tests), échoue sur tout `INSERT`, `UPDATE` ou `DELETE` vers une table hors socle 3, table dynamique comprise ; exceptions nominatives et justifiées (comptes du socle 1, notification du socle 8, mandat et IBAN du socle 25), **qui font échouer le test dès qu'elles ne servent plus** | `le_module_d_inscriptions_n_ecrit_que_dans_ses_tables`, `le_detecteur_voit_les_ecritures_interdites`, `les_commandes_attendues_sont_appelees` |
| **« Ne pas solliciter »** | Inchangé et vérifié : aucune invitation, demande d'accord ni relance vers une personne marquée (tests A0 bis) ; l'import ne contacte personne et ne rapproche jamais d'une personne qui a un compte | `second_parent_ne_pas_solliciter_aucune_invitation`, `ne_pas_solliciter_pose_avant_l_envoi_bloque_l_invitation`, `relance_refusee_vers_une_personne_ne_pas_solliciter` |

**Écarts et limites du lot I1 (à valider par Florian).**
1. **Avant le lot E4 du socle 2**, `ent_eleve_inscription` n'existe pas : l'inscription annuelle prévue est une ligne `projetee` de `ent_eleve_scolarite_annee` (statuts `historique`, `courante`, `projetee`, `reinscription`). Le contrat de `enregistrer_inscription_annuelle_tx` ne changera pas quand E4 arrivera. La « scolarisation » (ELV-08 : entrée, origine) n'a pas encore de commande : l'événement `eleve.created` et la date de création en tiennent lieu.
2. **Statut de l'élève** : un élève validé hors arrivée en cours d'année reste « préinscrit » (colonne `status` héritée) jusqu'à la constitution des classes ; le statut dérivé (`a_venir`) vient avec E4.
3. **Import** : une ligne dont l'e-mail correspond à une fiche **créée par un import** et sans compte (fratrie d'un même fichier) reprend cette fiche et son foyer unique ; **toute autre correspondance donne une nouvelle fiche**, jamais un lien implicite. Cela peut créer des doublons de personnes (à fusionner par le socle 4) ; l'ancien comportement reprenait aussi les fiches saisies à la main sans compte. Le cadre d'import reste destiné au socle 9.
4. **Écritures restantes hors socle 3** (liste d'exceptions du test de garde) : création de compte et appartenance à l'établissement à l'acceptation d'invitation (socle 1), notification d'accueil (socle 8), mandat SEPA et historique d'IBAN (socle 25). Elles ne touchent aucune table des socles 2 et 4 ; leurs commandes arrivent avec I6 (comptes) et I7 (finances).
5. **Affectation en réaction** : la validation affecte directement (même transaction) pour une arrivée en cours d'année ; la réaction `d1.affectation_inscription` reste comme filet **idempotent** (rejouable) et passe par la même commande.
6. **Pas d'écran** : `ValidationBody` gagne `niveau` et `parcours`, `ValidationDto` gagne `inscription_annuelle_id` et `affecte_en_classe` (champs facultatifs) ; l'écran de validation (choix du niveau et du parcours dans l'offre) relève de I14.
7. **Reste du §16** : M3 à M12 (campagnes et offre, dossiers, responsables, réinscription unifiée, pièces, droits, RLS, retraits) — lots I2 et suivants.

## 16 quater. État du lot I2 « modèle : campagnes, offre, démarches, dossiers, déclarations » (2026-11-08, branche `ent-socle-inscriptions-i2`)

**Implémenté et testé en local (PostgreSQL 16 local, résultats chiffrés dans `PROJECT_STATUS.md`) ; non validé par Florian, non prêt pilote.** Base : `ent-integration-lot8b` (socles 2 à 4 réunis) + `ent-mc0hoe` (conception du socle 8). Migrations **302 à 309** (registre : `MIGRATIONS.md`) ; `MIGRATION_TAG` reste `370_edt_creneau_groupe` (la plus haute du répertoire).

| Critère §17 / §16 | Fait | Test |
|---|---|---|
| **M3 Campagnes et offre** | `annee_id` (référentiel S2-2, résolu à la création par la commande `ent_assurer_annee` du référentiel ; la reprise ne crée jamais une année), `site_id`, quatre natures (`admission`, `affectes`, `reinscription`, `en_cours_annee`), six états de campagne avec table de transitions (une campagne fermée **ne se rouvre pas**, §2.2 n° 18 ; suspendre / reprendre), règles paramétrables dans les bornes du logiciel (accord des responsables, décision requise, motif communiqué, mode de réinscription, délais 2 à 30 jours, annulation d'une validation 0 à 168 h), **version et journal** à chaque modification après la création, **offre** (niveau × parcours × régime, capacité facultative, historique des changements de capacité, unités d'hébergement R-I15), niveaux de campagne repris en offres sans parcours | `i2_campagne_etats` (36 paires d'états), `i2_campagne_regles_annee_et_offres_des_niveaux`, `i2_offres_cles_unites_et_droits` |
| **M4 Dossiers** | **une démarche par dépôt** (`ent_inscription_demarche`), `etat` (états du §7.1 à §7.3), **référence** `ADM-2026-0142` atomique par établissement / préfixe / année (compteur verrouillé), vœux classés (cinq au plus), **décisions en lignes** (motif interne obligatoire, motif communiqué selon la campagne, dépassement de capacité tracé, chaîne de révision), colonnes du modèle (`etape_code`, `echeance_famille`, `rang_attente`, `clos_le`, `motif_cloture`, `cand_*` additionnels…), clés composées établissement | `i2_references_uniques_sous_concurrence` (16 insertions simultanées → ADM-2071-0001 à 0016), `i2_decision_est_une_ligne_et_motif_communique` |
| **M5 Responsables** | `ent_inscription_declaration_responsable` : **une déclaration par démarche** (et non par dossier), rangs 1 à 6 (tuteur, beau-parent pour un seul enfant : `pour_enfants`), tenue d'accord avec les lignes par dossier par un déclencheur (même fonction pour la reprise) ; valeurs scellées (adresse à protéger, motif de « ne pas solliciter ») jamais copiées : seul l'indicateur l'est | `i2_fratrie_une_demarche_plusieurs_dossiers`, `i2_adresse_a_proteger_jamais_en_clair_dans_la_declaration` |
| **Reprise idempotente** | statut historique → `etat` (sans toucher au statut), démarche, référence, déclaration, décision (depuis `decision_*`), offre (depuis `ent_inscription_campagne_niveau`), vœu n°1 ; **rapport** (`ent_inscription_reprise_rapport`) pour l'année absente du référentiel, le niveau absent, le dossier dont le niveau n'a pas d'offre ; rejouée deux fois : aucun compte ne bouge | `i2_reprise_idempotente` |
| **Transitions contrôlées par le service** | **une seule table** (`ent-db/src/inscriptions_etats.rs`) pour les quatre natures, un seul point d'écriture de l'état (`transitionner_tx`), des transitions **réservées** à leur cause (validation, annulation de validation, décision, blocage) ; les états valides du SQL (`ent_inscription_etat_valide`) sont comparés à ceux du service | `i2_machine_d_etats_chaque_transition_est_testee` : **toutes** les paires (nature, de, vers) × toutes les causes contre le service et la base (états, vue historique, dossier inchangé quand refus) |
| **Compteur de places juste** | vue `ent_inscription_offre_occupation` recalculée à chaque lecture : `proposées` (admis), `confirmées`, `validées`, `liste_attente` (ne consomme pas), `demandes` (informatif), `réserve_reinscription` (offre jumelle d'une campagne de réinscription, mode tacite compris), `disponibles = capacité − réserve − proposées − confirmées − validées` (négatif = dépassement assumé) ; un dossier ne compte qu'une fois (son vœu retenu) ; un dossier bloqué garde sa place ; **admettre sur une offre pleine exige une confirmation** et se trace dans la décision ; deux admissions simultanées sur la dernière place : une seule passe | `i2_compteur_de_places_juste`, `i2_derniere_place_une_seule_admission` |
| **Fratrie native** | `POST /preinscription/demarches` : un à six enfants, une seule déclaration des responsables, un dossier et une référence par enfant, vœux par enfant ; le dépôt d'un seul enfant est une démarche d'un enfant (même code) | `i2_fratrie_…`, `i2_routes_http_…` |
| **Écritures des socles 2 et 4** | inchangé : **uniquement** par leurs commandes (test de garde I1 vert, sans nouvelle exception) ; les commandes inverses de l'annulation de validation sont écrites **dans les socles propriétaires** (`eleve_ports::annuler_creation_eleve_tx`, `retirer_inscription_annuelle_prevue_tx`, `dossier_personnes_ports::suspendre_liens_crees_le_tx`, `finances::annuler_lignes_annuelles_brouillon_tx`) | `le_module_d_inscriptions_n_ecrit_que_dans_ses_tables` |
| **Envois par la file** | prolongation et fermeture préviennent les familles par `ent_inscription_envoi` (type `information`), une fois par démarche et par adresse, après la validation de la transaction, **jamais** vers une adresse marquée « à ne pas solliciter » | `i2_prolonger_une_echeance_en_masse`, `i2_fermeture_suspend_les_delais_et_les_relances` |

**Règles des personas portées par le lot.**
- **« Ne pas solliciter » après le dépôt (INS-57, R-I8)** — `POST /inscriptions/dossiers/{id}/ne-pas-solliciter` (direction ★) : la marque est posée sur la ligne du dossier **et** la déclaration, le motif est scellé et jamais renvoyé, les invitations, liens en attente et envois **déjà programmés** vers cette personne sont annulés, l'événement ne porte ni motif ni personne, le déposant (interlocuteur du dossier) ne peut pas être marqué. Test : `i2_ne_pas_solliciter_apres_le_depot`.
- **Adresse à protéger (R-I9)** — l'adresse n'est jamais en clair (ligne du dossier : colonne vide + valeur scellée ; déclaration : aucune adresse). Test ci-dessus.
- **Annuler une validation erronée (INS-68, R-I11)** — `POST /inscriptions/dossiers/{id}/annuler-validation` (direction ★) : délai de la campagne (72 h par défaut), refus si un autre socle a produit des données depuis (**la liste des types d'événements est montrée**), commandes inverses dans **une** transaction (fiche créée par la validation archivée, inscription annuelle prévue retirée, liens créés suspendus, invitations révoquées, lignes de facturation brouillon retirées ; refus si une facture est émise), retour à `confirme` (`confirmee` pour une réinscription, `complete` pour un élève affecté), nouvelle validation possible avec une nouvelle fiche. Test : `i2_annuler_une_validation_erronee`.
- **Prolonger une échéance en masse (INS-79)** — `POST /inscriptions/campagnes/{id}/prolonger` : date limite de la famille, clôture des dépôts ou échéances de dossiers (toute la campagne ou une sélection), **jamais en arrière** (une échéance plus lointaine reste), familles prévenues par la file, relances recalées, journal des prolongations, version de campagne.
- **Fermetures du secrétariat (INS-80, R-I6)** — `POST /inscriptions/fermetures` : toute échéance qui tombe pendant la fermeture est reportée au jour de réouverture + N jours (N paramétrable), les échéances recalculées pendant la fermeture respectent la même règle, les relances sont **suspendues** (409), un message (motif public, contact d'urgence) s'affiche sur la démarche de la famille, la fermeture est annulable sans reculer les échéances déjà reportées.
- **Autres** : désaccord des responsables (`POST …/blocage`, secrétariat pose, direction lève ; le dossier garde sa place, aucune relance ni décision ni validation) ; retrait par la famille (`…/retirer`, avant admission) et désistement (`…/desister`, après admission, avant validation) ; transitions du personnel hors instruction (`POST …/transition` : sans suite, annulation par la direction, désistement constaté, non-présentation, réaffectation, non-réinscription par l'établissement, sans réponse) ; offres (`GET/POST /inscriptions/campagnes/{id}/offres`, `PUT/DELETE /inscriptions/offres/{id}`) ; état de campagne (`POST …/statut`).

**Écarts et limites du lot I2 (à valider par Florian).**
1. **`statut` historique et `etat`** coexistent le temps de migrer les lecteurs (M12) : un déclencheur (migration 304) tient `statut` d'accord avec `etat`, dans les deux sens. Les colonnes `eleve_*` ne sont **pas** renommées en `cand_*` (renommage reporté à M12 : seuls les champs nouveaux — nom d'usage, prénoms, commune et pays de naissance — sont ajoutés).
2. **Les dates de campagne restent des `DATE`** (`date_ouverture`, `date_cloture`) ; l'heure d'ouverture (`TIMESTAMPTZ` du §8.1) n'est pas portée. À trancher avec le lot I13 (paramétrage).
3. **Pas de RLS sur les nouvelles tables** : comme les tables d'inscription existantes (M9 « RLS et clés composées » est un lot à part, car le service d'inscriptions n'ouvre pas encore de `TenantTx`). Toutes les requêtes filtrent par `etablissement_id` et les clés étrangères sont composées `(etablissement_id, id)` ; sur les tables existantes les clés composées sont `NOT VALID` (nouvelles lignes contrôlées, anciennes non réécrites).
4. **Année du référentiel** : à la création d'une campagne pour une année absente, la commande `ent_assurer_annee` du référentiel la crée (1ᵉʳ septembre – 31 août), y compris pour une année future ; si l'établissement a des dates d'année propres, la création de campagne échoue proprement plutôt que d'inventer. À confirmer avec le socle 5 (préparation de l'année suivante).
5. **Décision depuis l'instruction seulement** : décider depuis `deposee` ou `attente_famille` passe implicitement par la prise en charge (deux lignes de suivi). **La revue d'une décision** (états `admis` ⇄ `refuse` ⇄ `liste_attente`) est dans la table de transitions mais l'endpoint historique la refuse encore (`decision_admise`) : elle arrive avec le lot I5 (commission, conditions).
6. **Confirmation** (`confirme`) : sans conditions avant le lot I5, la validation franchit `admis → confirme → validee` d'elle-même (deux lignes de suivi).
7. **Motif de décision** : le comportement historique est conservé (le motif saisi est aussi communiqué quand la campagne le permet) ; pour garder un motif interne seulement, envoyer `motif_communique: ""`. La famille ne lit **que** le motif communiqué.
8. **Réserve des réinscrits** : calculée sur les réinscrits dont le **vœu retenu** porte sur l'offre jumelle ; la préparation des dossiers de réinscription (lot I8) devra poser ce vœu. Les unités d'hébergement ont leur capacité et leur occupation (vœux rattachés), mais l'attribution d'une unité à un dossier relève du socle 41.
9. **Annulation de validation** : la fiche créée est marquée **`archive`** (statut dérivé ; `status` hérité intact pour ne pas ouvrir de scolarisation par le pont de l'ancien chemin), avec trace au journal et événement `eleve.creation_annulee` — en attendant la commande ELV-02 complète du socle 2 (le statut `archive` peut être recalculé par une projection ultérieure : l'événement et le dossier font foi) ; elle est refusée si la fiche a déjà une scolarisation, une classe ou une sortie. Les lignes de facturation retirées sont les lignes annuelles brouillon (`annuel:{année}`) ; une facture émise bloque l'annulation.
10. **Pas d'écran** (lot I14) ; **pas de relance planifiée** ni de bascule « sans suite proposée » à l'expiration d'un délai (tâche planifiée du lot I5/I8) ; `sans_suite` et `sans_reponse` se posent à la main.

## 17. Découpage en lots de code (équipe Sonnet)

Chaque lot livre : migrations additives, dépôt (`ent-db`), service en une transaction, routes (`ent-api`) et types (`ent-types`), tests Postgres (isolation entre établissements, matrice de droits, parcours), écrans quand le lot en a, documentation (ce document, `SOCLE2_DOMAINES.md` D3, `PROJECT_STATUS.md`). Revue `review-sonnet` sur chaque lot ; revue sécurité (skill `security-review`) sur I0, I2, I4, I6, I8 et sur tout ce qui touche aux routes publiques. Travaux parallèles en sessions séparées ou worktrees vérifiés (F-007), quatre compilations au plus en même temps (F-006). Effort : **S** ≤ 3 j, **M** 1–2 sem., **L** > 2 sem. [H].

| Lot | Contenu | Dépend de | Critère de fin | Effort |
|---|---|---|---|---|
| **I0 Correctifs S2-3** | M0 | — | rapprochement implicite supprimé (test d'attaque : un dossier qui nomme l'e-mail d'un parent existant ne lui donne aucun accès) ; fratrie = un foyer ; routes à jeton limitées ; tests rouges sans base en CI ; S2-3 fusionnable dans `ENT` | S–M |
| **I1 Ports et frontières** | M1, M2 ; commandes dans les socles 2 et 4 ; test de garde | I0 ; `02_ELEVE.md` E0 souhaitable | plus aucun INSERT/UPDATE hors socle 3 dans le module ; tests S2-3 verts | M |
| **I2 Modèle : campagnes, offre, démarches, dossiers, déclarations** | M3, M4, M5 ; fratrie ; états du §7.1–7.2 ; références ; compteur de places | I1 | reprise idempotente ; transitions interdites refusées par le service (test de chaque transition) ; compteur juste sur un jeu de labo | L |
| **I3 Pièces réelles** | types, règles conditionnelles, dépôt via le socle 7 (photo mobile → PDF), réutilisation dans la démarche, rattachement à la validation | I2 ; stockage du socle 7 existant | contrôle du type réel ; pièce « famille » déposée une fois pour deux enfants ; purge des non-aboutis testée | M |
| **I4 Accords des responsables** | demandes, réponses, papier, désaccord bloquant ; écran sans compte | I2 | dossier bloqué en cas de désaccord ; aucune décision possible ; levée avec pièce | M |
| **I5 Décisions, commission, liste d'attente, propositions, conditions** | §8.7 ; courriers générés | I2 | décision sans motif interne refusée ; dépassement confirmé tracé ; place libérée → suivant désigné, envoi sur confirmation | L |
| **I6 Validation refondue** | assistant de rapprochement (élève, responsables), inscription annuelle `prevue`, tâches de rentrée, invitations seulement aux adresses confirmées, plus d'affectation (M10) | I1, I4, I5 ; socle 2 E4 (ou ports sur le schéma actuel) | une transaction ; ancien élève réintégré ; aucun lien vers une personne sans choix humain ; événement sans donnée personnelle | M |
| **I7 Frais, contrat, IBAN** | commandes vers facturation et documents, conditions remplies par événements, remboursement au désistement | I5 ; branches finances et documents | acompte payé → condition remplie ; désistement → demande de remboursement selon contrat | M |
| **I8 Réinscription unifiée** | M6 ; modes ; 3 gestes multi-enfants ; niveau par décision de passage ; non-réinscription établissement ; sans-réponse ; impayés signalés | I2, I7 | une seule campagne par année ; ancienne réinscription reprise sans perte ; un seul émetteur d'événement | L |
| **I9 Départs** | démarche, checklist interrogeant les domaines, certificat, clôture via ELV-11, non-présentés, départs en lot, listes mairies | I1 ; socle 2 E4 | départ enregistré = scolarisation close + services informés ; certificat généré ; liste mairie par commune | M–L |
| **I10 Entonnoir et rendez-vous** | contacts, portes ouvertes, créneaux, réservations provisoires, reconfirmation, iCal ; conversion | I2 | parcours Next.js égalé sans dépendance Google ; consentement daté ; purge | M |
| **I11 Élèves affectés et imports** | type `affectes`, import générique (format documenté) puis formats nationaux **après** spécifications (socle 9) ; écran d'import | I2 ; cadre d'import S2-3 | liste importée → dossiers à compléter, jamais d'élève créé ; validation avec pièces en attente | M |
| **I12 Pilotage et prévisions** | tableau de bord, instantanés, hypothèses, comparaison N-1, exports | I2, I8 | chiffres reproductibles à partir des événements ; écart N-1 explicable | M |
| **I13 Paramétrage et profils** | profils d'établissement, modèles de courriers, critères, relances, page publique | I2 | profils public / privé / hors contrat appliqués ; bornes respectées | M |
| **I14 Écrans** | sous-lots : I14a dossier + file de travail ; I14b préinscription et Mes démarches ; I14c campagnes et offre ; I14d commission et attente ; I14e réinscriptions ; I14f départs ; I14g entonnoir ; I14h prévisions ; I14i paramétrage | lots back correspondants | tests de rendu et contre le vrai backend ; clavier, 375 px, axe-core | L (total) |
| **I15 Mobile famille** | Mes démarches, photo de pièce, réponse, réinscription, signature ; pointage QR accueil | I3, I8 ; socle 11 | contrat JSON testé ; parité des droits avec l'API | M |
| **I16 Conservation et droits des personnes** | purges, extrait des données d'un dossier (droit d'accès), journal de consultation | I2, DPO (Q-I15) | purge journalisée ; extrait complet (avis compris) | S–M |

Ordre recommandé : **I0** (débloque la fusion de S2-3) → **I1** → **I2** → (I3, I4, I5 en parallèle) → I6 → (I7, I8, I9 en parallèle) → (I10, I11, I12, I13) → I14 au fil de l'eau → I15, I16. Le parcours minimal vendable (« saison d'inscription d'un établissement privé ») est **I0 → I8 + I14a/b/c/d/e** ; celui d'un établissement public est **I0, I1, I2, I3, I6, I8, I9, I11**.

---

## 17 bis. Ajouts de la passe « personas » [personas]

Origine, personas et gravité : `docs/socles/PERSONAS_02-04.md`. Le lot qui porte chaque règle est indiqué ; **I0** et **A0** signalent un lot en cours.

| # | Règle [personas] | Lot |
|---|---|---|
| R-I1 | **Liste noire étendue.** Le logiciel ne demande jamais, et signale dans les questions personnalisées et les pièces demandées : titre de séjour, situation administrative au regard du séjour, nationalité des responsables, revenus ou avis d'imposition **avant l'admission** (la tarification relève du socle 26, après), en plus de la religion, de l'origine, des opinions et de la santé avant l'admission (§15.5). Le catalogue des types de pièces ne contient aucun « titre de séjour ». Fondement : droit à l'instruction indépendant de la situation administrative [NV, à vérifier] | I2, I13 |
| R-I2 | **Pièce « sans objet ».** Statut de pièce `sans_objet`, distinct de `dispensee`, avec motif codé (`arrivee_etranger`, `instruction_famille`, `premiere_scolarisation`, `autre`) : un enfant arrivé de l'étranger n'a pas de certificat de radiation ; jamais bloquant ; compté à part dans les statistiques | I3 |
| R-I3 | **Pièces en langue étrangère.** Attribut `langue` sur la pièce ; traduction jointe **facultative** ; aucune exigence de traduction assermentée sauf paramétrage explicite [NV] | I3 |
| R-I4 | **Parcours famille multilingue.** Préinscription, Mes démarches, accord du second parent et réinscription disponibles en français et dans d'autres langues (liste du logiciel ; premières langues proposées [R] : anglais, arabe, espagnol, portugais, turc, ukrainien, russe, dari) ; les textes du logiciel sont traduits par ScolIA ; les textes de l'établissement restent en français, avec traduction facultative ; la langue choisie est gardée pour les envois (socle 8) ; les documents contractuels en français font foi | I14b (+29) |
| R-I5 | **Arrivées hors campagne.** La campagne `en_cours_annee` de N+1 s'ouvre dès la clôture des admissions de N+1 (arrivées d'été, mutations) ; un établissement peut choisir un mode **admission au fil de l'eau** (campagne d'admission permanente, sans date de clôture : hors contrat surtout) | I2 |
| R-I6 | **Délais et fermetures.** Les fermetures du secrétariat sont des événements du calendrier (socle 5, cible « secrétariat ») ; un délai famille qui expire pendant une fermeture est reporté au jour de réouverture + N jours (paramètre) ; relances suspendues ; les délais peuvent se compter en jours ouvrés (paramètre) | I5, I8 |
| R-I7 | **Publication programmée.** Désactivée par défaut ; une décision programmée n'est visible ni de la famille ni des compteurs publics avant l'heure ; confirmation demandée pour une heure entre 20 h et 8 h, un dimanche ou un jour férié | I5 |
| R-I8 | **« À ne pas solliciter ».** Marque posée par la direction (INS-57) ou demandée par la famille (INS-58) ; effets **immédiats** : aucune demande d'accord, invitation, relance ou copie vers la personne, y compris déjà programmées ; la personne n'apparaît dans aucune liste d'envoi de la démarche ; à la validation, la marque est transmise au socle 4 (interdiction de communication ou adresse protégée, décidées par la direction). **I0 doit déjà respecter la marque pour la demande d'accord ; A0 pour l'invitation** | **I0**, I4, I6 ; **A0** |
| R-I9 | **Confidentialité déclarée à la préinscription.** Une adresse déclarée « à protéger » est stockée scellée dans le dossier (`champs_chiffres`) et **n'est jamais transmise en clair** au socle 4 : `integrer_declaration` reçoit le champ réservé `adresse_a_proteger` et la parque pour la direction (socle 4, R-A39) ; aucune demande d'accord ne part tant que la direction n'a pas statué | I2, I6 ; **A0** |
| R-I10 | **Accord dispensé.** Quand la règle est « tous les titulaires » et qu'un titulaire est « à ne pas solliciter », la condition d'accord est remplie par une **dispense motivée** de la direction (pièce attendue), jamais automatiquement | I4 |
| R-I11 | **Annulation d'une validation.** Délai paramétré (défaut 72 h, bornes 0 à 7 jours) ; refus si un autre socle a produit des données depuis (note, absence, paiement reçu, document signé) ; sinon commandes inverses dans **une** transaction : annulation de l'élève créé (ELV-02) ou de l'inscription annuelle, liens créés clos par le socle 4 (sans effacement), invitations révoquées, facture annulée par commande ; le dossier revient à `confirme` | I6 |
| R-I12 | **Recours.** Échéance de réponse paramétrée ; la réponse est une décision humaine motivée ; dans le public, le recours contre une affectation relève de l'autorité académique [NV, à vérifier] : ScolIA enregistre et oriente, ne décide pas | I5 |
| R-I13 | **Conflit d'intérêts en commission.** Un membre lié au candidat (socle 4 : responsable, contact, payeur ; socle 6 : apparenté déclaré) se voit proposer le déport ; ses avis déjà donnés restent mais sont marqués ; il ne vote pas | I5 |
| R-I14 | **Décisions liées.** Groupe de dossiers déclaré par la famille (jumeaux, fratrie) ; la commission voit le groupe ; des décisions différentes exigent une confirmation explicite et un motif | I5 |
| R-I15 | **Sous-capacités d'internat.** Une offre de régime interne peut se décomposer en **unités d'hébergement** (dortoirs, chambres adaptées) avec leur capacité et leurs conditions d'accès, définies par l'établissement ; le compteur de places dit s'il reste une place **compatible** ; l'affectation à une chambre reste au socle 41 | I2 |
| R-I16 | **Départ express.** Un départ annoncé moins de 5 jours ouvrés avant sa date déclenche une checklist prioritaire : certificat émis le jour de l'enregistrement, documents de l'année déposés au coffre, compte famille gardé 30 jours, soldes signalés sans bloquer | I9 |
| R-I17 | **Élève majeur sans responsable.** Aucun responsable n'est exigé ; la règle d'accord ne s'applique pas ; payeur : l'élève lui-même ou un organisme ; les contrôles de qualité « aucune autorité active » ne s'appliquent pas à un majeur | I2, I6 |
| R-I18 | **Fratrie sur plusieurs campagnes.** Un enfant en admission et un autre en réinscription la même année : une seule page « Mes démarches », une seule signature du contrat quand le gabarit le permet | I8, I14b |
| R-I19 | **Attestation d'inscription.** Vérifiable par code ou QR (socle 7) ; mention « sous réserve de la décision de passage » quand le niveau n'est pas connu ; bilingue possible | I6 (+7) |
| R-I20 | **Dossier scolaire transmis.** Liste fermée de contenus ; la famille en est informée et peut le consulter ; jamais santé (34) ni suivi social (36) ; envoi par lien court sécurisé ou papier, avec bordereau conservé | I9 |
| R-I21 | **Fraude suspectée.** Aucune conséquence automatique ; note scellée lisible de la direction ; dans le public, procédure de l'autorité [NV, à vérifier] | I5 |
| R-I22 | **Co-remplissage par l'élève.** Désactivé par défaut ; âge minimal paramétré ; l'élève ne voit ni les champs financiers ni les coordonnées des responsables ; seul le responsable dépose | I14b |
| R-I23 | **Avis externe.** La famille est informée dans le formulaire qu'un avis extérieur peut être demandé (finalité, destinataire) ; le tiers ne voit que le strict nécessaire ; son lien expire en 7 jours | I5 |
| R-I24 | **Tests d'admission.** Les résultats sont des avis ; un tri par résultat n'est qu'un affichage choisi par une personne ; aucun seuil d'admission automatique | I5 |
| R-I25 | **Familles sans domicile stable.** Le justificatif de domicile n'est jamais bloquant ; une attestation de domiciliation (centre communal d'action sociale, association) ou une attestation sur l'honneur est acceptée [**[V]** pour l'inscription sur la liste scolaire du maire : D131-3-1 ; second degré : avis du juriste, VERIF R48] | I3 |
| R-I26 | **Reprise en pleine saison.** Table de correspondance des états de l'ancien logiciel ; dates d'origine conservées (dépôt, décision) ; aucune notification, relance ni accusé de réception rétroactif ; rapport des dossiers non rapprochés | I11 (+28) |
| R-I27 | **Courriers et documents en trames prêtes (D-023).** Les « modèles de courriers paramétrables » et le « gabarit du certificat » (§9, §11) deviennent un **choix parmi 3 ou 4 trames** par type (accusé de réception, demande de pièces, convocation, décisions, proposition de place, non-réinscription, réponse à un recours, attestation d'inscription, certificat de radiation, bordereau du dossier scolaire, fiche d'inscription, listes mairie) ; l'établissement n'ajuste que son identité et un texte libre encadré ; aucune trame ne se construit dans un traitement de texte. Le paramétrage existant n'est pas supprimé : il est **signalé** pour décision commune (`PERSONAS_02-04.md` §5 bis.2, S-07 à S-14) | I13 (+7) |

**Droits** : aucun nouveau droit ; INS-57 et INS-62 utilisent `inscriptions.decider`, INS-79 et INS-80 `inscriptions.campagnes.gerer`. **Événements ajoutés** : `admission.modifiee`, `inscription.deposant_change`, `inscription.sollicitation_suspendue`, `inscription.confidentialite_demandee`, `admission.decisions_publiees`, `inscription.deport`, `inscription.recours_recu` / `_repondu`, `inscription.validation_annulee`, `inscription.voeu_change`, `scolarite.dossier_transmis`, `inscription.echeance_prolongee`, `inscription.fermeture_declaree`, `inscription.session_tests_*`, `inscription.avis_externe_*` ; charges utiles sans donnée personnelle. **Consommé** : `eleve.decede` (socle 2) clôt toute démarche en cours de l'élève sans aucun message.

---

## 18. Questions à trancher par le porteur du projet

| # | Question | Options | Recommandation |
|---|---|---|---|
| **Q-I1** | Numéros de migration : S2-3 occupe 140–145, que `02_ELEVE.md` réservait à l'élève ; 150, 170–172, 180–182, 190–193 sont pris ailleurs | — | **socle 2 → 160–169, socle 3 → 200–219** ; corriger `02_ELEVE.md` §7 |
| **Q-I2** | Corriger S2-3 (lot I0) **avant** de le fusionner dans `ENT`, ou fusionner puis corriger ? | avant / après | **avant** : le rapprochement implicite par e-mail donne un accès à des données d'élèves |
| **Q-I3** | La validation ne crée **plus d'affectation en classe** ; l'affectation relève de la constitution des classes (socle 5), sauf arrivée en cours d'année | oui / non | **oui** |
| **Q-I4** | Un candidat n'est **pas** un élève (plus de statut « préinscrit » sur `ent_eleve`) | oui / non | **oui** : l'élève naît à la validation ; la réinscription porte sur un élève existant |
| **Q-I5** | Accord des deux parents : demande d'accord **avant** l'engagement (dépôt ou admission) pour l'admission dans le privé, en complément de l'invitation de compte à la validation prévue par D-014 | oui / non | **oui**, règle paramétrée par type de campagne, préréglée « tous les titulaires » pour l'admission dans le privé ; à faire valider juridiquement [S partiel, §4.4] |
| **Q-I6** | Fratries : une démarche pour plusieurs enfants | oui / non | **oui** |
| **Q-I7** | Pièces de santé (PAI, PAP, PPS) avant l'admission (Next.js demande le PAP au rendez-vous) | jamais / paramétrable | **jamais avant l'admission** ; annonce d'un besoin sans détail après admission, contenu au socle 34 |
| **Q-I8** | Capacité : bloquer ou permettre le dépassement avec confirmation | bloquer / confirmer | **confirmer et tracer** (l'existant bloque) |
| **Q-I9** | Impayés à la réinscription (privé) | refus automatique / signalement + retenue humaine / rien | **signalement + retenue décidée par une personne**, jamais de refus automatique |
| **Q-I10** | Liste d'attente : envoi automatique de la proposition de place au suivant | automatique / sur confirmation | **sur confirmation** (un clic), conformément à `CLAUDE.md` (envois soumis à confirmation humaine) |
| **Q-I11** | Motif communiqué en cas de refus | jamais / paramétrable / toujours | **paramétrable**, préréglé « facultatif » ; motif interne toujours obligatoire |
| **Q-I12** | Rendez-vous : créneaux internes + iCal, Google en option désactivée par défaut | — | **oui** (§10.3) |
| **Q-I13** | Entonnoir (contacts, portes ouvertes) dans le socle 3 | oui / socle séparé | **oui** : c'est la porte commerciale du privé (`SOCLES.md` §3) |
| **Q-I14** | Le professeur principal voit-il l'état des réinscriptions de sa classe (sans avis ni données financières) ? | oui / non | **oui, en lecture**, par un droit cochable (`inscriptions.lire` portée « mes classes » à ajouter) |
| **Q-I15** | Durées de conservation (contacts, brouillons, non-aboutis, avis, listes mairies) | — | **à demander au DPO** ; proposition : fin de la campagne suivante pour les non-aboutis |
| **Q-I16** | `ent_foyer_iban_histo` (078) : colonne `iban` créée en clair ; chiffrée depuis ? | — | **vérifier**, chiffrer comme `ent_mandat_sepa_demande` sinon (socle 25) |
| **Q-I17** | Public : ScolIA propose-t-il sa propre inscription en ligne des affectés quand un téléservice national existe ? | ScolIA / national / les deux | **paramètre**, préréglé sur le national quand il existe (import), ScolIA pour le reste |
| **Q-I18** | Démarches réglementaires à vérifier avant les lots I5, I8, I9, I11 : formats Affelnet/SIECLE, certificat de radiation, listes mairies (art. L131-6), accord des parents, non-réinscription | — | les ajouter à la liste des démarches de `INTEROPERABILITE_NATIONALE.md` §12 ; consultation juridique pour l'accord des parents et la non-réinscription |
| **Q-I19** | Paiement en ligne des frais de dossier et de l'acompte | — | dépend de Q5 (`SOCLE2_DOMAINES.md`) ; en attendant : virement, chèque, prélèvement après mandat |
| **Q-I20** | Réinscription « tacite » préréglée pour le public, « explicite » pour le privé | oui / non | **oui** |
| **Q-I21** [personas] | « À ne pas solliciter » (INS-57, R-I8) et confidentialité déclarée à la préinscription (INS-58, R-I9) : à intégrer **dès I0**, avant la fusion de S2-3 ? | maintenant / plus tard | **maintenant** : sans cela, la demande d'accord du second parent peut révéler l'établissement d'un enfant protégé |
| Q-I22 [personas] | Délai d'annulation d'une validation erronée (R-I11) | 0 / 72 h / 7 jours | **72 h**, tant qu'aucun autre socle n'a produit de données |
| Q-I23 [personas] | Langues du parcours famille (R-I4) : lesquelles d'abord, et qui traduit les textes du logiciel ? | — | anglais, arabe, espagnol, portugais, turc, ukrainien, russe, dari ; traduction professionnelle relue, pas de traduction automatique non relue |
| Q-I24 [personas] | Publication programmée des décisions (R-I7) : activée par défaut dans le privé ? | oui / non | **non** (choix de chaque établissement) |
| Q-I25 [personas] | Recours (INS-66) : délai de réponse par défaut ? | — | **15 jours** [R], paramétrable ; à confronter aux contrats de scolarisation |
| Q-I26 [personas] | Ouverture automatique de la campagne `en_cours_annee` de N+1 à la clôture des admissions (R-I5) ? | oui / non | **oui** |

---

## 19. Récapitulatif : faits, hypothèses, sources

**Faits vérifiés [F]** : S2-3 sur `ent-socle2-inscriptions`, intégré dans `ent-integration-lot5`, absent de `ENT` ; migrations 140–145 lues en entier ; écritures directes dans `ent_eleve`, `ent_personne`, `ent_foyer`, `ent_eleve_lien` (`inscriptions.rs`, `imports.rs`) et `ent_affectation` (`inscriptions_reactions.rs`, `imports.rs`) ; rapprochement de responsables par e-mail à la validation, y compris le second parent non vérifié, avec droits du lien codés en dur ; nouveau foyer à chaque validation ; homonyme exact ou INE présent = refus ; validation en une transaction avec événement minimal ; aucune invitation si la personne rattachée a déjà un compte ; aucun consommateur des décisions et étapes (famille non prévenue) ; décision non révisable ; e-mails des réactions envoyés dans la transaction ; campagne fermée réouvrable ; import reclassé hors transaction ; aucune table du socle sous RLS ni en `TenantTx` ; pièces sans fichier ; classe saisie en texte, erreur silencieuse ; pas d'action de lecture ; routes à jeton sans limitation de débit ; tests sans base silencieusement verts ; deux mécanismes de réinscription et événements doublés (018/028/029 contre 140) ; vœux (077) et suivi (078) non reliés à S2-3 ; numéros de migration pris sur les autres branches ; la branche finances réagit à `inscription.validee` ; la branche communication a des créneaux de rendez-vous (192) et des formulaires (193). Next.js : préinscription simple, rendez-vous Google avec reconfirmation J-7, portes ouvertes, 17 types de pièces, ni liste d'attente, ni contrat, ni frais, ni départ, ni réinscription.

**Faits sourcés [S]** (recherches du 2026-11-06, à recouper sur Légifrance avant de coder) :
- inscription dans le privé = acte non usuel, accord des deux parents ; présomption d'accord (art. 372-2 du code civil) pour les actes usuels, qui tombe en cas de désaccord notifié ; qualification au cas par cas ; en cas de désaccord, juge aux affaires familiales : [question écrite au Sénat, 2024](https://www.senat.fr/questions/base/2024/qSEQ240310668.html), [Eduscol — l'autorité parentale](https://eduscol.education.gouv.fr/5112/l-autorite-parentale), [Enseignement catholique de Bretagne (présentation)](https://cdn.netboard.me/42511/files/d0d577617e4a93533c06bc931aae4b58.pdf), [article d'avocat sur l'acte usuel](https://consultation.avocat.fr/blog/stephanie-mantione/article-24479-l-appreciation-de-l-acte-usuel-de-l-autorite-parentale-par-l-administration.html), [question écrite à l'Assemblée nationale (14ᵉ législature)](https://questions.assemblee-nationale.fr/dyn/14/questions/QANR5L14QE66112.pdf) ;
- certificat de radiation demandé au changement d'établissement ; inscription selon la capacité d'accueil ; dérogation par le DASEN : [service-public.gouv.fr F32844](https://www.service-public.gouv.fr/particuliers/vosdroits/F32844?lang=en), [justice.fr](https://www.justice.fr/fiche/changement-college-lycee-cours-annee) ;
- liste des élèves au maire dans les huit jours de la rentrée et mutations mensuelles : [éducation.gouv.fr — établissements privés](https://www.education.gouv.fr/les-etablissements-d-enseignement-scolaire-prives-2942), [Banque des territoires — guide hors contrat](https://www.banquedesterritoires.fr/un-guide-pratique-sur-les-ecoles-hors-contrat-precise-le-role-du-maire), réponses ministérielles à l'Assemblée nationale ;
- délais et motifs de non-réinscription, arrhes : contractuels, variables : [exemple de contrat de scolarisation](https://carcado-saisseval.com/content/uploads/2025/01/contrat-de-scolarisation-Carcado-Lycees-2526.pdf), [autre exemple](https://notredamedubonaccueil.apprentis-auteuil.org/files/2021/01/CONTRAT-DE-SCOLARISATION-2023-2024.pdf).

**Vérification du 2026-11-09** (`docs/VERIF_REGLEMENTAIRE.md` R10, R47-R49, R52) : liste au maire, pièces exigibles (premier degré) et instruction en famille sont [V] ; l'accord exprès des deux parents pour une inscription dans le privé est [V] (réponse ministérielle du 25-4-2024).

**Hypothèses [H] / non vérifié [NV]** : fonctionnement exact d'Affelnet, du téléservice d'inscription et des flux SIECLE pour l'inscription ; impossibilité pour le public de refuser un élève affecté ; principe de ne pas bloquer une inscription pour une pièce manquante dans le public ; article L442-1 (accueil sans distinction) ; mentions du certificat de radiation ; obligations déclaratives du hors contrat (registre) ; signalement d'un élève sorti sans solution ; ULIS, UPE2A et circuits MDPH, CASNAV ; RGPD art. 15 et 22 appliqués aux avis et aux décisions ; pratiques de marché et calendrier des saisons ; efforts des lots.

**Contradictions signalées** :
- S2-3 écrit directement dans les tables des socles 2 et 4, contre D-021 et `02_ELEVE.md` §4 — lots I0, I1.
- `02_ELEVE.md` §7 réserve les migrations 140–149 déjà prises par S2-3 — Q-I1.
- D-014 place l'invitation du second parent à la validation ; l'accord des deux parents pour une admission dans le privé doit exister avant l'engagement — Q-I5.
- `SOCLE2_DOMAINES.md` D3 annonçait `ent_dossier_admission`, `ent_admission_*`, `ent_depart` ; le code a pris `ent_inscription_*` et n'a pas de départ — ce document garde `ent_inscription_*` et ajoute `ent_inscription_depart`.
- `SOCLE2_DOMAINES.md` range INS-10 et INS-11 (régime, options en cours d'année) dans les inscriptions ; `02_ELEVE.md` les met au socle 2 (ELV-10) — ce document suit `02_ELEVE.md`.
- Trois entrées F-008 distinctes existaient avant la fusion du lot 6 ; renumérotées : F-008 (contrôles d'accès annexes), F-009 (contrainte redéfinie par une migration), F-010 (sessions cloud sur Opus).
- Next.js adosse les rendez-vous à Google Agenda, contre l'orientation de souveraineté de `CLAUDE.md` — Q-I12.
