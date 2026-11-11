# Carte des socles de l'ENT ScolIA

Date : 2026-11-06. Branche de rédaction : `ent-carte-socles`. Statut : **socles 1 à 4 fixés par Florian ; socles 5 à 51 = proposition [R], à valider.** Aucun code modifié pour ce document.
Remplace la partie « socle 2 / socle 3 » de l'ancienne `ROADMAP.md`. Dépasse le découpage en 26 domaines de `docs/SOCLE2_DOMAINES.md`, qui reste la **référence des actions métier (identifiants stables DOS-, VS-, FIN-…) et des événements** : on n'y renumérote rien.

**Conventions.** **[F]** fait vérifié dans le dépôt (chemin cité) ; **[H]** hypothèse (réglementaire ou de marché, de mémoire, à vérifier avant le socle) ; **[R]** recommandation (non décidée). « Existant » = code présent, **jamais** « validé pilote ».
Branches citées non fusionnées dans `ENT` au 2026-11-06 [F] : `ent-socle2-inscriptions` (migrations 140–145, écrans), `ent-socle2-cahier` (migration 150, écrans), `ent-socle2-finances` (ouverte, ne contient encore que la fusion de S2-3), `ent-mobile` (applis uniques iOS / Android, `docs/MOBILE.md`).

Sommaire : §1 Ce qu'est un socle · §2 La carte en un tableau · §3 Les socles un par un · §4 Jalons · §5 Ce qui n'est pas un socle · §6 Correspondance avec l'ancien découpage · §7 Hors carte (à décider) · §8 Questions à Florian · §9 Faits, hypothèses, contradictions.

---

## 1. Ce qu'est un socle

Un socle est **un domaine métier entier, conçu en profondeur avant d'être codé, puis livré complet** : tables, actions, droits, événements, écrans web, écrans mobiles quand l'usage est quotidien, imports/exports, tests, documentation. Il n'est pas une liste d'écrans.

**Règles de la carte**
1. **Un socle possède ses données.** Un autre socle ne les écrit jamais directement : il appelle les commandes du propriétaire, ou réagit à ses événements (toile, `SOCLE2_DOMAINES.md` §4).
2. **Un socle vient après ce qu'il consomme en profondeur.** Avant, il peut s'appuyer sur la version existante d'un socle futur (ex. le socle 2 utilise le référentiel déjà livré en S2-2, que le socle 5 approfondit).
3. **Ce qui n'est pas encore refait reste servi par l'existant** (backend actuel et ancienne interface `ui.rs`, figée, D-010). Un établissement pilote n'attend donc pas les 51 socles.
4. **Chaque socle va au-delà du marché** (ligne « Faire mieux ») : c'est là que se joue « mieux que Pronote / École Directe / Charlemagne ».
5. **Public et privé ensemble** (D-019) : chaque socle traite les deux, et ses imports/exports nationaux.

**Cycle d'un socle** (méthode de la roadmap) : **1. réflexion de fond** → document `docs/socles/NN_<NOM>.md` (état des lieux, recherche réglementaire sur les textes officiels, acteurs, modèle de données, actions, droits, événements émis et consommés, écrans, mobile, RGPD, tests, au-delà du marché, questions fermées) ; **2. action** (code) ; **3. test** (automatisé, isolation, CI) ; puis mise à jour de `PROJECT_STATUS.md`.

**Un socle est terminé quand** : chaque action de son périmètre existe côté serveur avec son droit dans le catalogue (D-017) ; tous ses faits inter-domaines passent par l'outbox, en transaction ; ses écrans Leptos remplacent ceux de `ui.rs` pour son domaine ; ses tests d'isolation entre établissements sont dans la matrice ; ses données ont une catégorie de conservation ; ses indicateurs sont exposés au pilotage ; ses actions sont décrites pour l'assistant (identifiant, effets, confirmation). **[R, proposition du 2026-11-06]** Précisé par le cœur IA : ses lectures et commandes sont déclarées au registre `ent-ia` selon le *contrat IA* (`docs/socles/IA_COEUR.md` §5 : droit D-017, portée, sensibilité, volume ; pour les commandes, simulation, niveau de confirmation, réversibilité, texte d'aperçu), avec les tests de garde (équivalence écran / IA, isolation, simulation sans écriture) et le jeu d'évaluation du socle.

---

## 2. La carte en un tableau

État de l'existant : **I** implémenté côté serveur (souvent avec l'ancienne interface), **P** partiel, **E** embryonnaire, **A** absent ; **L** écrans Leptos déjà présents ; **B** travail sur une branche non fusionnée.

| N° | Socle | Étage | Dépend de | Existant |
|---|---|---|---|---|
| 1 | Sécurité | A · Cœur | — | **fait** (L) |
| 2 | L'élève | A · Cœur | 1 (+ référentiel existant) | P, L |
| 3 | Inscriptions et réinscriptions | A · Cœur | 2 (+ 4, 7 existants) | P, B |
| 4 | Dossier administratif (foyers, responsables, coordonnées, identifiants officiels) | A · Cœur | 2, 3 | I, L |
| 5 | Structure de l'établissement et année scolaire | B · Cadre | 1 | I, L |
| 6 | Personnels, services et équipes | B · Cadre | 5 | P |
| 7 | Documents, pièces et signatures | B · Cadre | 2, 4 | P |
| 8 | Notifications et diffusion | B · Cadre | 4 | P |
| 9 | Interopérabilité nationale et identité fédérée | B · Cadre | 2, 4, 5, 6 | P |
| 10 | Emploi du temps | C · Journée | 5, 6 | I, L |
| 11 | Applications mobiles (coquille commune) | C · Journée | 1, 8, 10 | P, B |
| 12 | Appel et assiduité | C · Journée | 2, 4, 8, 10, 11 | I, L |
| 13 | Discipline et suivi éducatif | C · Journée | 12, 7 | I, L |
| 14 | Accueil, sorties et sûreté | C · Journée | 4, 10, 12 | P |
| 15 | Absences des personnels et remplacements | C · Journée | 6, 10, 8 | P |
| 16 | Messagerie et conversations | D · Échanger | 4, 6, 7, 8 | I, L |
| 17 | Informations, annonces et campagnes | D · Échanger | 8, 16 | P |
| 18 | Formulaires, autorisations et consentements | D · Échanger | 4, 7, 8 | P |
| 19 | Agenda, réunions et rendez-vous | D · Échanger | 6, 10, 16 | E |
| 20 | Cahier de textes et travail personnel | E · Apprendre | 10, 7 | E, B |
| 21 | Évaluations et notes | E · Apprendre | 6, 10, 20 | I |
| 22 | Compétences et livrets | E · Apprendre | 21, 9 | P |
| 23 | Bulletins, conseils de classe et passage | E · Apprendre | 21, 22, 12, 13 | P |
| 24 | Facturation | F · Argent | 2, 3, 4, 7 | I, B |
| 25 | Paiements, prélèvements et recouvrement | F · Argent | 24 | I |
| 26 | Bourses et aides financières | F · Argent | 24, 25, 18 | P |
| 27 | Conformité RGPD, conservation et archives | G · Premier établissement | tous les précédents | E |
| 28 | Mise en service, reprise de données et support | G · Premier établissement | 2–9 | E |
| 29 | Portails, « Aujourd'hui » et à-faire unifié | G · Premier établissement | 8, 12–26 | E, L |
| 30 | Assistant IA et « chat first » | H · Au-dessus des domaines | 16, 27, 29 | E |
| 31 | Automatisations et règles de l'établissement | H · Au-dessus des domaines | 8, 30 | E |
| 32 | Espaces de travail et bureautique | H · Au-dessus des domaines | 7, 20 | A |
| 33 | Visioconférence et continuité pédagogique | H · Au-dessus des domaines | 16, 19 | A |
| 34 | Santé et infirmerie | I · Chaque élève | 2, 4, 7, 12 | I |
| 35 | Besoins éducatifs particuliers et inclusion | I · Chaque élève | 34, 6, 23 | E |
| 36 | Suivi social, psychologique et protection de l'enfance | I · Chaque élève | 2, 4, 19 | P |
| 37 | Orientation et parcours | I · Chaque élève | 23, 19 | P |
| 38 | Examens et certifications | I · Chaque élève | 10, 21, 35 | A |
| 39 | Stages, PFMP et entreprises | I · Chaque élève | 7, 12 | I |
| 40 | Restauration | J · Services | 12, 24, 34 | I |
| 41 | Internat | J · Services | 12, 24, 34 | I |
| 42 | Sorties et voyages scolaires | J · Services | 18, 24, 34, 6 | A |
| 43 | Activités périscolaires et transports | J · Services | 12, 18, 24 | E |
| 44 | Ressources matérielles, prêts et maintenance | J · Services | 5, 10 | E |
| 45 | CDI et ressources documentaires | J · Services | 9, 44 | A |
| 46 | Instances et vie démocratique | J · Services | 4, 6, 18 | A |
| 47 | Pilotage et indicateurs | J · Services | tous | E |
| 48 | Comptabilité, budget et achats | K · Organisme | 24, 25, 26 | E |
| 49 | Paie et administration du personnel | K · Organisme | 6, 15 | A |
| 50 | Premier degré (écoles) | L · Extensions | 2–29 | E |
| 51 | Post-bac et apprentissage | L · Extensions | 2–29, 39 | A |

**51 socles.** Fusions possibles sans perte si l'on veut viser 50 : 32 + 33 (travail collaboratif et visio), ou 43 + 44.

---

## 3. Les socles un par un

### Étage A — Le cœur : l'élève, son entrée, ceux qui en répondent

#### 1. Sécurité — **fait**
- **Contient** : authentification (passkey obligatoire à terme D-009, session web en cookie `HttpOnly` et session mobile longue D-011, récupération en libre-service D-018, SSO OIDC durci) ; moteur de rôles paramétrables (catalogue d'actions × portées, cumul, délégations, D-017) ; isolation des établissements (sous-domaine D-012, type `Tenant`, RLS pilote) ; chiffrement des champs sensibles ; journal en ajout seul ; anti-abus ; URL pré-signées.
- **Ne contient pas** : fédération ÉduConnect / guichet agents (→ 9) ; outils RGPD (→ 27).
- **Apporte** : à chaque socle, le catalogue où déclarer ses actions, la matrice d'isolation où ajouter ses routes, le chiffrement pour ses champs sensibles.
- **Existant [F]** : `ent-auth`, `ent-oidc`, migrations 084–100, `docs/SOCLE1_SECURITE.md` (suivi lot par lot ; restes à vérifier dans ce document, ex. essai du cookie dans un vrai navigateur).

#### 1 bis. Cœur IA (transverse, dès le socle 2) — **proposition [R], non validée**
- **Pourquoi ici** : D-024 place l'IA « au cœur, pas à la fin ». Ce qui doit exister tôt n'est pas l'assistant conversationnel (il reste au socle 30) mais **le contrat** que chaque socle remplit au moment où il est codé ; sans lui, chaque socle déjà livré serait à reprendre. Ce n'est pas un socle numéroté (il ne possède aucune donnée métier) : il se construit **en parallèle du socle 2**, comme le socle 5. Aucun socle n'est renuméroté ni déplacé.
- **Contient** : registre statique des capacités (lectures et commandes déclarées par les socles) ; exécution sous l'identité et les droits effectifs de la personne (D-017), sans droit propre ; brouillons d'action avec simulation, confirmation humaine hors modèle et re-vérification des droits ; journal IA en ajout seul ; fiche de contexte minimisée (classes de sensibilité, pseudonymisation, données non fiables encadrées) ; fournisseur nul et simulé, puis adaptateurs de fournisseurs souverains par établissement, sans repli ; quotas et coûts ; actions `ia.*` ; rebranchement du copilote existant (portée à la source, constat RF-04 b).
- **Ne contient pas** : la conversation, les familles et les élèves, les enchaînements (→ 30) ; les règles automatiques (→ 31) ; la recherche sémantique (→ 30 au plus tôt).
- **Dépend de** : 1. **Apporte** : à chaque socle, le contrat IA de ses critères de fin (§1) ; au socle 2, les premiers usages (synthèse de fiche, recherche structurée, qualité des données expliquée).
- **Étapes** : IA-0 fondations et IA-1 « socle 2 décrit » **sans aucun modèle de langage** ; premier fournisseur en labo après revue contractuelle (Q-IA2) ; premiers usages réels avec les socles 3–4, AIPD faite. Lots IA-L0 à IA-L10.
- **Existant [F]** : `ent-db/src/copilote.rs` (déterministe, sans modèle), `docs/AI_PLATFORM.md` ; Next.js utilise Mistral (évaluation franche : `IA_COEUR.md` §2.4).
- **Conception** : `docs/socles/IA_COEUR.md`.

#### 2. L'élève
- **Contient** : identité et état civil historisés (noms de naissance et d'usage, prénoms, sexe, naissance, nationalité) ; **cycle de vie** explicite dans l'établissement (prévu → inscrit → actif → sorti → archivé) ; **scolarité datée** année par année : classe et groupes (affectations), options, MEF, régime (externe, demi-pension, interne), statuts (redoublant, nouvel arrivant, auditeur) ; compte élève et invitation ; photo et trombinoscope ; champs personnalisés de l'établissement ; **fiche 360°** (coquille où chaque socle branche sa section) ; **chronologie** typée filtrée par les droits ; **point d'entrée des actions** (le catalogue proposé selon les droits du lecteur) ; recherche d'élève, homonymes, **fusion réversible de deux dossiers élèves** ; passage à la majorité et recalcul des droits.
- **Ne contient pas** : responsables, foyers, coordonnées, identifiants officiels (→ 4) ; admission, inscription, départ (→ 3) ; santé (→ 34) ; notes, absences, factures (chaque socle alimente la fiche par la chronologie).
- **Dépend de** : 1 ; utilise le référentiel existant (années, classes, groupes : S2-2) en attendant le socle 5.
- **Apporte** : un `eleve_id` stable ; la fonction de référence « scolarité de X à la date D » (droits du professeur, appel, notes, facturation au prorata) ; les événements `eleve.*` et `scolarite.*` (changement de classe, de régime, d'option) qui déclenchent restauration, internat, facturation.
- **Existant [F]** : `ent_eleve` (001), `ent_eleve_periode` (006), scolarité et options annuelles (018), `ent_eleve_compte` (014), invitation (039), informations complémentaires (072) ; `domain-eleve/src/{dossier,homonyme,trombi,continuum}.rs` ; `ent-db/src/perimetre.rs` ; fiche 360° et chronologie Leptos `/web/eleves/{id}` (S2-1). Manquent : cycle de vie explicite, fusion de dossiers élèves (plus de 100 tables portent `eleve_id`), majorité automatique (Q10), recherche.
- **Place** : fixée par Florian. Tout le modèle pointe sur l'élève ; le fixer d'abord évite de refaire les 50 autres.
- **Faire mieux** : tout est lisible **à une date** (« qui était dans quelle classe, avec quel régime, le 12 mars ? ») ; la fiche explique ses alertes ; une seule liste d'actions contextuelles, identique web et mobile ; fusion réversible.

#### 3. Inscriptions et réinscriptions
- **Contient** : **entonnoir de recrutement** (demande d'information, visite, portes ouvertes, candidature) ; campagnes d'admission et de réinscription (niveaux, capacités, calendrier, pièces) ; préinscription en ligne vérifiée avant affichage (D-014) ; instruction (étapes paramétrables, entretien, commission, liste d'attente, relances) ; décision motivée et courrier ; **validation de l'inscription** (élève actif, affectation, régime, invitation du second parent, contrat de scolarisation à signer, frais d'inscription ou acompte demandés à la facturation, reprise de l'IBAN masqué D-014) ; réinscription « en 3 gestes » ; non-réinscription ; départ, radiation, exeat, changement d'établissement ; effectifs prévisionnels de la rentrée.
- **Ne contient pas** : la gestion courante des responsables (→ 4) ; factures et prélèvements (→ 24, 25 : appelés par commande) ; l'import SIECLE en masse (→ 9).
- **Dépend de** : 2 ; s'appuie sur le modèle « personne » déjà livré (S2-1) et sur les documents existants, que les socles 4 et 7 approfondissent.
- **Apporte** : `inscription.validee` et `scolarite.depart_*`, déclencheurs de l'affectation, des comptes, des documents et de la facturation ; les effectifs prévus au socle 5 pour préparer l'année suivante.
- **Existant [F]** : sur `ENT` : réinscription (028, 029), suivi d'inscription, vœux (077), IBAN d'inscription (078). Sur `ent-socle2-inscriptions` (non fusionnée) : campagnes, dossier d'inscription, préinscription vérifiée, demande de mandat SEPA, lots d'import, invitation (140–145) et écrans Leptos (préinscription, instruction, réinscription).
- **Place** : fixée par Florian : c'est la porte d'entrée de l'élève, et la porte commerciale du privé. **Point d'attention [R]** : 3 vient avant 4 alors qu'il crée des responsables. Pour éviter une reprise, le socle 3 n'écrit les personnes et les liens **que par les commandes du domaine « personnes » existant** ; le socle 4 les approfondit sans changer ce contrat. (Inverser 3 et 4 réduirait encore le risque ; l'ordre fixé est conservé.)
- **Faire mieux** : suivi des prospects comme un vrai outil de recrutement ; tout pré-rempli avec consentement ; capacité par niveau en temps réel ; relances sobres et automatiques.
- **Conception** (2026-11-06, non décidée) : `docs/socles/03_INSCRIPTIONS.md`. La règle de contournement ci-dessus y est précisée (§5.2 : aucune personne créée avant la validation ; une seule commande du socle 4, `integrer_declaration`, à la validation).

#### 4. Dossier administratif
- **Contient** : personnes (responsables, contacts, tiers) distinctes des comptes (D-020) ; foyers et adresses ; liens élève–personne (autorité parentale, garde exclusive ou alternée, droits fins, dates, **suspension sur décision de justice** avec pièce) ; **parents séparés** natifs (fiches et envois distincts, un e-mail par parent, D-014) ; fratries ; coordonnées historisées (adresses, téléphones, e-mails, préférences de contact) et **changements proposés par la famille, validés par l'établissement** ; contacts d'urgence et personnes autorisées à venir chercher l'élève ; payeurs et répartition ; **identifiants officiels** (INE, identifiants SIECLE / ONDE, rattachement ÉduConnect) ; données demandées par les remontées nationales (profession et catégorie socioprofessionnelle des responsables [H]) ; pièces administratives (livret de famille, jugement, justificatif de domicile) via le socle 7 ; **fiche de rentrée annuelle** vérifiée en ligne ; fusion de personnes.
- **Ne contient pas** : coordonnées bancaires et mandats (→ 25) ; santé (→ 34) ; comptes et droits de connexion (→ 1).
- **Dépend de** : 2, 3.
- **Apporte** : à toute l'application, la réponse à « à qui écrire, qui décide, qui paie, qui peut venir chercher l'élève » (notifications, messagerie, facturation, accueil, sorties) ; les identifiants pour les imports et exports (9).
- **Existant [F]** : `ent_foyer`, `ent_eleve_lien` (007), `ent_personne` et reprise (120), liens d'autorité (121), contacts d'urgence (122), fusion de personnes (123), `domain-eleve/src/ine.rs`, 21 routes `/dossier/*`, écrans Leptos de la fiche (S2-1). Reste listé en `SOCLE2_DOMAINES.md` §6.2 (mise à jour des coordonnées par le parent, séparation d'un foyer en un geste, invitation depuis une personne sans compte, majorité).
- **Place** : fixée par Florian ; clôt le cœur. Après 4, on sait qui est l'élève, comment il est entré et qui en répond.
- **Faire mieux** : fiche de rentrée sans ressaisie ; proposition « mettre aussi à jour chez l'autre établissement » avec consentement (D-013) ; alertes de cohérence (aucune autorité active, parent séparé sans e-mail, payeur sans foyer).
- **Conception** (2026-11-06, non décidée) : `docs/socles/04_ADMINISTRATIF.md`. Point à corriger ici après validation (Q-A2) : les identifiants de l'**élève** (INE, SIECLE, ONDE) relèvent du socle 2 (`02_ELEVE.md` §7.3) ; le socle 4 garde ceux des **personnes**.

### Étage B — Le cadre commun

#### 5. Structure de l'établissement et année scolaire
- **Contient** : établissement, sites, bâtiments et salles (référentiel) ; années scolaires (préparation, courante, clôturée) ; périodes ; calendrier (vacances par zone, fermetures, journées banalisées) ; grille horaire ; niveaux, MEF, formations ; classes, groupes, ateliers ; matières (codes nationaux) ; **bascule d'année** (préparation, reconduction, montée, ouverture, clôture) ; paramètres d'établissement.
- **Ne contient pas** : professeurs et services d'enseignement (→ 6) ; créneaux et séances (→ 10) ; affectations des élèves (→ 2, posées sur cette structure) ; réservations de salles (→ 44).
- **Dépend de** : 1 ; déjà utilisé tel quel par 2 à 4.
- **Apporte** : le « où » et le « quand » de tous les socles.
- **Existant [F]** : largement fait (S2-2) : 092, 110–112, ~30 routes `/referentiel/*`, écrans `/web/referentiel/*`. Reste : coupure de la lecture du texte `classe` (Q14), périodes par niveau, aperçu serveur de l'ouverture d'année (`SOCLE2_DOMAINES.md` §6.3).
- **Place** : juste après le cœur parce que l'essentiel existe et que le cœur s'en sert déjà ; doit être terminé avant l'emploi du temps, l'appel et les notes. [R] Meilleur candidat à mener **en parallèle** de 2 à 4.
- **Faire mieux** : bascule d'année guidée avec aperçu des effets ; calendrier officiel importé ; rien ne se perd d'une année sur l'autre.
- **Conception** (2026-11-06, non décidée) : `docs/socles/05_STRUCTURE.md`. Points à corriger ici après validation : les **affectations** des élèves (table et commandes) relèvent du socle 5, le socle 2 les lit (Q-S1) ; la génération des séances datées part au socle 10 (Q-S11) ; le professeur principal au socle 6 (Q-S4).

#### 6. Personnels, services et équipes
- **Contient** : fiche personnel (fonctions, disciplines, quotité, statut public / privé / contractuel, établissements) ; arrivées et départs ; annuaire interne ; **services d'enseignement** (professeur × matière × classe ou groupe × année) ; professeur principal ; équipes pédagogiques et éducatives ; référents (décrochage, harcèlement, numérique…) ; responsables de groupes et d'ateliers ; délégations datées (remplaçant, intérim) ; rattachement des comptes aux rôles (moteur du socle 1).
- **Ne contient pas** : absences et remplacements (→ 15) ; contrats de travail et paie (→ 49) ; emploi du temps (→ 10).
- **Dépend de** : 1, 5.
- **Apporte** : la portée « mes classes » des droits ; les destinataires « équipe de la classe » ; les membres des conseils de classe, les surveillants d'examen, les encadrants de sortie.
- **Existant [F]** : `ent_membership`, rôles paramétrables (098), `ent_service_enseignement` (092), responsables de groupes (S2-2), absences des personnels (080). Pas de fiche personnel.
- **Place** : avant l'emploi du temps (un créneau se rattache à un service) et avant l'appel et les notes (les droits du professeur en découlent).
- **Faire mieux** : délégations qui expirent seules ; équipes calculées, jamais saisies deux fois ; import STS-Web (9).
- **Conception** (2026-11-06, non décidée) : `docs/socles/06_PERSONNELS.md`. Défauts relevés à corriger en premier (lot P0) : aucun départ possible (accès sans état ni date de fin), dernier administrateur non protégé, mot de passe temporaire renvoyé en clair, un seul profil par compte et par établissement (professeur-parent impossible).

#### 7. Documents, pièces et signatures
- **Contient** : stockage objet privé (S3 compatible, Scaleway prioritaire) ; URL pré-signées courtes ; contrôle du type réel, limites, analyse antivirale [R] ; GED de l'élève et des personnes (catégories, versions, durée de conservation) ; pièces demandées / fournies / validées ; documents à signer avec preuve (niveau de signature à décider [H]) ; gabarits et publipostage ; moteur de conversion maison (D-005) ; **coffre-fort famille** (factures, bulletins, attestations) ; certificat de scolarité en libre-service.
- **Ne contient pas** : bureautique en ligne (→ 32) ; contenus pédagogiques (→ 20) ; diffusion des documents (→ 8, 17).
- **Dépend de** : 1, 2, 4.
- **Apporte** : toutes les pièces de tous les socles ; les signatures des inscriptions, PAI, sorties, conventions de stage, autorisations.
- **Existant [F]** : `ent-object` (disque, mémoire, S3 simulé), documents de l'élève (016), autorisations d'accès aux documents, documents à signer (070), pièces de messagerie (038), PDF dédiés (`bulletin_pdf.rs`, `facturation_pdf.rs`, `fiche_sante_pdf.rs`, `psy_pdf.rs`), éditions à modèles (081).
- **Place** : 3 et 4 produisent les premières pièces (contrat de scolarisation, jugement, justificatifs) ; tous les socles suivants en ont besoin.
- **Faire mieux** : conversion sans quitter nos serveurs (RGPD) ; preuve de signature consultable par la famille ; certificat de scolarité sans passer par le secrétariat.
- **Conception** (2026-11-07, non décidée) : `docs/socles/07_DOCUMENTS.md` (brique `documents` de l'architecture D-026, service publié aux autres briques ; quatre styles maison déclinés pour chaque nature ; niveaux de signature S0–S2, guichet, papier ; coffre ; pièces avec quarantaine, antivirus et désarmement ; conservation validée par le DPO ; lots D0–D15 ; 30 questions). Défauts à corriger en premier (lot D0) : téléchargement de documents bureautiques qui renvoie un stub, double signature concurrente possible, lien de fichier qui révèle la clé de stockage.

#### 8. Notifications et diffusion
- **Contient** : moteur « événement → destinataires → gabarit → canaux » ; préférences par personne ; canaux in-app, e-mail (fournisseur à choisir), SMS (OVH, D-004), push ; regroupement, heures calmes, plafond et coût des SMS ; accusés de remise ; **« pourquoi ai-je reçu ce message ? »** (chaîne de causalité) ; file de validation humaine des envois sensibles.
- **Ne contient pas** : conversations (→ 16) ; annonces et campagnes rédigées (→ 17).
- **Dépend de** : 4 (destinataires) ; l'outbox existante.
- **Apporte** : chaque socle publie des faits ; le socle 8 décide qui est prévenu, comment et quand, en revérifiant les droits au moment de l'envoi.
- **Existant [F]** : `ent_notification` et consommateur `d0.notifications` (102, `ent-db/src/reactions.rs`, 3 réactions) ; `ent-mail` (SMTP), `ent-sms` (simulé), `ent-push` ; files d'envoi (055, 057, 058).
- **Place** : avant la journée scolaire : une absence constatée doit prévenir la famille, une seule fois, sur le bon canal.
- **Faire mieux** : sobriété (pas de doublon, regroupement) ; explication de chaque envoi ; certains avis non désactivables (sécurité, absence non justifiée).
- **Conception** (2026-11-07, non décidée) : `docs/socles/08_NOTIFICATIONS.md` (brique `notifications` de l'architecture D-026 : les briques déclarent leurs natures d'avis — urgence U0–U3, sensibilité, règle de destinataires du socle 4, plancher non désactivable — et notifient par événement ou par un service publié dans leur transaction ; destinataires = personnes, y compris sans compte ; enveloppe neutre hors de l'ENT ; cascade de canaux et « lu partout » ; délai de confirmation et correction d'une absence ; preuves, « pourquoi ai-je reçu », notification effective pour les socles 7 et 13 ; diffusions avec aperçu, coût, confirmation N3 et délai d'annulation ; urgence à modèles ; lots N0–N14 ; 31 questions). Défauts à corriger en premier (lot N0) : adresse et objet en clair dans l'événement `mail.envoye`, sanction et carnet détaillés par SMS, courrier du psychologue en pièce jointe, avis d'infirmerie perdus hors de l'ENT, aucun arrêt des envois au décès d'un élève, ancien chemin d'envoi qui ignore les droits du lien. **Précision proposée (Q-N2)** : la mécanique des envois groupés (aperçu, coût, confirmation, exécution, suivi) est au socle 8 ; le socle 17 garde le contenu éditorial et ses audiences.

- **Passe personas** (2026-11-08, non décidée) : `docs/socles/PERSONAS_08.md` (71 manques, 3 bloquants : fermeture décidée la veille qui arriverait après le car, aperçus de diffusion qui désignent une famille protégée, aucune garde contre un avis famille sur une information préoccupante) ; ajouts [personas] dans `08_NOTIFICATIONS.md` (NTF-119 à 164, R-N50 à R-N72, CN-51 à CN-80).

#### 9. Interopérabilité nationale et identité fédérée
- **Contient** : cadre d'import commun (lots, prévisualisation, différences, rejets ligne à ligne, annulation d'un lot) ; imports SIECLE (élèves, responsables, structures), STS-Web (personnels, services), nomenclatures BCN ; connexion ÉduConnect (élèves, responsables) et guichet agents (personnels) ; GAR (annuaire et identité ; **les affectations se font dans la console du GAR**, correction du 2026-11-08, `socles/09_INTEROP.md` §11) ; conformité au schéma directeur des ENT ; **traces d'export** communes ; registre des connecteurs.
- **Ne contient pas** : les exports métier eux-mêmes, qui restent dans leur socle et utilisent le cadre de traces : LSU (22), LSL (23), Affelnet / Parcoursup (37), DNB (38), comptabilité (48), ONDE (50).
- **Dépend de** : 2, 4, 5, 6 (cibles des imports) ; 1 (fédération).
- **Apporte** : l'entrée sur le marché public (D-019) et la fin de la ressaisie.
- **Existant [F]** : étude `docs/INTEROPERABILITE_NATIONALE.md` (lots proposés) ; exports CSV `domain-eleve/src/{lsun,lsl,notanet,onde,parcoursup}_export.rs` ; import EDT (060) ; import MEF (S2-2) ; lots d'import (144, branche `ent-socle2-inscriptions`) ; `ent-oidc` comme base de fédération.
- **Place** : D-019 impose le public dès le départ ; les imports arrivent dès que leurs cibles (2, 4, 5, 6) existent, avant qu'un établissement public ne saisisse quoi que ce soit à la main.
- **Faire mieux** : imports « sans surprise » (A18) ; jamais d'écrasement silencieux d'une donnée corrigée par l'établissement.
- **Conception** (2026-11-08, non décidée) : `docs/socles/09_INTEROP.md` (moteur d'échanges à connecteurs déclarés ; tout ce qui entre devient des propositions dans le domaine propriétaire ; **mémoire de la source et différence à trois voies** : une correction locale n'est jamais écrasée ni redemandée ; garde-fous d'identité du fichier (UAI, année, ancienneté) et de volume ; exports pré-contrôlés, figés, tracés, bouclés par les comptes rendus nationaux ; guichet central de fédération `connexion.scolia.fr`, rattachement par jointure ou code, jamais par e-mail ; ScolIA fournisseur d'identité pour le GAR ; 57 fiches de flux ; lots X0–X19 ; parcours minimal vendable X0–X7 **sans aucun accès officiel** ; 16 démarches hors code, qui forment le chemin critique ; 32 questions). Migrations 3900–3999.
- **Passe personas** (2026-11-08, non décidée) : `docs/socles/PERSONAS_09.md` (16 personas, 92 manques M-405 à M-496, 7 bloquants en 4 points : première année d'un lycée privé qui quitte Charlemagne — LSU, LSL, Parcoursup et BEE passent par un logiciel validé, Parcoursup exige les bulletins de N-1 ; rattachement ÉduConnect de rentrée qui demanderait 1 300 validations à la main ; dans le public, ScolIA souvent raccordé à l'ENT de la collectivité plutôt qu'ENT lui-même ; reprise des mandats SEPA) ; ajouts [ajout personas] dans `09_INTEROP.md` (ITP-119 à 154, R-X17 à R-X28, CX-47 à CX-65, 19 flux, lots X7 bis et X8 bis, Q-X33 à Q-X44). Note de cohérence (C10) : « sans aucun accès officiel » ci-dessus vaut pour le palier tableurs ; SIECLE et STS-web exigent des jeux d'or réels anonymisés (`09_INTEROP.md` §25.2).

### Étage C — La journée de l'établissement

#### 10. Emploi du temps
- **Contient** : créneaux, semaines A/B, **séances datées** matérialisées ; exceptions (annulation, déplacement, changement de salle) ; imports depuis les logiciels d'emploi du temps ; contraintes ; consultation par professeur, classe, élève, salle ; iCal ; **annulation en masse** (grève, intempéries) avec effets automatiques ; changements annoncés aux familles.
- **Ne contient pas** : ~~la fabrication automatique de l'emploi du temps (solveur, §7)~~ (D-025 : dans le périmètre ; la conception du 2026-11-09 la place **dans ce socle**, lots T6 à T8b, Q-EDT1 à valider) ; l'absence d'un personnel, la décision de remplacer et la recherche du remplaçant (→ 15 ; le socle 10 garde la modification de séance) ; les réservations (→ 44).
- **Dépend de** : 5, 6.
- **Apporte** : la séance datée, support de l'appel, du cahier de textes, des remplacements, des examens, des sorties.
- **Existant [F]** : 021, 042, 048, 060, 083 ; séances datées (110) ; `ent-api/src/ical.rs` ; module Leptos `edt`.
- **Place** : première brique de la journée ; tout ce qui suit se rattache à une séance.
- **Faire mieux** : réaction en deux minutes à une fermeture (familles, cantine, études prévenues ensemble, A14).
- **Conception** (2026-11-09, rattrapage, non décidée) : `docs/socles/10_EMPLOI_DU_TEMPS.md` — plan versionné / séance datée stable / modification ; génération automatique déterministe et explicable (D-025) ; 86 actions EDT-100 à EDT-198, 63 cas limites, lots T0 à T13 ; **lot T0 de correctifs urgents** sur l'existant (cahier de textes effacé à la régénération, EDT nominatif ouvert à tout professeur contre D-022, motifs d'absence visibles des professeurs, aucune RLS). Remontée hebdomadaire du remplacement de courte durée (arrêté du 14/08/2023) à prévoir avec les socles 9 et 15. Plage de migrations 4000–4099 (Q-B2) ou 440–459.
- **Passe personas** (2026-11-09, non décidée) : `docs/socles/PERSONAS_10.md` (16 personas, 82 manques M-497 à M-578 : 3 bloquants, 50 importants, 20 confort, 9 différenciants ; parité vérifiée sur les pages de Charlemagne Emploi du temps et d'EDT) ; **à trancher avant T1 et T8a** : barrettes et répartition des élèves depuis leurs choix (Q-EDT17), explications typées et filtrées (R-E31), versions temporaires avec retour au plan (Q-EDT18) ; ajouts marqués « ajout personas » dans la conception : AESH, réparation, lycée professionnel (périodes, PFMP), ateliers à inscription (matrice EDT-15), EDT-200 à EDT-238, R-E21 à R-E39, CL-64 à CL-82, lots T14 à T16, S-90 à S-99.

#### 11. Applications mobiles (coquille commune)
- **Contient** : une appli iOS (SwiftUI) et une Android (Compose) pour tous les rôles (D-003) : connexion par passkey native, session longue révocable, déverrouillage biométrique, rattachement à l'établissement par lien ou QR code, plusieurs établissements et plusieurs enfants, push, hors ligne (lecture, puis saisie de l'appel), associations de domaine `scolia.fr`, publication dans les stores, parité stricte des droits avec l'API.
- **Ne contient pas** : les écrans métier : **chaque socle suivant livre ses écrans mobiles** quand l'usage est quotidien ; les tâches complexes restent sur le web.
- **Dépend de** : 1, 8 (push), 10.
- **Existant [F]** : branche `ent-mobile` (non fusionnée) : une appli par plateforme, cœurs Kotlin et Swift testés, interfaces non compilées dans le conteneur, API manquantes M1–M12 listées dans `docs/MOBILE.md` de la branche.
- **Place** : juste avant l'appel, premier usage mobile du professeur et condition de vente.
- **Faire mieux** : hors ligne réel (salle sans réseau, sortie) ; une seule appli pour tous, rôles cumulés (parent et professeur).

#### 12. Appel et assiduité
- **Contient** : appel par séance (web, mobile, hors ligne), **pré-rempli** (absences annoncées, dispenses, stage, sortie, examen) ; vérification des appels non faits et relances ; absences à la séance et **demi-journées dérivées** ; retards et billets d'entrée ; déclarations par l'accueil et la famille ; justificatifs, motifs, décisions ; dispenses (EPS…) ; compteurs et seuils ; **signalement d'assiduité** à l'autorité académique ; relevés et courriers ; statistiques d'assiduité.
- **Ne contient pas** : incidents et sanctions (→ 13) ; absences des personnels (→ 15) ; appel du soir d'internat (→ 41) ; appel d'une activité (→ 43, qui réutilise le même composant).
- **Dépend de** : 2, 4, 6, 8, 10, 11.
- **Existant [F]** : le plus avancé : 003, 004, 009–013, 035, 043, 067, 087, 130–132 ; `ent-db/src/assiduite.rs` ; écrans Leptos appel, journée, absences, vie scolaire (S2-4). Règles réglementaires [H] non vérifiées sur les textes officiels.
- **Place** : obligation légale de chaque cours ; condition de vente n° 1.
- **Faire mieux** : décompte réglementaire automatique et dossier de signalement pré-rempli (A5) ; moins d'appels téléphoniques à la vie scolaire (A1, A2).
- **Conception** (2026-11-11, non décidée, recommandations appliquées par défaut D-031) : `docs/socles/12_ASSIDUITE.md` — trois objets (constat à la séance, situation connue qui pré-remplit, absence en épisode), réglementaire vérifié (R131-5 à R131-7, L131-8, R312-2, R531-31 ; circulaire 2014-159 qui abroge celle de 2011 ; pas de définition légale de la demi-journée), appel web, mobile et hors ligne, règle d'assiduité versionnée, dossier annuel (R131-6) et signalement pré-rempli selon un profil académique ; 152 actions AS-100 à AS-251, 72 cas limites, 40 règles, lots AS0 à AS16 ; **lot AS0 de correctifs urgents** (professeur qui déclare, efface ou retarde n'importe quel élève ; une absence par séance ; seuil annuel de 10 codé en dur qui écrit aux familles ; RLS inopérante). Plage de migrations **2240–2259**.
- **Passe personas** (2026-11-11) : `docs/socles/PERSONAS_12.md` (16 personas dont attaquants et DPO, 72 manques M-2592 à M-2663) ; quatre bloquants tranchés : autre parent informé (R-AS33), élève exclu de cours suivi (R-AS37), cache hors ligne protégé (R-AS38), doublon avec Charlemagne (décision de Florian).

#### 13. Discipline et suivi éducatif
- **Contient** : incidents (faits, impliqués, témoins, lieu) ; punitions (retenues planifiées avec salle et surveillant) ; sanctions selon l'échelle réglementaire [H] ; procédures disciplinaires et conseils de discipline ; commissions éducatives ; mesures de responsabilisation ; effacement planifié ; suivi éducatif (observations, entretiens CPE, engagements) ; alertes CPE ; protocole harcèlement (signalement, suivi) [H].
- **Ne contient pas** : carnet de correspondance (→ 16) ; informations préoccupantes (→ 36).
- **Dépend de** : 12 ; 7 (convocations, courriers).
- **Existant [F]** : 019, 054, 061, 069, 133, 134 ; `ent-db/src/discipline_vs.rs` ; écran Leptos `/web/discipline`.
- **Place** : deuxième pilier de la vie scolaire, mêmes acteurs que l'appel.
- **Faire mieux** : procédure contradictoire guidée, délais calculés, effacement sans oubli ; détection précoce explicable (A6, avec le socle 31).
- **Conception** (2026-11-11, non décidée) : `docs/socles/13_DISCIPLINE.md` — incident unique à plusieurs personnes et rôles, une suite par élève ; punition ≠ procédure ≠ sanction ; délais calculés par le serveur (2 jours ouvrables, 5 jours, appel 8 jours) ; droit de se taire (décret 2025-609) ; procédure obligatoire en cas de harcèlement, conseil obligatoire en cas d'arme ; privé sous contrat : discipline contractuelle, garanties du public par défaut ; effacement R511-13 IV codé et prouvé ; 46 règles vérifiées (RG-01 à RG-46), actions DIS-100 et suivantes, cas limites CD-01 et suivants, lots D0 à D14, migrations **2260–2279**. **Lot D0 de correctifs urgents** : CPE et secrétariat prononcent des sanctions sans procédure, effacement trop précoce et jamais déclenché, aucun délai de défense, textes internes et SMS servis aux familles, RLS inopérante.
- **Passe personas** (2026-11-11) : `docs/socles/PERSONAS_13.md` (ajouts au §32 de la conception).

#### 14. Accueil, sorties et sûreté
- **Contient** : accueil (registre des visiteurs, badge visiteur, appel d'un parent transformé en déclaration d'absence) ; régimes et autorisations de sortie (niveau, régime, emploi du temps) ; contrôle aux portes ; **remise de l'élève à une personne autorisée** (vérifiée contre le socle 4) ; sortie anticipée ; **mise en sûreté et évacuation** (PPMS) : liste des présents en temps réel, appel d'urgence, compte rendu d'exercice [H] ; main courante.
- **Ne contient pas** : sorties et voyages pédagogiques (→ 42) ; internat (→ 41).
- **Dépend de** : 4, 10, 12.
- **Existant [F]** : sorties (062–065), passage par badge (036), déclarations de l'accueil (131). Rien pour la mise en sûreté.
- **Place** : complète la journée (qui entre, qui sort, qui est présent). La liste des présents en cas d'alerte est un argument de sécurité que peu d'ENT couvrent [H].
- **Faire mieux** : en cas d'alerte, chaque adulte voit sur son téléphone qui il doit avoir avec lui.
- **Conception** (2026-11-11, non décidée) : `docs/socles/14_ACCUEIL_SURETE.md` et passe personas `docs/socles/PERSONAS_14.md` : 249 actions (ACS-100 à ACS-348), lots AS0 à AS14 dont **AS0 = correctifs urgents de 062–065** ; module accueil de Next.js **partiel** (saisie d'absences, 10 actions reprises) ; « passage par badge (036) » est en fait le self (socle 40). Plage de migrations 2280–2299.

#### 15. Absences des personnels et remplacements
- **Contient** : déclaration et validation des absences des personnels ; liste automatique des séances touchées ; remplacements de courte durée, suppléances ; information des familles (sans motif) ; décompte des heures supplémentaires effectives ; absences pour formation.
- **Ne contient pas** : paie (→ 49, qui lit les heures).
- **Dépend de** : 6, 10, 8.
- **Existant [F]** : `ent_absence_rh` (080), `domain-eleve/src/absence_rh.rs` ; exceptions d'emploi du temps avec remplaçant.
- **Place** : la journée est complète quand un cours sans professeur est remplacé ou annoncé.
- **Faire mieux** : de l'absence du professeur à la famille prévenue sans saisie intermédiaire.
- **Conception** (2026-11-11, non décidée) : `docs/socles/15_ABSENCES_PERSONNELS.md` — trois objets (fait RH confidentiel à **nature catégorisée**, sans motif libre ni diagnostic ; **impact** calculé sur les séances, postes et engagements ; **réponse** d'organisation) plus les **heures constatées** ; circuit porté par l'engagement (État / OGEC) ; module Next.js M05 jugé **partiel** (règles reprises comme spécification, NX15-01 à NX15-58 ; 16 défauts AD-, dont motif médical lisible des surveillants et auto-validation, signalés à Florian) ; **lot A0** sur l'ENT Rust (motif lisible de tout le personnel, auto-validation, déclarant écrasé, levées supprimées : AR-01 à AR-26) ; réglementaire : décret 2023-732 (plan RCD), arrêté du 14/08/2023 (remontée hebdomadaire SI-RCD), décret 2023-738 (privé), circulaire 96-248 (familles prévenues, sortie), CNIL 2019-160 ; lots A0 à A15 ; migrations **2300–2319**.
- **Passe personas** (2026-11-11, non décidée ; bloquants appliqués par défaut, D-031) : `docs/socles/PERSONAS_15.md` (16 personas, 62 manques M-1501 à M-1562) ; bloquants : EDT de référence avant tout effet vers les familles, signalement non confirmé sans effet extérieur (canular), circuit externe (rectorat, DSDEN/PIAL décident), présence visible à qui en a besoin ; 200 actions ABP-100 à ABP-299 au total.

### Étage D — Échanger

#### 16. Messagerie et conversations
- **Contient** : conversations, groupes (classe, équipe, instance) ; matrice « qui peut écrire à qui » ; pièces jointes privées ; accusés ; modération ; message « à propos d'un élève » visible dans sa chronologie ; **carnet de correspondance numérique** avec signature ; temps réel ; recherche ; archivage ; **cartes d'action** dans la conversation (justifier, signer, payer, répondre).
- **Ne contient pas** : diffusions en masse (→ 17) ; visio (→ 33) ; assistant (→ 30, qui s'insère ici).
- **Dépend de** : 4, 6, 7, 8.
- **Existant [F]** : 020, 023, 037, 038, 047, 050 ; module Leptos `messagerie` ; messagerie dans `ent-mobile`.
- **Place** : condition de vente ; c'est la surface du futur « chat first » (30).
- **Faire mieux** : l'action se fait dans la conversation, sans chercher le bon menu.
- **Conception** (2026-11-09, rattrapage, non décidée) : `docs/socles/16_MESSAGERIE.md` — **lire = participer** (participants nommés, plus de visibilité déduite du lien avec l'élève) ; un fil par foyer pour les parents séparés ; **boîtes de service** (vie scolaire, secrétariat, infirmerie…) avec prise en charge ; matrice « qui peut écrire à qui » calculée sur les faits des socles 4, 5, 6 (sept réglages bornés) ; fils encadrés adulte–mineur, pas de messagerie entre élèves ; plage calme nationale (20 h–7 h, week-end) ; gel, signalement, aucune suppression par un utilisateur ; IA en brouillons confirmés ; **lot M0a de correctifs urgents** sur l'existant (interdiction de communication ignorée, fils privés lus par tout personnel qui suit l'élève, élève et parents qui lisent les fils les uns des autres, contenu en clair dans les notifications) ; 133 actions MSG-100 à MSG-308, 80 cas limites, lots M0a à M17. Plage de migrations 4600–4699 (Q-B2) ou 480–499.

#### 17. Informations, annonces et campagnes
- **Contient** : annonces ciblées (tous, classe, niveau, groupe, rôle) ; actualités ; lettres d'information ; campagnes mail / SMS / push avec prévisualisation des destinataires, coût et double confirmation ; suivi de lecture ; publipostage.
- **Dépend de** : 8, 16.
- **Existant [F]** : annonces (086) ; envois groupés par e-mail et SMS (routes existantes).
- **Place** : même public que la messagerie, autre usage.
- **Faire mieux** : on sait qui n'a pas lu, et on le relance seul.
- **Esquisse** (2026-11-09, dans `docs/socles/16_MESSAGERIE.md` §17 et §19.11, non décidée) : une publication (annonce, actualité, lettre, campagne) est un acte éditorial à **audience décrite**, résolue sous les droits de l'auteur ; la **diffusion** (aperçu, coût, confirmation, relance des non-lecteurs) est la mécanique du socle 8 (Q-N2) ; pas de commentaires publics, « poser une question » ouvre un fil ; reprise des annonces (086) et du contenu des campagnes (191) ; 17 actions ANN-100 à ANN-116, lot M15. Conception propre à faire. Plage réservée 4700–4799 (Q-B2) ou 500–519.
- **Conception et passe personas** (2026-11-11, branche `ent-socle-17-18-conception`, non décidées) : `docs/socles/17_ANNONCES.md`, `docs/socles/PERSONAS_17.md` — huit natures de publication, audience par critères en table (critères sensibles interdits), **publier ≠ notifier**, budget d'attention et résumé hebdomadaire (titres « diffusables » seulement), suivi par famille (touchée / pleinement), campagnes à attentes (18, 7, 19, 25), droit à l'image vérifié au socle 18 avant chaque publication et masquage automatique au retrait, représentants de parents sans validation de fond (D111-9), mode « miroir » pendant l'année de doublon ; **lot AN0** de correctifs (publication sans aperçu par `confirmer:true`, élève qui lit les annonces des parents, parent interdit qui lit, brouillons visibles, absence de portée) ; 171 actions (ANN-100 à ANN-270), lots AN0 à AN13. **Plage de migrations 2340–2359** (remplace 4700–4799 / 500–519).

#### 18. Formulaires, autorisations et consentements
- **Contient** : moteur de formulaires et de sondages ciblés (anonyme ou nominatif annoncé ; une réponse par responsable ou par foyer) ; autorisations récurrentes (droit à l'image, usage du numérique, sortie, transport) ; **consentements versionnés** avec retrait ; résultats et exports.
- **Dépend de** : 4, 7, 8.
- **Apporte** : la vérification « a-t-on le droit ? » avant de publier une photo, d'emmener en sortie, d'inscrire à un service. [R] Les formulaires propres de 3 (préinscription) et 4 (fiche de rentrée) seront rebranchés sur ce moteur.
- **Existant** : rien (A).
- **Place** : avant toutes les fonctions qui exigent un accord (trombinoscope, sorties, élections, activités).
- **Faire mieux** : contrôle de cohérence automatique entre consentement et usage (A15).
- **Correction (2026-11-11)** : l'existant n'est pas « rien » : `193_formulaires.sql`, `formulaires.rs`, écrans Leptos `formulaires/` et masquage de la photo du trombinoscope existent [F] (P).
- **Conception et passe personas** (2026-11-11, branche `ent-socle-17-18-conception`, non décidées) : `docs/socles/18_FORMULAIRES.md`, `docs/socles/PERSONAS_18.md` — quatre natures juridiques (information, collecte, autorisation, consentement), décideurs calculés par les socles 2 et 4 (règles `un` / `tous` / élève, avis de l'enfant pour son image, désaccord, capacité, interdiction, délégataire), droit à l'image par usage et par canal (deux titulaires par défaut, à confirmer par un juriste), port unique « a-t-on le droit ? » jamais vrai par défaut, versions à usages, cohérence accord ↔ usage, moteur de formulaires typé sans JSON, sondages réellement anonymes (tables séparées, lots mélangés, route hors journaux, seuil de 5) ; **lot FO0** de correctifs (accord de la version 1 appliqué à la version 2, parent interdit qui lit et signe, décision de l'autre parent visible, majeur non prévenu, capacité ignorée, un seul accord toujours suffisant) ; 195 actions (FRM-100 à FRM-294), lots FO0 à FO13. **Plage de migrations 2360–2379.**

#### 19. Agenda, réunions et rendez-vous
- **Contient** : calendrier de l'établissement ; réunions (convocation, présence, compte rendu) ; **rencontres parents-professeurs** (plages proposées automatiquement selon les services) ; prise de rendez-vous ; disponibilités ; iCal ; lien visio (33).
- **Dépend de** : 6, 10, 16.
- **Existant [F]** : rendez-vous psy (046) ; iCal de l'emploi du temps.
- **Place** : attendu par les familles ; sert l'orientation (37), le suivi (36), la visio (33).
- **Faire mieux** : soirée parents-professeurs planifiée sans tableur.
- **Conception** (2026-11-11, non décidée) : `docs/socles/19_AGENDA_RDV.md` (agenda **calculé** depuis les socles propriétaires, moteur unique de créneaux pour rendez-vous, rencontres parents-professeurs, sessions, rendez-vous d'inscription du 3 ; réunions et instances avec délais réglementaires ; **réservation de lieux portée par le 19** — reprise du module Next.js fiable, le 44 garde le matériel ; programmes d'interventions (refonte du « planning par domaines ») ; parents séparés et interdictions ; 227 actions AGD-100 à AGD-341 ; lots AG0 à AG12 ; lot AG0 = 11 correctifs de `rencontres.rs`). Passe personas : `docs/socles/PERSONAS_19.md` (70 manques, 4 bloquants). Migrations 2380–2399.

### Étage E — Apprendre et évaluer

#### 20. Cahier de textes et travail personnel
- **Contient** : contenu des séances, progressions ; travail à faire ; travail à rendre en ligne, remises, copies annotées et retour ; exercices et QCM corrigés automatiquement ; **charge de travail** (éviter trois contrôles le même jour) ; agenda de l'élève ; visa de la direction.
- **Dépend de** : 10, 7, 6.
- **Existant [F]** : 049 ; branche `ent-socle2-cahier` (non fusionnée) : migration 150, cahier, travail, copies, agenda, charge, écrans Leptos ; cahier de textes dans `ent-mobile`.
- **Place** : ouvre l'étage pédagogique : quotidien de chaque professeur et de chaque élève.
- **Faire mieux** : charge de travail visible avant de donner un devoir.
- **Conception** (2026-11-11, rattrapage, non décidée) : `docs/socles/20_CAHIER_DE_TEXTES.md` — cahier rattaché au **cours** (5), auteur = intervenant en saisie à la date (6), entrée sur la séance stable (10) avec **propositions de report** ; préparer ≠ publier ; **plage calme opposable** (arrêté du 7-9-2026 [V]) ; charge **par élève** (obligation de la circulaire 2010-136 [V]) ; dépôts immuables et prouvés, non-rendu = fait ; variantes et aménagements sans motif ; visa direction et inspection, archive un an actif + cinq ans [V] ; IA sans note ni détection ; **lot C0 de correctifs urgents** (contenu détruit à la régénération, copies lisibles par un ancien remplaçant, la vie scolaire et les autres professeurs, droits des liens ignorés, élève parti qui dépose) ; 187 actions CDT-100 à CDT-289, 90 cas limites, 44 règles, lots C0, C2a, C1 à C14. Plage de migrations **2400–2419**.
- **Passe personas** (2026-11-11, non décidée ; bloquants appliqués par défaut, D-031) : `docs/socles/PERSONAS_20.md` (17 personas, 78 manques M20-01 à M20-78 : 3 bloquants, 46 importants, 27 confort, 2 différenciants) ; bloquants : (A) plage calme livrée juste après C0 (lot C2a), (B) toute modification publiée visible des lecteurs avec ses versions, (C) corrigés d'exercices jamais envoyés au client ; à Florian : usage réel ou doublon en 2027-2028 (Q-CDT2), durées des copies et AIPD (DPO), charte et règlement intérieur, convention SAPAD.

#### 21. Évaluations et notes
- **Contient** : évaluations, barèmes, coefficients ; saisie en grille (web, mobile) ; absent / dispensé / non noté distincts ; **publication explicite ou différée** aux familles ; moyennes selon des règles paramétrables (arrondis, sous-matières) ; rattrapages ; correction tracée après clôture ; statistiques par évaluation.
- **Ne contient pas** : compétences (→ 22) ; bulletins et conseils (→ 23).
- **Dépend de** : 6, 10, 20.
- **Existant [F]** : 008, 022, 056 ; `domain-eleve/src/notes.rs` ; notes dans `ent-mobile`.
- **Faire mieux** : règles de moyenne lisibles par la famille ; aucune note publiée par erreur.
- **Conception** (2026-11-09, rattrapage, non décidée) : `docs/socles/21_EVALUATIONS.md` — moteur de calcul existant gardé (exact) ; **15 défauts** de l'existant dont des fuites (bulletin lisible par un parent sans droit aux notes, appréciations visibles avant le conseil), réouverture de période par l'administratif sans trace, notes écrasées et supprimées, aucune RLS, `absent_compte_zero` contraire à la règle nationale (mémento eduscol 2025 [V] : un absent n'est jamais un zéro) → **lot V0 de correctifs** avant tout ; évaluation sur le cours et la période du socle 5, public à la date, saisir / publier / arrêter / figer, instantané d'arrêt, fenêtre du conseil (D-022), IA sans note ni décision ; 133 actions EVA-100 à EVA-232, 64 cas limites, lots V0 à V13 ; contient aussi ce que les socles 22 et 23 attendent du 21 (§6). Plage de migrations 5100–5199 (Q-B2) ou 460–479.
- **Passe personas** (2026-11-09, non décidée ; bloquants appliqués par défaut, D-031) : `docs/socles/PERSONAS_21.md` (16 personas, 90 manques M-579 à M-668 : 5 bloquants en trois points, 44 importants, 38 confort, 3 différenciants ; parité lue sur les pages Pronote 2025-2026 et Aplim) ; **bloquants** : (A) période et verrou **par élève** pour les cours multi-classes (R-V29, schéma de V1), (B) **cohabitation outillée** de l'an 1 du lycée pilote — export de l'assiduité vers Charlemagne, bulletins Charlemagne déposés et **distribués par ScolIA** selon les droits des liens (lot V14 juste après V0), (C) **moyenne externe** pour une matière suivie hors de l'établissement (CNED, autre lycée, conservatoire ; R-V38) ; ajouts « ajout personas » : EVA-233 à EVA-280, R-V29 à R-V47, CE-65 à CE-93, R-25 à R-39 (C. civ. 373-2-1 [V] : parent sans exercice informé des choix importants ; non-rendus signalés au bulletin [V] ; contrôle continu et évaluation de remplacement au bac [V]), lots V14 à V16, Q-EV16 à Q-EV25, S-121 à S-131.

#### 22. Compétences et livrets
- **Contient** : référentiels (socle commun à 8 composantes, référentiels disciplinaires et professionnels) ; évaluation et positionnement ; bilans périodiques et de fin de cycle ; **LSU au format officiel** [H] ; attestations.
- **Dépend de** : 21, 9 (traces d'export).
- **Existant [F]** : 088 ; `lsun_export.rs` (CSV, pas le format officiel).
- **Place** : condition de vente au collège.
- **Conception et passe personas** (2026-11-11, non décidées ; recommandations appliquées par défaut, D-031) : `docs/socles/22_COMPETENCES.md`, `docs/socles/PERSONAS_22.md` — fusion des deux modèles existants (088, 181) en un référentiel versionné (national figé [V D. 122-1, D. 311-7], établissement rattaché, diplôme) ; positionnements historisés saisis dans la grille du 21 ; synthèse **proposée** et validée ; bilan de fin de cycle préparé, arrêté en conseil, **trois visas** (PP, chef, parents [V arrêté du 31/12/2015 art. 6]) ; attestations PSC, ASSR, AER, ASSN, Pix ; le socle ne compte plus au DNB 2026 [V] mais le bilan reste au livret ; IA sans positionnement ; LSU après la démarche éditeur, pré-contrôle avant ; lot **C0** (10 défauts CD1–CD10, dont bilan visible avant publication et livret publié sans contrôle) ; 120 actions (CMP-100 à 266), lots C0 à C13, migrations **2440–2459** ; 3 bloquants personas (droit `recevoir_bulletins` du socle 4 préalable, observations jamais sur un comportement isolé, élèves à deux équipes).
- **Attentes posées par la conception du socle 21** (2026-11-09) : `21_EVALUATIONS.md` §6.1 — évaluations par positionnement (6ᵉ–4ᵉ sans note admis par l'arrêté du 31/12/2015 [V]), fusion des deux modèles de compétences existants, LSU depuis les moyennes annuelles du port du 21, « en attente » et « dispensé » (EPS, LV2) ; fuite des compétences avant publication corrigée par le lot V0 du 21.

#### 23. Bulletins, conseils de classe et passage
- **Contient** : appréciations par matière et générale ; **préparation automatique du conseil** (moyennes, assiduité, discipline, appréciations manquantes) ; tenue du conseil en séance (tablette) ; mentions, avis ; **décisions de passage, redoublement, orientation** ; bulletin figé et versionné, publication ; LSL [H] ; recours.
- **Dépend de** : 21, 22, 12, 13, 7.
- **Apporte** : les décisions qui pilotent la montée (5), l'orientation (37), les aménagements (35).
- **Existant [F]** : 071, 056 ; `bulletin_pdf.rs`, `lsl_export.rs` ; montée en lot (S2-2).
- **Faire mieux** : conseil préparé en un clic (A7).
- **Conception et passe personas** (2026-11-11, non décidées ; D-031) : `docs/socles/23_BULLETINS.md`, `docs/socles/PERSONAS_23.md` — conseil en un clic, séance sur tablette minimisée, **verrou** à la tenue, PV ; le conseil **propose**, le chef **décide** (D. 331-31 à 38, D. 331-62 [V]) ; décision motivée, 3 jours ouvrables, appel, commission (public, privé), maintien, avis de l'élève recueilli ; notification sans voies et délais bloquée (CJA R. 421-5 [V]) ; **fiche de dialogue de Next.js (M10) reprise** (16 actions NXF, 12 défauts corrigés, production pour 2027-2028, D-035) ; 4 trames de bulletin, exemplaires calculés à la publication ; an 1 : bulletins Charlemagne distribués par ScolIA ; montée qui lit enfin les décisions ; lot **B0** (14 défauts BD1–BD14, dont décisions écrasées, bulletin qui imprime des décisions provisoires, cascade qui efface l'avis) ; 152 actions (BUL-100 à 426) + 37 par la passe (BUL-430 à 466), lots B0 à B15, migrations **2460–2479** ; 3 bloquants personas (notification valable et délai d'appel par responsable, appel de chaque titulaire, actes engageants authentifiés par passkey). **Contradiction levée** : la fiche de dialogue est au socle 23, pas au 37 (`TRANSITION_T0_INVENTAIRE.md` l. 207).
- **Attentes posées par la conception du socle 21** (2026-11-09) : `21_EVALUATIONS.md` §6.2 — instantané d'arrêt comme entrée unique du conseil et du bulletin ; décisions historisées et verrou à la tenue ; mentions = données de l'établissement, **jamais « avertissement »** (sanction du chef d'établissement, R. 511-13 [V]) ; redoublement décidé par la direction après avis du conseil (D. 331-62 [V]) ; 4 modèles de bulletin (collège, lycée GT, lycée pro, sans note) rendus par le socle 7 ; correctifs urgents des routes historiques de conseil et de bulletin pris par le lot V0 du 21.

### Étage F — L'argent des familles

#### 24. Facturation
- **Contient** : articles, tarifs, formules (scolarité, demi-pension, internat, études, sorties) ; barèmes au quotient ; réductions (fratrie, personnel) ; génération annuelle ; **facturation des consommations par événement**, au prorata ; factures, avoirs ; **répartition multi-payeurs** (garde alternée) ; TVA ; compte famille ; dépôt dans le coffre famille (7).
- **Ne contient pas** : encaissements et prélèvements (→ 25) ; bourses (→ 26) ; comptabilité générale (→ 48).
- **Dépend de** : 2 (régime), 3 (inscription validée), 4 (payeurs), 7, 8.
- **Existant [F]** : 024, 026, 027, 032, 075, 079 ; `domain-eleve/src/facturation*.rs` ; branche `ent-socle2-finances` ouverte.
- **Place** : condition de vente du privé. Placée après la pédagogie parce que l'existant (factures, échéances, SEPA, relances) fonctionne déjà dans l'ancienne interface et que la facturation par événements a besoin de tous les faits de la journée (régime, repas, études).
- **Faire mieux** : une rentrée facturée sans ressaisie (A8) ; « d'où vient cette ligne ? » en un clic.

#### 25. Paiements, prélèvements et recouvrement
- **Contient** : encaissements (chèque, espèces, virement) ; SEPA (mandat par créancier, pré-notification [H], fichiers, rejets) ; **paiement en ligne par carte** (prestataire à choisir, Q5) ; rapprochement bancaire ; relances par paliers ; plans d'apurement ; impayés et contentieux ; remboursements ; quittances, attestations fiscales ; coordonnées bancaires chiffrées (IBAN masqué, reprise D-014).
- **Dépend de** : 24.
- **Existant [F]** : 025, 029, 030, 031, 059, 078 ; `ent-bank` (rapprochement simulé) ; historique d'IBAN.
- **Faire mieux** : la famille voit son solde, ses échéances et paie en deux gestes.

#### 26. Bourses et aides financières
- **Contient** : bourses nationales (campagne, dossier, échelons, déduction ou versement) [H] ; fonds sociaux (commission, décision anonymisée) ; aides locales (région, département) ; quotient familial ; primes [H].
- **Dépend de** : 24, 25, 18, 7.
- **Existant [F]** : 017, 033, 075 ; types de bourses et bourses de l'élève.
- **Place** : complète l'argent ; indispensable au public, où l'essentiel de la facturation relève de l'agent comptable [H].

### Étage G — Prêt pour un premier établissement

#### 27. Conformité RGPD, conservation et archives
- **Contient** : registre des traitements de l'établissement ; durées de conservation par catégorie (validées par le DPO) ; purge et anonymisation planifiées ; archivage de fin d'année et de sortie d'élève ; droit d'accès (extrait), rectification, opposition ; **journal des accès** visible par la famille, par service (Q9) ; procédure de violation de données ; sous-traitants ; **export de réversibilité** en fin de contrat.
- **Dépend de** : tous les socles précédents (chacun a déclaré ses catégories de données).
- **Existant [F]** : journal en ajout seul (091), journal des modifications (068), `docs/DATA_GOVERNANCE.md`, règles de rétention des événements (`SOCLE2_DOMAINES.md` §4.4).
- **Place** : aucun établissement réel ne peut démarrer sans.
- **Faire mieux** : droit d'accès en un clic (A12) ; transparence des consultations (A11).

#### 28. Mise en service, reprise de données et support
- **Contient** : création d'un établissement (domaine D-012, paramètres, rôles modèles) ; assistant de paramétrage ; **reprise de données** depuis les logiciels en place (Charlemagne, École Directe, Pronote, SIECLE) avec rapprochement et contrôle ; données de démonstration ; aide contextuelle et formation intégrée ; support (demandes, accès support tracé et consenti) ; supervision, PRA testé, page d'état du service ; abonnement de l'établissement à ScolIA.
- **Dépend de** : 2 à 9.
- **Existant [F]** : `ent-lab`, `ent-db/src/{seed,scenarios,pra}.rs`, `ent-obs`, `docs/demo-one-click.md`, runbooks (`docs/ent-rust-prod-runbook.md`, `docs/migrations-prod-runbook.md`) ; catalogue des 659 tables de Charlemagne (`docs/ent-metier-checklist.md`), base d'une reprise.
- **Place** : la reprise des données est le premier frein au changement de logiciel [H] ; sans elle, pas de premier client.
- **Faire mieux** : un établissement migré en un week-end, avec rapport des écarts.

#### 29. Portails, « Aujourd'hui » et à-faire unifié
- **Contient** : page d'accueil par rôle ; **à-faire unifié** des familles (signer, justifier, payer, répondre, réinscrire, prendre rendez-vous, A3) et des personnels (appels, notes, justificatifs à traiter) ; recherche globale et palette de commandes ; multi-enfants, multi-établissements ; accessibilité RGAA (audit) [H] ; retrait de `ui.rs` pour les rôles entièrement couverts.
- **Dépend de** : 8 et tous les socles qui produisent des « choses à faire » (12 à 26).
- **Existant [F]** : accueil Leptos (Bento, tuiles).
- **Place** : arrive quand assez de socles alimentent l'à-faire ; ferme le jalon pilote.
- **Faire mieux** : les familles ne cherchent plus dans dix menus.
- **Conception** (2026-11-11, non décidée) : `docs/socles/29_PORTAILS.md` — le 29 ne possède aucune donnée métier : **index des tâches** revérifié à chaque lecture et reconstructible (natures déclarées par chaque socle dans son kit, ouvertes et closes dans la transaction du fait), **composition** des accueils par un contrat de tuile unique (partagé avec la fiche 360° du socle 2), **recherche fédérée** et **palette** sur le registre des capacités de l'IA ; frontière avis (8) / tâche (29) ; files de travail laissées à leurs socles ; neutralité totale entre parents ; réunion multi-établissements sur l'appareil seulement ; plage calme de l'**arrêté du 7/09/2026** (20 h–7 h, week-end) appliquée à l'affichage ; 167 actions POR-100 à POR-266, 86 cas limites, 38 règles, lots P0 à P16 ; **lot P0** (annonces jamais visibles des familles sur l'accueil, accueil famille sans emploi du temps, une requête par enfant et notes téléchargées pour un compteur, tuiles commandées par le rôle fixe au lieu de D-017, profils administratifs renvoyés à `ui.rs`) et **lot P1** « démo de décembre » sans migration. Plage de migrations 2580–2599.
- **Passe personas** (2026-11-11, non décidée ; bloquants appliqués par défaut, D-031) : `docs/socles/PERSONAS_29.md` (16 personas, 48 manques M-2900 à M-2947) ; **bloquants** : purge et expiration du cache hors ligne d'un parent au lien suspendu (R-PO33), compteurs multi-établissements par le guichet central et jamais par CORS (R-PO34), alerte de restriction de remise visible en mode discret (R-PO35), démo écrite sur le contrat de tuile (R-PO36) ; ajouts : neutralité temporelle entre parents (R-PO38), ajustements préparés de la plage calme (R-PO37), information des « choix importants » au parent sans exercice de l'autorité.

### Étage H — Au-dessus des domaines (ancien « socle 3 » de la roadmap)

#### 30. Assistant IA et « chat first »
- **Contient** : assistant dans la conversation (16), dans le périmètre exact de l'utilisateur (jamais plus de droits) ; questions en langage naturel ; **propositions d'actions du catalogue** (identifiants stables), exécutées par les services métier après **confirmation humaine** ; explication (chaîne de causalité) ; rédaction assistée validée (appréciations, messages, comptes rendus) ; résumé de dossier ; choix du fournisseur (Mistral prioritaire, sous garanties contractuelles) ; aucune clé côté client ; minimisation, journal, évaluation de la qualité ; fonctionnement complet sans IA.
- **Dépend de** : 16, 27 (minimisation, registre), 29 ; le catalogue d'actions (1) et la chronologie (2).
- **Existant [F]** : `ent-db/src/copilote.rs` (assistant déterministe, lecture seule, sans LLM) ; `ent-brain` ; `docs/AI_PLATFORM.md`.
- **Repose sur le cœur IA [R]** (§1 bis, `docs/socles/IA_COEUR.md`) : registre, brouillons, confirmation, journal, minimisation et fournisseurs y sont construits dès le socle 2 ; le socle 30 n'apporte que la conversation, ses enchaînements de tâches et l'ouverture aux familles et aux élèves.
- **Place** : ancien « socle 3 » de la roadmap. Après le jalon pilote parce que l'IA vaut ce que valent les actions et les données qu'elle peut toucher ; **ses points d'accroche sont posés dès le socle 2** (chaque socle décrit ses actions pour l'assistant). [R] Peut remonter juste après 16 si la priorité commerciale l'exige : il ne dépend durement que de 16, 27 et du catalogue.
- **Faire mieux** : « fais l'appel de la 4B comme lundi sauf Léa » → aperçu → confirmation ; jamais d'action invisible.

#### 31. Automatisations et règles de l'établissement
- **Contient** : règles « quand… si… alors… » paramétrables sans code sur les événements ; **simulation à blanc** (« cette action entraînera… », A20) ; tâches planifiées ; détection précoce explicable (décrochage, A6) ; contrôles de cohérence inter-domaines (A15) ; files de validation humaine ; journal des exécutions.
- **Dépend de** : 8, 30 (file de validation partagée), l'outbox.
- **Existant [F]** : `ent-brain` (réaction à `appel.closed`), consommateurs de l'outbox, signaux d'absence (043), alertes CPE (069).
- **Place** : avec l'IA, le second moteur au-dessus des domaines.

#### 32. Espaces de travail et bureautique
- **Contient** : espaces de classe, de groupe, de projet ; documents partagés ; casier numérique ; espace personnel avec quotas ; **bureautique ArtCraft** en WebAssembly (D-006, version figée, sans marques) ; portfolio de l'élève.
- **Dépend de** : 7, 20, 6.
- **Existant** : rien (décision D-006 seulement).
- **Place** : différenciant, pas une condition de vente.

#### 33. Visioconférence et continuité pédagogique
- **Contient** : visio intégrée (cours, rendez-vous, conseils) ; solution souveraine ou auto-hébergée à choisir, aucun service externe par défaut ; classe virtuelle ; plan de continuité en cas de fermeture.
- **Dépend de** : 16, 19, 10.
- **Existant** : rien.

### Étage I — Accompagner chaque élève

#### 34. Santé et infirmerie
- **Contient** : fiche santé chiffrée ; vaccinations ; passages et soins ; registre ; renvoi au domicile ; information de la famille sans détail médical ; PAI, protocoles d'urgence, trousses, traitements ; dispenses médicales (vers 12) ; **contraintes minimales publiées** (« PAI alimentaire », « trousse d'urgence ») seules partagées avec cantine, sorties, professeurs ; question de l'hébergement certifié (Q7).
- **Dépend de** : 2, 4, 7, 12.
- **Existant [F]** : 040, 045, 052, 053 ; champs chiffrés ; `domain-eleve/src/{fiche_sante,pai,infirmerie}.rs`.
- **Place** : ouvre l'étage, car restauration, internat et sorties en dépendent.
- **Conception** (2026-11-10, non décidée) : `docs/socles/34_SANTE.md` (secret confié, conséquences partagées ; trois niveaux d'activation N-0 / N-1 « sécurité et organisation » sans dossier de soins, après avis du DPO / N-2 « cabinet » seulement en HDS de bout en bout, base séparée ; public : LIEN et Esculape, pas de double saisie ; 160 actions SAN-100 à SAN-295, 58 cas limites, 34 règles, lots S0–S15, migrations 6400–6499 ou repli 580–599). Défauts à corriger en premier (lot S0) : direction et administrateur lisent les dossiers médicaux, interdictions de remise ignorées au renvoi au domicile, garde du module seulement HTTP, motif médical des dispenses visible des professeurs, avis d'infirmerie perdus, ordonnances non chiffrées, activation sans attestation.

#### 35. Besoins éducatifs particuliers et inclusion
- **Contient** : PAP, PPS, PPRE, ULIS, élèves allophones [H] ; AESH (affectation, planning) ; aménagements pédagogiques et d'examen visibles des seuls professeurs concernés ; équipes de suivi ; révisions et échéances ; tutorat.
- **Dépend de** : 34, 6, 23.
- **Existant [F]** : `ent_eleve_dispositif` (016).

#### 36. Suivi social, psychologique et protection de l'enfance
- **Contient** : psychologue (entretiens, rendez-vous, dossier cloisonné) ; service social (suivi, aides) ; groupe de prévention du décrochage [H] ; **informations préoccupantes** : rédaction et transmission tracées, accès très restreint [H] ; cellule de veille.
- **Dépend de** : 2, 4, 19.
- **Existant [F]** : 041, 046, 051 ; `domain-eleve/src/psy.rs`.

#### 37. Orientation et parcours
- **Contient** : parcours Avenir ; intentions et vœux (3e, 2de, terminale) ; avis du conseil et dialogue avec la famille ; Affelnet, Parcoursup (fiche Avenir) [H] ; entretiens ; forums.
- **Dépend de** : 23, 19.
- **Existant [F]** : 077 ; `parcoursup_export.rs`.

#### 38. Examens et certifications
- **Contient** : examens blancs, épreuves, salles, placements, surveillances, convocations ; aménagements (35) ; cours neutralisés (10) ; dépôt sécurisé des sujets ; notes (21) ; résultats officiels (DNB, bac) ; contrôle en cours de formation et oraux [H].
- **Dépend de** : 10, 21, 35.
- **Existant [F]** : `notanet_export.rs` seulement.

#### 39. Stages, PFMP et entreprises
- **Contient** : entreprises et tuteurs (accès par lien à usage unique, sans compte) ; conventions signées par les trois parties ; périodes (absences attendues à l'appel) ; visites, évaluation, attestations ; PFMP du lycée professionnel, stages d'observation.
- **Dépend de** : 7, 12.
- **Existant [F]** : 082 ; `domain-eleve/src/stage.rs`.
- **Conception** (2026-11-11, non décidée) : `docs/socles/39_STAGES.md` et `PERSONAS_39.md`. **Pas de convention signée, pas de départ** (R-SG1) ; l'élève reste un élève, localisé ailleurs ; signataires calculés (4, 6) et circuit du 7 ; tuteur sans compte par lien ; contrôles horaires des mineurs jour par jour ; travaux réglementés et aptitude ; visites, évaluation, attestation, décompte des semaines ; allocation PFMP calculée au jour (ApLyPro, mode suivi pendant le doublon) ; reprise du module Next.js (37 actions) avec **4 failles critiques ou hautes à signaler à Florian (F-SG1)** ; 240 actions STG-100 à STG-359 ; lots ST0 à ST12 (+ ST3a) ; lot ST0 = 10 correctifs du Rust 082 ; passe personas : 18 personas, 72 manques, 4 bloquants (année N+1 préparée en juin, mise à l'abri sur signalement de violences, convention de l'organisme, attestation). Plage de migrations 2780–2799.

### Étage J — Services et vie de l'établissement

#### 40. Restauration
- **Contient** : services, forfaits et grilles ; réservations et annulations ; badges et passages ; menus et allergènes ; régimes alimentaires non médicaux ; alerte minimale PAI au passage ; prévision des effectifs cuisine ; volume facturable ; gaspillage.
- **Dépend de** : 12, 24, 34.
- **Existant [F]** : 017, 034, 036, 074 ; `domain-eleve/src/{repas,menu_cantine}.rs`.
- **Conception** (2026-11-11, non décidée) : `docs/socles/40_RESTAURATION.md` et `PERSONAS_40.md`. Une seule règle du droit au repas pour la cuisine, le self, la famille et la facture ; non-repas automatiques à cause codée ; consignes du 34 sans diagnostic ; lot R0 = correctifs urgents du Rust existant ; pas de bascule du self sans niveau N-1 du 34 (ou liste attestée). Plage de migrations 2800–2819.

#### 41. Internat
- **Contient** : chambres et affectations ; appel du soir ; absences d'internat annoncées (repas annulés) ; suivi ; sorties de week-end ; études du soir ; planning des surveillants.
- **Dépend de** : 12, 24, 34.
- **Existant [F]** : 066 ; `domain-eleve/src/internat.rs`.
- **Conception** (2026-11-11, non décidée) : `docs/socles/41_INTERNAT.md` et `PERSONAS_41.md`. Attendu de la nuit calculé, présence en faits horodatés, procédure « non retrouvé » minutée, absences annoncées par les familles, sorties autorisées par les bons titulaires, présents de nuit pour l'évacuation ; le planning des surveillants reste au 6 ; lot I0 = correctifs urgents du Rust existant ; bascule recommandée à l'été 2027. Plage de migrations 2820–2839.

#### 42. Sorties et voyages scolaires
- **Contient** : projet, budget, validation (direction, conseil d'administration [H]) ; participants ; autorisations et pièces ; paiements et aides ; encadrants (ordres de mission) ; absences, dispenses de repas et cours des encadrants générés ; fiche d'urgence hors ligne ; bilan et solde.
- **Dépend de** : 18, 24, 34, 6.
- **Existant** : rien (le Brain prévoit le cas « séjour → absence »).
- **Existant [F, Next.js]** : le module voyages de l'add-on, en production (D-035), est inventorié dans `docs/socles/42_SORTIES_VOYAGES.md` §2 : 74 actions reprises et 48 défauts.
- **Conception** (2026-11-10, non décidée) : `docs/socles/42_SORTIES_VOYAGES.md`. Le 42 prend les sorties **collectives** ; les sorties individuelles restent au 14. Lot V0 = parité avec Next.js branchée sur l'élève. Plage de migrations 7200–7299, repli 600–619.

#### 43. Activités périscolaires et transports
- **Contient** : clubs, association sportive, études surveillées, garderie, ateliers ; inscriptions et places ; appel (composant du socle 12) ; facturation (24) ; transports scolaires (lignes, abonnements, souvent gérés par la région : import plutôt que saisie [H]).
- **Dépend de** : 12, 18, 24.
- **Existant [F]** : groupes et ateliers avec responsables (S2-2).

#### 44. Ressources matérielles, prêts et maintenance
- **Contient** : réservation de salles et de matériel sans conflit avec l'emploi du temps ; prêts (ordinateurs, tablettes, manuels scolaires) avec caution ; inventaire, parc informatique ; demandes d'intervention ; clés.
- **Dépend de** : 5, 10.
- **Existant [F]** : salles (076).

#### 45. CDI et ressources documentaires
- **Contient** : catalogue, prêts et retours, réservations ; accès aux ressources numériques (GAR, via 9) ; portail documentaire.
- **Dépend de** : 9, 44.
- **Existant** : rien.

#### 46. Instances et vie démocratique
- **Contient** : élections (délégués, conseil d'administration, CVL, CVC, parents) ; corps électoral calculé ; candidatures ; **vote électronique secret** ; mandats ; convocations et procès-verbaux ; délibérations.
- **Dépend de** : 4, 6, 18.
- **Existant** : rien.

#### 47. Pilotage et indicateurs
- **Contient** : tableaux de bord de la direction ; indicateurs documentés et reconstruits depuis les événements ; comparaison d'une année sur l'autre ; remontées de rentrée [H] ; aucun classement nominatif public des personnels.
- **Dépend de** : tous (chaque socle a exposé ses indicateurs).
- **Existant [F]** : `copilote.rs`, éditions de vie scolaire (081), `ent-obs` (métriques techniques seulement).
- **Place** : consolidation ; chaque socle livre déjà ses propres listes et compteurs.

### Étage K — La gestion de l'organisme

#### 48. Comptabilité, budget et achats
- **Contient** : privé (OGEC) : plan comptable, journaux, écritures générées depuis 24–26, grand livre, balance, clôture, immobilisations, budget prévisionnel, analytique (par activité, par sortie), fournisseurs, commandes, factures fournisseurs, crédits par discipline. Public : interface avec la comptabilité de l'agent comptable [H].
- **Dépend de** : 24, 25, 26.
- **Existant [F]** : export OGEC (`facturation_ogec.rs`).

#### 49. Paie et administration du personnel
- **Contient** : privé : personnels de droit privé de l'organisme de gestion (les enseignants du privé sous contrat sont payés par l'État [H]) ; contrats, éléments variables (heures et absences issues de 15), bulletins, déclarations sociales (DSN) [H], conventions collectives [H], congés, entretiens, formation. Public : peu de paie locale [H].
- **Dépend de** : 6, 15.
- **Existant** : rien.
- **Place** : en fin de carte : réglementation lourde et changeante, utile surtout au privé (voir §8, Q-B).

### Étage L — Extensions de périmètre

#### 50. Premier degré (écoles)
- **Contient** : écoles des ensembles scolaires puis écoles publiques : ONDE [H], appel par demi-journée, cahier journal, livret scolaire du premier degré, relation avec la commune (cantine, périscolaire) [H].
- **Dépend de** : 2 à 29.
- **Existant [F]** : `onde_export.rs`.

#### 51. Post-bac et apprentissage
- **Contient** : BTS, CPGE, licences professionnelles en lycée (semestres, crédits) ; apprentissage (contrats, organismes financeurs, calendrier d'alternance, facturation) [H] ; taxe d'apprentissage [H].
- **Dépend de** : 2 à 29, 39.
- **Existant** : rien.

---

## 4. Jalons

| Jalon | Après le socle | Ce qu'on peut affirmer |
|---|---|---|
| J0 | 1 | Sécurité démontrée (fait). |
| J1 « Cœur » | 4 (et 5 en parallèle) | On sait qui est l'élève, comment il est entré, qui en répond, où et quand il est scolarisé. [R] Le cœur IA (§1 bis) est en place et les socles 2 à 4 sont décrits pour lui. |
| J2 « Journée » | 15 (et 16) | Une journée complète d'établissement (emploi du temps, appel web et mobile, vie scolaire, accueil, remplacements, messagerie) sans l'ancienne interface. |
| J3 « Pilote » | 29 | Premier établissement réel : pédagogie, argent des familles, conformité, reprise de données, à-faire unifié. Les domaines non refaits restent servis par l'existant. |
| J4 « Différence » | 33 | IA, automatisations, travail collaboratif, visio. |
| J5 « ENT complet » | 47 | Couverture Pronote + École Directe + Charlemagne (hors gestion de l'organisme). |
| J6 « Gestion et extensions » | 51 | Comptabilité, paie, premier degré, post-bac. |

**Parallélisation [R]** (nécessaire pour D-007) : 5 en parallèle de 2–4 ; 6 et 7 en parallèle ; 16–19 en parallèle de 20–23 ; 24–26 en parallèle de l'étage E ; 32, 33 indépendants ; dans l'étage I, 36, 37 et 39 sont indépendants entre eux. La colonne « Dépend de » du §2 dit ce qui peut avancer.

---

## 5. Ce qui n'est pas un socle (et que chaque socle enrichit)

| Élément | Où il vit | Ce que chaque socle y ajoute |
|---|---|---|
| Toile des événements (outbox, consommateurs idempotents, rejeu) | `ent-db/src/outbox.rs` (S2-0) | ses événements, publiés en transaction ; leurs types dans `ent-types` |
| Catalogue d'actions et moteur de droits | socle 1 (`ent-auth/src/actions.rs`) | ses actions, portées et modèles de rôles par défaut |
| Fiche 360° et chronologie | socle 2 | sa section et ses entrées de chronologie (titres neutres) |
| Design system et frontend Leptos | `ent-rust/web/`, `docs/FRONTEND_LEPTOS.md` | ses écrans ; retrait des écrans correspondants de `ui.rs` |
| Applications mobiles | socle 11 | ses écrans quotidiens |
| Isolation | matrice de séparation, RLS | ses routes et tables |
| Pilotage | socle 47 | ses indicateurs |
| Cœur IA [R] | §1 bis, `docs/socles/IA_COEUR.md` (dès le socle 2) ; conversation au socle 30 | ses lectures et commandes au registre (contrat IA), ses tests de garde, son jeu d'évaluation |

---

## 6. Correspondance avec l'ancien découpage

**Domaines de `SOCLE2_DOMAINES.md` → socles**

| Ancien domaine | Socles |
|---|---|
| D0 Plateforme | §5 (toile), 8, 27 |
| D1 Référentiel | 5 |
| D2 Dossier élève, personnes, foyers | 2, 4 |
| D3 Inscriptions | 3 |
| D4 Emplois du temps | 10 |
| D5 Vie scolaire | 12, 13, 14 |
| D6 Évaluations | 21, 22, 23 |
| D7 Cahier de textes | 20 |
| D8 Communication | 16, 17 |
| D9 Documents | 7 |
| D10 Finances | 24, 25, 26 |
| D11 Restauration | 40 |
| D12 Internat, sorties de l'établissement, stages | 41, 14, 39 |
| D13 Santé et psychologie | 34, 36 |
| D14 Pilotage | 47 |
| D15 Administration | 1, 6, 28 |
| D16 Examens | 38 |
| D17 Orientation | 37 |
| D18 Besoins particuliers | 35 |
| D19 Personnels, remplacements | 6, 15 |
| D20 Sorties et séjours | 42 |
| D21 Agenda, réservations | 19, 44 |
| D22 Activités, transports | 43 |
| D23 Interopérabilité | 9 (+ exports dans 22, 23, 37, 38, 48, 50) |
| D24 Formulaires, consentements | 18 |
| D25 Instances | 46 |

**Nouveaux par rapport aux 26 domaines** : 11 (mobile, autrefois transverse), 26 (bourses, autrefois dans D10), 28 (mise en service et reprise), 29 (portails et à-faire), 30–31 (IA, automatisations), 32–33 (espaces de travail, visio), 36 (social et protection de l'enfance), 44 (prêts, maintenance), 45 (CDI), 48–49 (comptabilité, paie), 50–51 (premier degré, post-bac).

**Lots déjà réalisés ou en cours → socles** : S2-0 → §5 et 8 ; S2-1 → 2 et 4 ; S2-2 → 5 (et séances pour 10) ; S2-3 (branche) → 3 (et import pour 9) ; S2-4 → 12 et 13 ; S2-6 (branche) → 20 ; `ent-socle2-finances` → 24–26 ; `ent-mobile` → 11. Les lots non commencés (S2-5, S2-7 à S2-15) sont absorbés par les socles de la colonne de droite du tableau ci-dessus. Les identifiants d'actions (DOS-, INS-, VS-…) restent valables.

---

## 7. Hors carte (candidats, à décider)

- ~~**Fabrication automatique de l'emploi du temps** (solveur) : produit à part chez les concurrents [H] ; gros chantier algorithmique ; socle 52 possible.~~ Dans le périmètre (D-025) ; proposée **dans le socle 10** par sa conception (2026-11-09, Q-EDT1).
- **Ensembles scolaires et réseaux** (plusieurs établissements, facturation de fratrie commune) : contredit l'absence de visibilité entre établissements (D-013) ; seulement comme « groupe » explicite et consenti (Q13).
- **Boutique et billetterie** (photos de classe, fournitures, événements) : faible priorité.

---

## 8. Questions à Florian (fermées)

| # | Question | Recommandation [R] |
|---|---|---|
| Q-A | Valider l'ordre 5 → 51 ? | Oui, avec 5 mené en parallèle de 2–4. |
| Q-B | **Paie** (socle 49) : la construire (déclarations sociales, conventions collectives) ou s'interfacer avec un logiciel de paie ? `ent-metier-checklist.md` l'avait écartée (« DROP »). | Garder le socle dans la carte, trancher au moment de sa réflexion de fond, après une étude du coût réglementaire. |
| Q-C | **Comptabilité générale** (socle 48) complète pour le privé, ou export vers le logiciel comptable existant ? (même mention « DROP ») | Même réponse que Q-B. |
| Q-D | Position de l'IA (socle 30) : après le pilote, ou remontée juste après la messagerie (16) ? | Après le pilote, points d'accroche posés dès le socle 2. **Mise à jour 2026-11-06 (D-024)** : le **cœur IA** (§1 bis) se construit en parallèle du socle 2 ; la conversation (30) garde sa place ; questions détaillées Q-IA1 à Q-IA14 dans `docs/socles/IA_COEUR.md` §18. |
| Q-E | Premier degré (50) et post-bac (51) : dans la sortie visée fin 2027 (D-007) ou après ? | Après, sauf client pilote qui les exige (cf. Q8 de `SOCLE2_DOMAINES.md`). |
| Q-F | Solveur d'emploi du temps : socle 52 ? | À étudier après J5. **Mise à jour** : D-025 l'inclut ; `10_EMPLOI_DU_TEMPS.md` propose de le construire dans le socle 10 (Q-EDT1), après le parcours minimal et un prototype mesuré. |
| Q-G | Hébergement certifié pour la santé (Q7) : à faire trancher par le DPO **avant** le socle 34. | Oui. |

---

## 9. Faits, hypothèses, contradictions

**Faits [F]** : migrations, fichiers et branches cités relevés par `ls`, `git log` et `git diff` le 2026-11-06 sur `ENT` (`d7a724b`) et sur les branches distantes nommées en tête. Rien n'a été compilé ni testé pour ce document.

**Hypothèses [H]** : règles réglementaires (assiduité, sanctions, LSU, LSL, Affelnet, Parcoursup, DNB, bourses, PPMS, informations préoccupantes, apprentissage, DSN, conventions collectives, paie des enseignants du privé, comptabilité publique) et pratiques du marché : **à vérifier sur les textes officiels pendant la réflexion de fond de chaque socle** (D-019).

**Contradictions signalées**
- Le mot « socle 2 » désignait jusqu'ici « tous les domaines » (`ROADMAP.md`, `SOCLE2_DOMAINES.md`, D-019, lots S2-x). Désormais **socle 2 = l'élève**. D-019 (public et privé ensemble) s'applique à tous les socles à partir du 2. Le nom du fichier `SOCLE2_DOMAINES.md` est conservé (historique) ; un avertissement y renvoie ici.
- `docs/ent-metier-checklist.md` classe « Compta OGEC / Paie » en « DROP » (hors cœur ENT) ; Florian les inclut dans la liste des socles : la carte les garde, en fin de parcours (Q-B, Q-C).
- L'ordre fixé place les inscriptions (3) avant le dossier administratif (4) qu'elles alimentent : règle de contournement au socle 3.

---

## 10. Trous relevés par la matrice

Ajout du 2026-11-06 (branche `ent-matrice-charlemagne`). Source : `docs/MATRICE_CHARLEMAGNE.md`, à lire avec ses limites (§0 : sources éditeur inaccessibles depuis la session de recherche, donc fonctions de Charlemagne **non vérifiées** au détail ; modules de Charlemagne établis par leur nom seulement). La carte ci-dessus n'est **pas modifiée** : ces propositions sont des **[R]** à valider.

| Trou | Objet | Où l'ajouter [R] |
|---|---|---|
| T-01, T-02 | Guichet de demandes internes et familles (routage, prise en charge, délégation) ; demandes de reprographie et d'heures supplémentaires | **Nouveau socle N1** « Demandes et flux de travail » (à défaut : 29 + 31) |
| T-03 | Construction de l'emploi du temps (génération avec contraintes) : module Emploi du temps de Charlemagne | Socle 10 (lot) ou **nouveau socle N2** ; à décider avec Q-F, **avant** J5 vu D-024. **Proposition du 2026-11-09** : socle 10, lots T6–T8b (`10_EMPLOI_DU_TEMPS.md` §11, Q-EDT1) |
| T-04, T-05 | Paie et comptabilité d'organisme : modules Paie et Comptabilité de Charlemagne | Socles 49 et 48 conservés ; Q-B et Q-C à trancher plus tôt (la checklist les classait « DROP ») |
| T-06 | Santé et sécurité au travail (registre, médecine du travail, habilitations obligatoires) | Socle 49 (sous-domaine) |
| T-07, T-08, T-09 | Accueil d'un nouvel agent, climat social anonyme, planning de service du personnel non enseignant | Socle 6 (+ 7, 15, 47) |
| T-10 | Attestations et diplômes d'établissement, chaîne de signatures, vérification publique | Socle 7 |
| T-11 | Fiches de dialogue avant le conseil de classe | Socle 23 (+ 19) |
| T-12 | Signalement bien-être confidentiel par l'élève | Socle 36 (+ 13) |
| T-13 | Classement automatique de pièces par lecture de PDF | Socle 7 (+ 28) |
| T-14, T-15 | Rattrapage des cours d'un professeur absent ; interventions transversales | Socles 15 et 10 |
| T-16, T-17, T-18 | Événements avec réponse (RSVP) et partenariats ; site public ; simulateurs de tarifs et de fournitures | Socles 19, 17 (ou **nouveau socle N3** « Présence publique »), 24 |
| T-19, T-20 | Covoiturage ; détails de voyages (devis de transport, commande cuisine, GED comptable, blog parents) | Socles 43 et 42 |
| T-21 | Ensemble scolaire / réseau d'établissements | Hors carte (§7), Q13 : à décider |
| T-22 à T-24 | Blog, wiki, forum, casier ; porte-monnaie prépayé de cantine ; permis à points | Socles 32, 40 + 25, 13 |
| T-25 | Échanges avec le rectorat / la DDEC et enquêtes de rentrée (module Rectorat de Charlemagne, contenu à préciser) | Socles 9 et 47 |
| T-26 à T-31 | Export complet d'un dossier ; garde-fous sociaux sur impayés ; pré-contrôle des exports officiels ; charge de saisie des enseignants ; sondage de créneaux ; annulation d'une bascule d'année | Socles 2 + 27, 25 + 40 + 36, 22 + 9, 8 + 20 + 47, 18, 5 |

**Question nouvelle à Florian** : Q-H — créer les socles N1, N2 (si Q-F est « oui »), N3 ? Recommandation [R] : oui pour N1 ; N2 seulement après avoir mesuré, avec le secrétariat, l'usage réel du module Emploi du temps de Charlemagne.
