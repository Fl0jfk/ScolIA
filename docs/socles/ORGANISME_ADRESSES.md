# Groupe scolaire, UAI et adresses des établissements (D-056)

**Statut :** conception (proposé, **à valider par Florian**). Aucun code, aucune migration.
**Date :** 2026-11-11 — **Branche :** `ent-conception-organisme-uai` (docs uniquement).
**Complète :** D-012 (un sous-domaine par établissement), D-013 (identité unique, passkey liée à `scolia.fr`), D-056 (question ouverte).
**À lire avec :** `docs/socles/05_STRUCTURE.md` §5, §9.1, §13 ; `02_ELEVE.md` Q-E13 ; `04_ADMINISTRATIF.md` §8.2 (`ent_organisme`) ; `28_MISE_EN_SERVICE.md` §6.1, MSE-101 à 107, Q-MSE3 ; `docs/RLS_ISOLATION.md` ; `docs/SESSION_WEB_COOKIE_ETUDE.md` ; `docs/IDENTITE_ET_PROFILS_FAMILLES.md` ; `docs/TRANSITION_NEXT_RUST.md`.

Notation : **[F]** fait vérifié dans le dépôt (chemin) ; **[H]** hypothèse ; **[R]** recommandation ; **[NV]** à vérifier (texte officiel ou documentation externe non consultés dans cette session) ; **[Q]** question pour Florian.

---

## 0. En bref

- **Le cloisonnement (RLS) reste au niveau du « locataire »**, qui devient explicitement le **groupe scolaire** : l'ensemble d'UAI qui travaillent ensemble (personnels, familles, EDT, finances communs). La colonne `etablissement_id` garde son nom dans le code (213 migrations l'utilisent) ; sa signification est précisée : *identifiant du locataire*.
- **L'UAI devient un objet de premier rang à l'intérieur du locataire** (`ent_uai`), auquel se rattachent classes, inscriptions annuelles, services d'enseignement et exports officiels. La séparation par UAI est une **règle métier et de portée**, pas une barrière RLS.
- **Le réseau** (un organisme de gestion qui porte plusieurs groupes à des endroits différents, une tutelle) reste **au-dessus** des locataires : consolidation seulement, jamais de données partagées (D-013, `28_MISE_EN_SERVICE.md` MSE-106).
- **Adresses** : chaque UAI a son adresse `<uai>.scolia.fr` (ex. `0999001a.scolia.fr`, codes fictifs), qui **renvoie (308) vers l'adresse canonique** du groupe. L'adresse canonique est celle d'un **UAI de référence** choisi par le groupe ; aucun diminutif à inventer. Un **portail neutre** `app.scolia.fr` sert d'entrée sans adresse connue (« je ne sais pas où me connecter ») et d'hôte unique pour les applis mobiles.
- `lpnb.scolia.fr` devient une **adresse historique** (alias) du groupe ; elle reste au Next.js pendant toute la cohabitation et n'est basculée que par Florian.
- Découpage en **9 lots** (§10), dont 4 avant la démo de mise en service (O1 à O4).

---

## 1. Faits relevés dans le dépôt [F]

### 1.1 Côté Rust (`ent-rust/`, branche de travail)

| Sujet | Fait | Source |
|---|---|---|
| Registre des établissements | `ent_etablissement (id, slug, nom, actif)` ; `ent_etablissement_domaine (domaine PK, etablissement_id)` ; domaine en minuscules, sans port | `migrations/096_*.sql` |
| Résolution par l'hôte | `TenantHote` lit `Host` (ou `X-Forwarded-Host` derrière un proxy de confiance, `ENT_TRUST_PROXY`) ; `ENT_TENANT_PAR_HOTE=strict\|souple` ; en strict, hôte inconnu → 404, l'`etablissement_id` du client est ignoré, une session d'un autre établissement → 401 | `crates/ent-api/src/tenant.rs:1-140`, `crates/ent-db/src/etablissement_repo.rs:96` |
| RLS | une seule variable de transaction `app.etablissement_id` posée par `TenantTx` ; politique `ent_rls_etablissement` : `etablissement_id = ent_rls_etablissement()` ; mode strict par `app.rls_strict` ; pose par `ent_rls_poser(text[])` | `migrations/100_rls_pilote.sql`, `580_rls_eleves_responsables.sql:7-30`, `crates/ent-db/src/tenant_tx.rs:62-80` |
| Ampleur | 213 fichiers de migration sur 232 mentionnent `etablissement_id` | `grep -l` sur `migrations/` |
| Comptes | `ent_user` (unique par e-mail), `ent_membership (user_id, etablissement_id, role)` ; casquettes par établissement (`ent_membership_casquette`, une session = une casquette) | `005_ent_auth.sql:14-20`, `357_casquettes_invitations.sql` |
| Sessions | `ent_session (user_id, etablissement_id, token_hash, expires_at, revoked_at, casquette)` : **une session = un établissement** ; tables sans RLS, lues avant le choix d'établissement | `005_ent_auth.sql:25-34`, `crates/ent-db/src/garde_rls_stricte.rs:11` |
| Cookie web | `__Host-ent_session ; HttpOnly ; Secure ; Path=/ ; SameSite=Lax` ; anti-CSRF : `Origin` dans une **liste statique** `config.origines` (ou `Sec-Fetch-Site: same-origin`) **et** en-tête `X-ENT-CSRF: 1` | `web/crates/ent-web-session/src/lib.rs:58-71, 161-177, 189-193` |
| Cookie SSO | `__Host-ent_oidc` (liaison du parcours au navigateur) | `crates/ent-api/src/routes.rs:2667-2669` |
| Passkeys | rattachées à la **personne** (`user_id`) ; `etablissement_id` informatif | `093_passkey_par_personne.sql` |
| WebAuthn | `ENT_RP_ID` (ex. `scolia.fr`), origines **statiques** `ENT_RP_ORIGIN(S)` sous le `rp_id`, `ENT_RP_ALLOW_SUBDOMAINS` | `crates/ent-auth/src/passkey.rs:104-213, 249` |
| SSO | fournisseurs OIDC configurés par établissement, `redirect_uri` stocké par fournisseur | `crates/ent-api/src/routes.rs:2258-2312`, `crates/ent-oidc/src/provider.rs:27, 87` |
| UAI | porté par le **site** (`ent_site.uai`) ; profil d'établissement avec UAI, secteur, contrat | `migrations/110:14-15` ; `05_STRUCTURE.md` §22 (lot S2) |
| Applis mobiles (prototypes) | adresse de base fixe ; `login(email, password, etablissementId: UUID)` : l'identifiant interne est saisi/transmis par l'appli | `apps/ios-famille/Sources/EntApiClient.swift:10, 401-413` |

### 1.2 Côté Next.js en production (lecture ciblée, aucune modification)

| Fait | Source |
|---|---|
| Un seul locataire en production, `lpnb`, **avec plusieurs sites** (`etablissement_site` : `ecole`, `college`, `lycee`… avec directeur, couleur, signature, niveaux) | `db/schema.ts:313-328` ; `TRANSITION_T0_INVENTAIRE.md` C01 |
| Classes rattachées à un site, avec le code de structure du rectorat | `db/schema.ts:744-747` (`classe_site_mapping`) |
| Circuits de validation « OGEC » (niveau groupe) **et** par site (directrice de l'école, du lycée) | `app/lib/absences-routing.test.ts:10-43` |
| Résolution du locataire par nom d'hôte (`hostnames` dans le registre) | `app/lib/tenant-registry.ts:69-99, 483` ; `app/lib/tenant-context.ts:28` |
| Précédent de redirection d'hôte : `lp.docslapro.com` → `https://lpnb.scolia.fr` en **308**, même chemin | `app/lib/legacy-hostname-redirects.ts:5-22` |

**Conséquence :** l'établissement de Florian *est déjà*, dans les faits, un groupe scolaire multi-UAI servi par un seul locataire. C'est le cas d'usage de référence, pas une exception.

### 1.3 Contradictions relevées dans la documentation

| # | Contradiction | Proposition |
|---|---|---|
| C1 | `02_ELEVE.md` Q-E13 : **un locataire peut regrouper plusieurs UAI** (fiche élève continue école → collège → lycée). `28_MISE_EN_SERVICE.md` Q-MSE3 : défaut « **un établissement par UAI**, organisme commun » ; §6.1 point 6 : chaque établissement d'un organisme est créé séparément, sans visibilité croisée | Corriger le 28 : défaut = **un locataire par groupe scolaire** quand les UAI travaillent ensemble (§5) ; locataires séparés reliés par un réseau sinon |
| C2 | « Organisme » désigne déjà, au socle 4, une **personne morale tierce** (`ent_organisme` : ASE, employeur, collectivité) ; le 05 prévoit `organisme_gestionnaire_id → ent_organisme` ; le 28 nomme `plt_etablissement_organisme` le lien « même organisme » | Ne **pas** appeler le locataire « organisme ». Vocabulaire du §2 |
| C3 | `05_STRUCTURE.md` porte l'UAI sur le **site** ; or un site peut abriter plusieurs UAI (école + collège à la même adresse) et un UAI peut s'étendre sur plusieurs sites (lycée et son annexe) | UAI = objet propre, lié aux sites en plusieurs-à-plusieurs (§6.1) ; `ent_site.uai` devient une projection |
| C4 | D-012 parle d'« établissement » pour l'unité d'adresse et de cloisonnement, alors que pour les familles et les textes un établissement est un UAI | Glossaire §2 ; D-012 à reformuler en « un sous-domaine par locataire, alias par UAI » |

---

## 2. Vocabulaire proposé [R]

| Terme (docs et interface) | Sens | Dans le code | Exemple |
|---|---|---|---|
| **Locataire** (docs techniques) / **l'établissement** ou **le groupe scolaire** (interface) | Unité de **travail et de cloisonnement** : une communauté de personnes, de données et de règles. Tout ce qui est RLS. | `ent_etablissement`, colonne `etablissement_id` (nom inchangé) | « Groupe scolaire Saint-X » |
| **UAI** (ou « établissement officiel ») | Unité administrative immatriculée par le ministère : école, collège, lycée. Porte les exports officiels, l'identité imprimée (bulletins, certificats), le chef d'établissement de l'UAI | `ent_uai` (nouveau) | « École Saint-X, 0999001A » |
| **Site** | Lieu physique (adresse, bâtiments, salles) | `ent_site` (existant) | « Campus Nord » |
| **Entité juridique** | Personne morale qui emploie, facture, tient la comptabilité : OGEC, association, EPLE, commune (école publique) | `ent_entite_juridique` (nouveau, lot O8) | « OGEC Saint-X, SIRET … » |
| **Réseau** | Regroupement de **plusieurs locataires** (organisme de gestion multi-groupes, tutelle, direction diocésaine, groupement d'EPLE) pour des tableaux consolidés, sans aucune donnée personnelle partagée | `plt_reseau`, `plt_reseau_membre` (renommage proposé de `plt_etablissement_organisme`, 28 §13) | « Réseau des OGEC du diocèse » |
| **Organisme** | Inchangé : tiers du dossier administratif (socle 4) | `ent_organisme` | « ASE du département » |

Dans l'interface, le mot « groupe scolaire » n'apparaît **que** si le locataire compte plusieurs UAI ; sinon on affiche simplement le nom de l'établissement. Les familles ne voient jamais le mot « locataire ».

---

## 3. Personas

| Persona | Situation | Ce qui lui fait perdre du temps aujourd'hui | Ce qu'il redoute | Ce qui le ravirait |
|---|---|---|---|---|
| **Claire, chef d'établissement coordinatrice** d'un groupe école + collège + lycée privé sous contrat (3 UAI, un OGEC) | Signe pour le lycée, coordonne le tout | Trois listes de personnels, trois annuaires de familles, des exports refaits à la main par UAI | Un export SIECLE ou une enquête de rentrée qui mélange les UAI | Une seule maison, et à chaque export « pour quel UAI ? » avec les bons effectifs |
| **Mme Martin, directrice de l'école** du même groupe | Chef de l'UAI école, enseignante à mi-temps | Passer par « l'adresse du lycée » ; voir des menus de lycée | Qu'on lui demande de valider des absences du lycée | Voir son école d'abord, le groupe quand elle en a besoin |
| **Karim, secrétaire unique** du groupe | Inscriptions des 3 UAI, accueil téléphonique | Ressaisir une famille pour chaque enfant selon l'UAI | Créer un doublon de famille | Une fratrie école/collège/lycée = un seul foyer, une seule fiche par enfant, l'UAI déduit de la classe |
| **Sophie, comptable de l'OGEC** | Facturation, paie des personnels OGEC | Une facture par UAI alors que la famille paie tout ensemble | Mélanger les comptes de deux entités juridiques | Une facture familiale unique, ventilée par UAI pour l'analytique |
| **M. Lefèvre, professeur de mathématiques** en service partagé collège et lycée du groupe | Deux UAI au rectorat, un seul EDT réel | Deux comptes, deux cahiers de textes | Que l'appel du collège n'apparaisse pas dans son EDT | Un EDT, un appel, un cahier de textes ; ses services comptés par UAI pour STS |
| **Nadia, CPE** du collège et du lycée | Vie scolaire commune | Fiches incidents par UAI qui ne se voient pas | Rater qu'un frère au collège est aussi suivi | Vision par élève, filtre par UAI |
| **Les Durand, parents** de trois enfants (CE2, 5ᵉ, 1ʳᵉ) dans le groupe | Un seul foyer | Trois codes, trois adresses, trois factures | Se tromper d'adresse et croire le site en panne | Une connexion, trois enfants, quel que soit le lien cliqué |
| **Julien, parent séparé** : un enfant dans le groupe Saint-X, un autre dans un autre établissement ScolIA d'une autre ville | Deux locataires | Deux comptes | Que l'un des établissements apprenne quelque chose de l'autre | Une passkey, un sélecteur d'espaces dans l'appli |
| **Léa, élève** qui passe de 3ᵉ (collège) en 2ⁿᵈᵉ (lycée) du même groupe | Changement d'UAI | Nouveau compte, historique perdu | Perdre ses documents et son ENT | Rien à faire : même compte, même dossier, nouvel UAI |
| **Thomas, référent numérique** | SSO Google Workspace du groupe (`@saint-x.fr`) | Un SSO à configurer par UAI | Une adresse de retour SSO cassée par une redirection | Un SSO pour le groupe, sur une seule adresse |
| **Le DPO** du groupe | Registre de traitements | Trois registres quasi identiques | Un partage de données entre deux responsables de traitement sans convention | Un registre par locataire, avec les UAI et entités juridiques listés |
| **Rectorat / DSDEN** (destinataire, pas utilisateur) | Reçoit exports et enquêtes par UAI | Fichiers incohérents entre UAI | Élèves comptés deux fois | Un fichier par UAI, conforme |
| **Opérateur ScolIA** (plateforme) | Crée les locataires, gère les domaines | Inventer et réserver des diminutifs | Un UAI attribué au mauvais client, une adresse détournée | Saisir les UAI, les adresses se créent seules |
| **Un groupe public : cité scolaire** (collège + lycée publics, deux UAI, souvent un seul EPLE ou deux EPLE avec services mutualisés) [H] | Agents de l'État et de la collectivité | — | Qu'un EPLE accède aux données de l'autre sans base légale | Le même fonctionnement qu'un groupe privé, si la base légale le permet [Q-O4] |

---

## 4. Ce qui doit être commun et ce qui doit rester distinct dans un groupe

| Commun au groupe (un locataire) | Distinct par UAI (dans le même locataire) |
|---|---|
| Identités, comptes, passkeys, sessions (une seule connexion) | Identité imprimée : nom officiel, adresse, logo, chef de l'UAI, signature (bulletins, certificats de scolarité, conventions) |
| Personnes, foyers, liens responsables, payeurs (une fratrie = un foyer) | Inscription annuelle de l'élève (l'UAI de sa classe), effectifs, constat de rentrée |
| Personnels, rôles, casquettes ; services partagés entre UAI | Services d'enseignement comptés par UAI (STS) [NV : modalités exactes de STS-Web et de son successeur] |
| Emploi du temps, salles, sites, calendrier (avec exceptions ciblées par UAI) | Imports et exports officiels : SIECLE/BEE, STS, LSU, Affelnet, Parcoursup, examens, enquêtes [NV] |
| Messagerie, annonces, agenda (ciblables par UAI) | Circuits de validation propres (« la directrice valide les absences des professeurs de l'école ») |
| Facturation familiale, encaissements, comptabilité de l'entité juridique | Analytique par UAI ; contrats d'association (privé sous contrat) par UAI ; budget de l'EPLE (public) [NV] |
| Vie scolaire, discipline, santé (droits par rôle, comme aujourd'hui) | Règlement intérieur, horaires, grille propres possibles (déjà ciblables par site/niveau, 05 lot S7) |
| Registre RGPD du locataire | Mentions par UAI dans le registre (qui est responsable de quoi) |

Règle [R] : **ce qui est distinct par UAI se règle par une colonne `uai_id` et des portées, jamais par un second locataire**.

---

## 5. Critère : un locataire ou plusieurs ? [R]

Un ensemble d'UAI forme **un seul locataire** si les trois conditions sont réunies :

1. **Base légale du partage** : une seule entité juridique responsable de traitement (cas courant : un OGEC pour école + collège + lycée), **ou** plusieurs responsables liés par un accord écrit de responsabilité conjointe (RGPD art. 26) validé par le DPO [Q-O4] ;
2. **Travail commun réel** : personnels ou familles partagés, EDT, salles ou finances communs ;
3. **Volonté des chefs d'établissement** de chaque UAI (signée à la mise en service, 28 MS1).

Sinon : **locataires séparés**, éventuellement membres d'un même **réseau** (consolidation sans données personnelles).

Exemples : école + collège + lycée d'un même OGEC sur un site → **un** locataire. Deux groupes du même OGEC dans deux villes, sans personnel commun → **deux** locataires, **un** réseau. Collège public + école publique voisine (commune) → **deux** locataires (responsables de traitement distincts, pas de travail commun au sens ci-dessus).

---

## 6. Modèle de données cible (pseudo-DDL, à préciser par les lots)

### 6.1 UAI et sites

```
ent_uai                                   -- RLS d'établissement (locataire), comme les autres
  id UUID PK, etablissement_id UUID NOT NULL → ent_etablissement
  code TEXT NOT NULL                      -- 7 chiffres + 1 lettre, stocké en majuscules
       CHECK (code ~ '^[0-9]{7}[A-Z]$')   -- + contrôle de la lettre clé [NV] (§6.4)
  nom_officiel TEXT NOT NULL, nom_usuel TEXT NULL
  nature TEXT NOT NULL                    -- ecole_maternelle, ecole_elementaire, ecole_primaire, college, lycee_gt… (D-052 : V1 sans LP ni post-bac)
  secteur TEXT NOT NULL CHECK (secteur IN ('public','prive_sous_contrat','prive_hors_contrat'))
  contrat TEXT NULL CHECK (contrat IN ('association','simple'))   -- seulement sous contrat
  academie TEXT NULL, departement TEXT NULL
  entite_juridique_id UUID NULL → ent_entite_juridique (lot O8)
  chef_personne_id UUID NULL → ent_personne   -- chef de l'UAI (signature, validations)
  adresse_id UUID NULL                    -- modèle d'adresse du socle 4
  ouvert_le DATE NOT NULL, ferme_le DATE NULL -- fermeture datée, jamais de suppression
  est_reference BOOLEAN NOT NULL DEFAULT false -- au plus un par locataire (index partiel)
  UNIQUE (code)                           -- un UAI n'appartient qu'à un seul locataire, sur toute la plateforme
  UNIQUE (etablissement_id, id)

ent_uai_site                              -- plusieurs-à-plusieurs
  etablissement_id, uai_id, site_id, du DATE, au DATE NULL
  FK composites (etablissement_id, uai_id), (etablissement_id, site_id)
```

- **Unicité plateforme du code UAI** : la contrainte `UNIQUE (code)` traverse les locataires. Sous RLS stricte, l'insertion d'un doublon échoue sur la contrainte sans révéler à quel locataire il appartient ; le message est « UAI déjà déclaré sur ScolIA, contactez le support » (28 CL28-02). La vérification passe par une fonction de plateforme `SECURITY DEFINER` qui ne renvoie qu'un booléen.
- `ent_site.uai` (texte, existant) devient une **projection** maintenue depuis `ent_uai_site` le temps que les lecteurs basculent, puis est retiré (même méthode que `05_STRUCTURE.md` M12).

### 6.2 Rattachements à l'UAI

| Objet | Rattachement | Règle |
|---|---|---|
| Classe (`ent_classe`) | `uai_id NOT NULL` (FK composite) | Une classe appartient à un UAI pour une année ; une classe à plusieurs niveaux reste dans un UAI |
| Groupe d'enseignement | **pas** d'UAI propre : un groupe peut mêler des élèves de deux UAI (option mutualisée collège/lycée [H], rare) ; l'UAI de l'élève vient de sa classe | Exports par UAI : par l'élève, pas par le groupe |
| Inscription annuelle / scolarité | UAI **dérivé** de la classe à la date ; pour un élève sans classe (préinscription) : `uai_vise_id` | L'historique conserve l'UAI de chaque année (passage 3ᵉ → 2ⁿᵈᵉ = changement d'UAI, pas une sortie : 02 §Continuité) |
| Service d'enseignement | UAI dérivé de la classe ou du groupe ; pour un groupe mixte, ventilation par UAI calculée | STS : heures par UAI [NV] |
| Événements, exports, documents officiels | `uai_id` sur le modèle de document et sur chaque production | Un export ne contient que des lignes de son UAI ; contrôle croisé avant envoi |
| Facture, écriture comptable | `entite_juridique_id` + analytique `uai_id` | Une facture familiale peut porter des lignes de plusieurs UAI de la **même** entité juridique ; jamais deux entités juridiques sur une facture |

### 6.3 Portée par UAI des rôles

- Les actions gardent leur portée « établissement » (= locataire) par défaut. On ajoute une **portée « UAI désignés »** au même titre que « sites désignés » (`05_STRUCTURE.md` §13, `calendrier.gerer`) : une affectation de rôle peut être limitée à un ou plusieurs UAI (directrice d'école : validation des absences des personnels de l'école, signature des bulletins de l'école).
- **Défaut** : pas de limitation (le groupe travaille ensemble) ; seules les actions **officielles et signées** sont par défaut réservées au chef de l'UAI concerné (signature, exports, PV) [R], ce qui correspond au circuit observé dans Next.js (§1.2).
- Filtre de confort « mon UAI » dans les écrans (mémorisé par personne), distinct de la portée de sécurité.

### 6.4 Format et contrôle du code UAI

- Format : 7 chiffres (les 3 premiers codent le département ou l'académie [NV]) + 1 lettre clé calculée à partir des chiffres (reste de la division par 23, alphabet sans I, O, Q [NV : algorithme à confirmer sur une source officielle avant le lot O1]). Saisie insensible à la casse, stockage en majuscules, **nom d'hôte en minuscules** (`0999001a`).
- Vérification d'existence dans l'annuaire officiel des établissements par le socle 9 quand il sera branché (28 MSE-102) ; en attendant, contrôle de format et de clé, et confirmation de l'opérateur.

### 6.5 Domaines

```
ent_etablissement_domaine  (existant, enrichi)
  domaine TEXT PK, etablissement_id
  + nature TEXT NOT NULL CHECK (nature IN ('canonique','uai','historique'))
  + uai_id UUID NULL                 -- renseigné pour nature = 'uai'
  + cree_par, cree_le, retire_le NULL  -- un domaine retiré reste réservé (jamais réattribué à un autre locataire)
  index unique partiel : un seul 'canonique' actif par locataire
```

Domaines de plateforme (hors table, configuration) : `app.scolia.fr` (portail neutre et API mobile), `scolia.fr` / `www.scolia.fr` (site public, fichiers `/.well-known/` des applis). Mots réservés étendus (28 S-560) : `www`, `admin`, `api`, `app`, `auth`, `support`, `status`, `demo`, `aide`, `mail`, `static`, `fichiers`, et **tout libellé au format UAI** (`^[0-9]{7}[a-z]$`) réservé aux UAI déclarés.

---

## 7. Options comparées

Deux questions sont mêlées dans D-056 : **où passe la frontière RLS** (UAI ou groupe) et **quelle adresse** l'utilisateur tape. On compare cinq options.

- **O-1 — Cloisonnement par UAI, organisme au-dessus qui partage.** Chaque UAI est un locataire ; un « organisme » regroupe des UAI et autorise des données communes.
- **O-2 — Cloisonnement par groupe, adresse par groupe avec diminutif** (D-012 tel quel : `lpnb.scolia.fr`).
- **O-3 — Cloisonnement par groupe, adresse unique `app.scolia.fr`**, choix de l'établissement à la connexion.
- **O-4 — Cloisonnement par groupe, une adresse par UAI** qui renvoie vers l'adresse canonique du groupe (UAI de référence).
- **O-5 — Cloisonnement par groupe, une adresse par UAI qui sert directement** (sans redirection, chaque UAI est une origine à part entière).

### 7.1 Tableau de synthèse

| Critère | O-1 UAI + organisme | O-2 diminutif | O-3 adresse unique | O-4 UAI → canonique | O-5 UAI sans redirection |
|---|---|---|---|---|---|
| Groupe = un tout | difficile (partage à construire partout) | oui | oui | oui | oui |
| Pas de diminutif inventé | oui | **non** | oui | oui | oui |
| RLS | **refonte** (§7.2) | inchangée | inchangée | inchangée | inchangée |
| Cookies | un par UAI : 3 connexions pour les Durand | un par groupe | un pour toute la plateforme | un par groupe | **un par UAI** : 3 connexions |
| Passkeys (D-013) | ok | ok | ok | ok (origines dynamiques) | ok (origines dynamiques) |
| Applis mobiles | sélecteur d'UAI | rattachement par lien | naturel | naturel (via `app.scolia.fr`) | naturel (via `app.scolia.fr`) |
| SSO | un IdP par UAI | un par groupe | un global, choix du groupe avant | un par groupe, adresse canonique | ambigu (quelle adresse de retour ?) |
| Lien reconnaissable (anti-hameçonnage, R-MSE15) | oui | oui | **faible** (même adresse pour tous) | oui | oui |
| Isolation navigateur entre locataires | par hôte | par hôte | **aucune** (même origine) | par hôte | par hôte |
| Migration `lpnb.scolia.fr` | 3 locataires à extraire d'un seul | rien | redirection | alias historique | alias historique |
| Reprise Next.js | **découpage** des données d'un locataire en trois | directe | directe | directe | directe |
| Coût | très élevé | nul | moyen | moyen | moyen |
| **Verdict** | écartée | écartée (refus de Florian) | **complément** | **recommandée** | écartée |

### 7.2 O-1 — Cloisonnement par UAI, organisme au-dessus

- **RLS** : la variable unique `app.etablissement_id` devient une liste (`app.etablissements` = UAI de l'organisme accessibles à la session) ; les politiques passent à `etablissement_id = ANY(...)` sur les 98+ tables en RLS forcée. Mais les objets **communs** (personne, foyer, personnel, salle, séance d'EDT, facture familiale) doivent appartenir à **un** UAI : lequel pour une fratrie école/lycée ? Il faut soit les dupliquer (doublons interdits par D-013), soit créer une « couche organisme » avec ses propres tables et une seconde politique. Les clés étrangères composites `(etablissement_id, id)` partout (05 §9, 04 §8) **interdisent** qu'une classe du lycée ait un professeur rattaché au collège : il faudrait les défaire.
- **Cookies** : une session par UAI (hôte) ou une session « organisme » à inventer ; dans les deux cas le modèle `ent_session.etablissement_id` ne suffit plus.
- **Passkeys** : inchangées (personne).
- **Mobile** : sélecteur d'UAI, données d'un parent dispersées.
- **SSO** : fournisseurs dupliqués par UAI ou montés au niveau organisme.
- **`lpnb.scolia.fr`** et **reprise Next.js** : le locataire unique de Next doit être **découpé** en trois (personnels, foyers, factures répartis ou dupliqués) : contraire à la règle « rien de ce qui marche ne doit être perdu » (D-035).
- **Seul avantage** : une frontière technique forte entre UAI. Elle n'a de sens que si les UAI ne doivent *pas* partager — et dans ce cas l'option recommandée les met dans des **locataires séparés** (§5). **Écartée.**

### 7.3 O-2 — Adresse par groupe avec diminutif (statu quo D-012)

- RLS, cookies, sessions, SSO, mobile : **rien à changer** (c'est le code actuel).
- Inconvénient : il faut inventer, réserver et faire accepter un diminutif (`lpnb`) ; collisions entre homonymes (« Saint-Joseph » existe des centaines de fois) ; diminutif ambigu pour les familles. C'est précisément ce que Florian veut éviter. **Écartée comme schéma principal**, mais le diminutif existant reste comme **alias historique** dans l'option recommandée.

### 7.4 O-3 — Adresse unique `app.scolia.fr`, choix à la connexion

- **RLS** : inchangée (la session porte le locataire, `TenantTx` le pose comme aujourd'hui).
- **Cookies** : un seul hôte. Soit une session par locataire avec **un cookie par locataire** (noms distincts, `__Host-ent_session_<id>`), soit une session « personne » qui change de locataire (refonte de `ent_session`). Dans les deux cas, **toutes les pages de tous les locataires partagent la même origine** : une faille d'injection de script dans l'espace d'un locataire donne accès aux sessions des autres locataires de la même personne ; on perd la défense en profondeur de D-012.
- **Passkeys** : simples (une seule origine).
- **Mobile** : naturel (un hôte, sélecteur d'espaces).
- **SSO** : le fournisseur dépend du locataire ; il faut **choisir le groupe avant** de se connecter (bouton « Se connecter avec Google » qui ne sait pas où aller), ou découvrir par le domaine de l'e-mail.
- **Hameçonnage** : tous les messages d'activation pointent vers la même adresse ; « l'adresse de votre établissement » n'est plus un repère (28 R-MSE15).
- **`lpnb.scolia.fr`** : redirection vers `app.scolia.fr` ; lien profond à reconstruire (le chemin ne dit plus le locataire).
- **Reprise Next.js** : directe.
- Bonne réponse au cas « je ne connais pas l'adresse » et aux applis mobiles. **Retenue comme complément** (portail neutre et API mobile), pas comme schéma principal du web.

### 7.5 O-4 — Une adresse par UAI qui renvoie vers l'adresse canonique (recommandée)

Chaque UAI déclaré a `<uai>.scolia.fr`. Une seule adresse par locataire est **canonique** : celle de l'**UAI de référence** du groupe (choisi à la mise en service, modifiable avec redirection permanente de l'ancienne, 28 MSE-107). Les autres adresses (autres UAI, historiques comme `lpnb.scolia.fr`) répondent par **308** vers l'adresse canonique, **même chemin et même requête**, **avant toute lecture ou pose de cookie**.

- **RLS** : **inchangée**. `TenantHote` résout l'alias en locataire puis redirige ; aucune transaction n'est ouverte sur un alias.
- **Cookies** : `__Host-ent_session` et `__Host-ent_oidc` vivent sur l'hôte canonique seulement → **une connexion pour tout le groupe** (les Durand). Règle testée : aucune réponse d'un alias ne contient `Set-Cookie`. Le préfixe `__Host-` interdit l'attribut `Domain` : un sous-domaine ne peut pas poser un cookie de session valable pour un autre (protection contre l'injection de cookie entre sous-domaines).
- **CSRF entre locataires** : tous les `*.scolia.fr` sont **le même site** au sens des navigateurs ; `SameSite=Lax` ne protège donc pas un locataire contre une page d'un autre locataire. La garde existante (`X-ENT-CSRF` impose un pré-vol CORS ; `Origin` contrôlé) doit passer d'une **liste statique** d'origines à la règle stricte **`Origin` = `https://` + hôte canonique de la requête** (§1.1, `ent-web-session/src/lib.rs:161-177`). Pas de CORS ouvert entre sous-domaines.
- **Ne pas inscrire `scolia.fr` à la Public Suffix List** : cela isolerait les sous-domaines (sites distincts) mais rendrait `scolia.fr` inutilisable comme `rp_id` de passkey (un suffixe public ne peut pas l'être), contraire à D-013 [NV : à confirmer sur la spécification WebAuthn].
- **Passkeys** : `rp_id = scolia.fr` inchangé (D-013) ; une passkey enrôlée sur un groupe sert sur tous. La liste **statique** `ENT_RP_ORIGINS` ne tient plus (des adresses naissent à chaque UAI déclaré) : on active `allow_subdomains` (déjà prévu, `passkey.rs:249`) **et** on ajoute un contrôle applicatif : l'origine signée par le navigateur (`clientDataJSON.origin`) doit être **exactement** l'hôte canonique de la requête ou `app.scolia.fr`, et la cérémonie (défi stocké) est liée à cet hôte. Un alias ne mène jamais de cérémonie (il redirige avant).
- **Applis mobiles** : un seul hôte d'API, `app.scolia.fr` (Bearer, D-009) : le locataire vient de la **session**, jamais du client ; l'appli garde une session par locataire (trousseau) et un sélecteur d'espaces (Julien). Domaine associé `webcredentials:scolia.fr` (passkeys) et liens universels `*.scolia.fr` (invitation reçue par e-mail sur une adresse UAI → ouverture de l'appli) [NV : prise en charge des jokers par Apple `applinks:*.scolia.fr` et par Android App Links, et emplacement des fichiers `apple-app-site-association` / `assetlinks.json`].
- **SSO (OIDC Google/Microsoft)** : fournisseur configuré par locataire (existant) ; `redirect_uri` = **adresse canonique** (jamais un alias : la redirection perdrait le `__Host-ent_oidc`). Changer d'UAI de référence oblige à mettre à jour l'adresse de retour chez le fournisseur : l'écran de changement le rappelle et liste les fournisseurs concernés (cas limite CL-O12). **ÉduConnect / GAR** (D-029, plus tard) : l'identité fédérée porte des UAI [NV] ; le registre `ent_uai` (UAI unique sur la plateforme) donne directement le locataire ; un seul point de retour de plateforme (`app.scolia.fr`) puis passage à l'adresse canonique (§8.3).
- **Hameçonnage** : chaque message porte l'adresse de l'UAI ou du groupe, reconnaissable ; l'aide « reconnaître un vrai message » explique la forme `<code UAI>.scolia.fr` (le code UAI figure sur les documents officiels de l'établissement).
- **`lpnb.scolia.fr`** : enregistré comme domaine **historique** du locataire repris (§9).
- **Reprise Next.js** : directe (un locataire Next → un locataire Rust ; ses sites → UAI et sites, §9.2).
- **Ce qu'on accepte** : savoir qu'un UAI est client de ScolIA est public (l'adresse répond) ; ce n'est pas une donnée personnelle et les établissements l'annoncent eux-mêmes à leurs familles.

### 7.6 O-5 — Une adresse par UAI qui sert directement

Mêmes effets que O-4 sauf : chaque adresse UAI est une origine à part, donc **un cookie et une connexion par UAI** (les Durand se connectent trois fois, ou l'on construit un partage de session entre sous-domaines, ce que `__Host-` interdit) ; le SSO doit déclarer trois adresses de retour ; les liens envoyés depuis l'école et ceux du lycée ouvrent deux sessions différentes du même locataire, et une session d'école qui affiche un enfant du lycée brouille le repère « adresse = UAI ». **Écartée.**

---

## 8. Option recommandée en détail : O-4 + portail neutre (O-3 en complément) [R]

### 8.1 Résolution d'une requête

1. Hôte normalisé (minuscules, sans port ni point final ; `X-Forwarded-Host` seulement derrière le proxy de confiance, inchangé).
2. Recherche dans `ent_etablissement_domaine` (domaine actif, locataire actif) :
   - **canonique** → traitement normal (comportement actuel de `TenantHote`) ;
   - **uai** ou **historique** → `308` vers `https://<canonique><chemin>?<requête>`, `Cache-Control: no-store` les premiers mois (pour pouvoir changer de canonique), aucun cookie, aucune transaction ;
   - **retiré** → page « cet établissement a changé d'adresse » si un successeur existe, sinon 404 générique ;
   - **inconnu** → 404 générique (mode strict), identique pour un libellé au format UAI non déclaré et pour tout autre libellé.
3. `app.scolia.fr` → mode **portail** (§8.3).

Exception aux redirections : les sondes `/health`, `/ready`, `/metrics` (déjà exemptées) et `/.well-known/` (servis par `scolia.fr`, pas par les locataires).

### 8.2 Adresse canonique

- **Par défaut** : l'UAI de référence est celui que le groupe désigne à la mise en service (suggestion : l'UAI qui porte la direction coordinatrice, sinon le premier saisi). Un locataire mono-UAI a donc pour adresse son UAI, sans choix à faire.
- **Changement** (opérateur, à la demande d'un responsable du locataire, passkey récente) : la nouvelle adresse devient canonique, l'ancienne passe en `uai` (alias) ; les sessions ouvertes sur l'ancienne expirent naturellement (2 h, D-011) ; les liens anciens continuent de marcher. Rappel SSO (§7.5). Événement `etablissement.adresse_canonique_changee`.
- **Fermeture de l'UAI de référence** : l'écran de fermeture d'un UAI (05 STR-05 étendu) impose de désigner une nouvelle référence avant de fermer. L'adresse de l'UAI fermé reste un alias **à vie** pour ce locataire.
- **Pas de diminutif nouveau** sur la plateforme. Un diminutif hérité (`lpnb`) reste un alias historique ; il peut redevenir canonique sur demande explicite [Q-O2].

### 8.3 Portail neutre `app.scolia.fr`

- **Pour qui** : la personne qui ne connaît pas l'adresse ; les personnes rattachées à plusieurs locataires (Julien, professeurs en poste dans deux groupes) ; les applis mobiles ; les retours de fédération de plateforme (ÉduConnect plus tard).
- **Parcours web** : passkey découvrable (aucun e-mail à saisir ; ou e-mail mémorisé) → **après authentification seulement**, liste des espaces de la personne (ses rattachements, D-013 : usage « connexion » explicitement prévu) → choix → **code de transfert** à usage unique (≤ 60 s, lié au locataire, à la personne et à la casquette choisie, haché en base, consommé une fois) → `https://<canonique>/connexion/transfert?code=…` qui ouvre la session et pose `__Host-ent_session` sur l'adresse canonique. Le portail lui-même ne garde qu'une session de portail courte (choix d'espace, gestion de ses passkeys), **sans aucun accès aux données d'un locataire**.
- **Recherche publique d'établissement** (avant connexion) : par nom, ville ou code UAI, parmi les locataires qui l'ont accepté ; renvoie l'adresse. Aucune donnée personnelle, aucun indice sur les rattachements d'une personne.
- **Sécurité du transfert** : protection contre la fixation de session (le code ne peut pas être forgé ni rejoué ; il est lié au navigateur par un cookie `__Host-` du portail vérifié au moment de l'émission, et la page d'arrivée exige `Sec-Fetch-Site` cohérent) ; refus si la personne n'a pas (ou plus) d'accès ouvert au locataire ; journal `auth.transfert_portail`. Revue de sécurité indépendante obligatoire (lot O5).
- **Pas d'agrégation de données entre locataires** dans le portail (pas de « tous mes messages ») : ce serait un croisement contraire à D-013. L'appli mobile peut, elle, afficher les espaces côte à côte, chacun avec sa propre session.

### 8.4 Applis mobiles

- Hôte unique `app.scolia.fr` ; connexion par passkey (`rp_id scolia.fr`) → choix de l'espace → session Bearer **par locataire** (une ligne `ent_session` chacune, comme aujourd'hui) ; l'appli **ne transmet plus** d'`etablissement_id` (le prototype le fait, §1.1) : en mode strict, le serveur l'ignore déjà sur un hôte canonique ; sur `app.scolia.fr`, il est pris dans la session.
- Rattachement d'un nouvel espace (D-012) : lien d'invitation (ouvre l'appli par lien universel), QR code remis par l'établissement, recherche par nom, ville ou **code UAI** (que les familles trouvent sur les documents de l'établissement).
- Notifications poussées : chaque notification porte le locataire ; l'ouverture bascule sur le bon espace.

### 8.5 Exports officiels par UAI

- Chaque écran d'export officiel commence par **« Pour quel UAI ? »** (pré-rempli si la personne n'a la portée que d'un UAI) ; aperçu des effectifs de l'UAI ; contrôle bloquant : aucune ligne d'un autre UAI, aucun élève compté dans deux UAI à la même date.
- Imports (SIECLE/BEE, STS) : le fichier porte un UAI [NV] ; il est **rapproché** de `ent_uai` ; un fichier d'un UAI inconnu du locataire est refusé avec un message clair (« ce fichier concerne l'UAI 0999004D, qui n'est pas déclaré dans votre groupe »).
- D-022 inchangé : exports réservés aux administratifs ; portée UAI en plus.

---

## 9. Existant : `lpnb.scolia.fr` et reprise Next.js

### 9.1 Cohabitation 2027-2028 (D-034, D-035)

- Pendant toute la cohabitation, **`lpnb.scolia.fr` reste servi par le Next.js** (intouchable ; seul Florian décide de l'arrêt de chaque module). L'ENT Rust est servi sur l'**adresse canonique UAI** du groupe (libre aujourd'hui) et sur les autres adresses UAI (alias). C'est un avantage concret de O-4 : aucun conflit d'adresse pendant le doublon.
- Passage d'un module Next vers son équivalent Rust : par des **liens** (le menu Next pointe vers l'adresse Rust, modification que seul Florian peut faire côté Next) ; **pas de partage de session** entre Next (cookies Better Auth) et Rust : deux connexions pendant la cohabitation, à annoncer [R]. Les passkeys Rust (`rp_id scolia.fr`) ne dépendent pas du Next.
- **Fin de cohabitation** (action de Florian, hors pilote) : `lpnb.scolia.fr` est enregistré comme domaine **historique** du locataire Rust et le routage (DNS ou proxy) bascule ; les anciens liens Next (`/eleves/…`) n'ont pas les mêmes chemins : table de correspondance des chemins les plus utilisés (liens dans les e-mails anciens, favoris), le reste vers l'accueil [R]. Même mécanisme que le précédent `lp.docslapro.com` → `lpnb.scolia.fr` (§1.2).
- Points d'exploitation pour Florian [Q-O5] : certificat joker `*.scolia.fr` (validation DNS), enregistrement DNS joker vers le frontal Rust **sans** capter `lpnb` (enregistrement explicite prioritaire sur le joker), en-tête HSTS `includeSubDomains` à poser seulement après vérification que tous les sous-domaines existants (dont `lpnb`) sont en HTTPS.

### 9.2 Reprise des données (T0, `TRANSITION_T0_INVENTAIRE.md` C01)

| Next.js | ScolIA | Règle |
|---|---|---|
| `etablissement` (slug `lpnb`) | `ent_etablissement` (un locataire) + domaine `historique` `lpnb.scolia.fr` | `slug` conservé comme information |
| `etablissement_site` (`ecole`, `college`, `lycee`…, directeur, couleur, signature, niveaux) | `ent_uai` (un par UAI réel) + `ent_site` (lieux) + `ent_uai_site` | Le « site » Next mêle UAI et lieu : chaque site Next devient un **UAI** si un code UAI lui correspond (fourni par Florian ou par les fichiers SIECLE), sinon un **site** ; directeur → `chef_personne_id` ; signature → document du socle 7 ; couleur → paramètre de l'UAI |
| `classe_site_mapping` (classe → site, code de structure du rectorat) | `ent_classe.uai_id` | Rapprochement par le site Next devenu UAI ; rejets listés |
| Circuits « OGEC » / par site (absences, validations) | Portée « UAI désignés » (§6.3) et entité juridique (O8) | Reprise comme paramétrage, pas comme code (D-035) |
| Registre `tenants/index.json`, secrets par établissement | **non repris** (28 MSE-D07) | Établissements en table, secrets dans le gestionnaire de secrets |

Les codes UAI réels du groupe de Florian ne sont pas dans ce document (non nécessaires à la conception) ; ils seront saisis à la mise en service [Q-O1].

---

## 10. Lots

| Lot | Contenu | Dépend de | Critères de fin (tests) | Taille |
|---|---|---|---|---|
| **O0** Décisions | Validation de ce document par Florian ; mise à jour de D-012 (« un sous-domaine canonique par locataire, alias par UAI ») ; correction du 28 (Q-MSE3, §6.1 point 6 ; renommage « organisme » → « réseau ») et du 05 (UAI ≠ site) | — | — | S |
| **O1** Registre UAI | `ent_uai`, `ent_uai_site`, contrôle de format et de clé [NV], unicité plateforme par fonction booléenne, projection `ent_site.uai`, UAI de référence (au plus un), fermeture datée, écran « Établissement » du 05 étendu (S12b), événements | O0, 05 S2 | RLS stricte sur les 2 tables ; UAI d'un autre locataire refusé **sans** oracle ; un seul UAI de référence ; fermeture de la référence refusée sans successeur | M |
| **O2** Domaines et redirections | colonnes `nature`, `uai_id`, `retire_le` ; création automatique du domaine `uai` à la déclaration d'un UAI ; middleware : 308 avant cookie et transaction ; mots réservés et libellés au format UAI ; changement d'adresse canonique (opérateur) | O1 | aucun `Set-Cookie` sur un alias (test) ; chemin et requête conservés ; hôte inconnu au format UAI = même 404 que tout hôte inconnu ; domaine retiré jamais réattribué | M |
| **O3** Origines dynamiques | WebAuthn : `allow_subdomains` + contrôle origine signée = hôte canonique de la requête ; anti-CSRF : `Origin` = hôte canonique (fin de la liste statique) ; démarrage refusé si `ENT_RP_ID` ≠ domaine de plateforme | O2 | passkey enrôlée sur le groupe A utilisable sur le groupe B ; cérémonie commencée sur A et terminée sur B refusée ; POST depuis l'origine d'un autre locataire refusé | M |
| **O4** Rattachement métier à l'UAI | `ent_classe.uai_id` (FK composite, `NOT VALID` puis validée), UAI dérivé pour scolarité et services, portée « UAI désignés » des affectations de rôle, filtre « mon UAI » | O1, 05, 06 | passage 3ᵉ → 2ⁿᵈᵉ sans sortie ; directrice d'école limitée à l'école pour les actions signées ; aucune classe sans UAI après reprise | L |
| **O5** Portail neutre et mobile | `app.scolia.fr` : connexion, liste des espaces, code de transfert, recherche publique ; API mobile sur hôte neutre (locataire tiré de la session) ; retrait de l'`etablissement_id` des applis | O2, O3, socle 11 | revue de sécurité indépendante (`review-sonnet`) ; code rejoué, expiré, d'un autre navigateur : refusé ; aucune donnée de locataire lisible depuis le portail | M |
| **O6** SSO et fédérations | `redirect_uri` canonique imposée ; rappel au changement d'adresse ; plus tard ÉduConnect/GAR : correspondance UAI → locataire par `ent_uai` [NV] | O2 ; D-029 | retour SSO sur un alias impossible ; changement de canonique liste les fournisseurs à mettre à jour | S puis M |
| **O7** Reprise `lpnb` | correspondance site Next → UAI / site, `classe_site_mapping` → `uai_id`, domaine historique, table de correspondance des chemins ; plan de bascule pour Florian | O1, O4 ; `ent-reprise` (T0) | rapport de reprise : chaque site Next classé (UAI ou site), aucune classe sans UAI, rejets listés | M |
| **O8** Entités juridiques | `ent_entite_juridique` (SIRET, nature, employeur, comptable), UAI → entité, analytique par UAI dans les socles 24, 48, 49 | O1 ; socles 24, 48, 49 | une facture ne mêle jamais deux entités ; paie par employeur | M |
| **O9** Réseau, scission, fusion | `plt_reseau` (consolidation sans donnée personnelle) ; **scission** (un UAI quitte le groupe : export de portabilité D-046 vers un nouveau locataire, personnes communes recréées par invitation, aucune copie silencieuse) ; **fusion** de deux locataires (procédure opérateur, rapprochement des personnes comme la fusion de doublons du socle 2, annulable) | O1 ; D-046 ; 27 | avant la commercialisation | L |

Ordre conseillé : O0 → O1 → O2 → O3 avant la démo (la démo se fait déjà sur une adresse UAI fictive) ; O4 avec l'épaississement du socle 5 ; O5 avec le socle 11 ; O7 au printemps 2027 avec la reprise ; O8 avec la comptabilité et la paie ; O9 avant l'ouverture au public.

**Aucune migration de renommage** de `etablissement_id` : on documente son sens (locataire) dans `docs/REGISTRE_TABLES.md` et `docs/RLS_ISOLATION.md` au lot O0.

---

## 11. Cas limites

| # | Cas | Comportement attendu |
|---|---|---|
| CL-O1 | Fratrie dans 3 UAI du groupe, lien d'e-mail venu de l'école, puis du lycée | Les deux liens mènent à l'adresse canonique ; une seule session ; les trois enfants visibles |
| CL-O2 | Enfant dans le groupe A, autre enfant dans le groupe B (autre locataire) | Deux sessions (deux adresses canoniques) ; une passkey ; A ignore B (D-013) ; sélecteur dans l'appli et dans le portail |
| CL-O3 | Élève de 3ᵉ admis en 2ⁿᵈᵉ dans le lycée du groupe | Changement d'UAI par la classe de l'année suivante ; même fiche, même compte ; exports : sortie de l'UAI collège, entrée dans l'UAI lycée au sens officiel [NV] |
| CL-O4 | Élève admis dans un lycée **hors** du groupe mais client ScolIA | Sortie du locataire A ; inscription dans le locataire B par le parcours normal ; aucun transfert automatique entre locataires (D-013) ; portabilité à la demande de la famille (D-046) |
| CL-O5 | Professeur en service partagé collège/lycée du groupe | Un rattachement, un EDT ; heures ventilées par UAI pour STS |
| CL-O6 | Professeur en poste dans deux groupes différents | Deux rattachements, deux sessions, une passkey |
| CL-O7 | Groupe d'enseignement mêlant élèves de deux UAI | Autorisé ; exports par l'UAI de la classe de chaque élève |
| CL-O8 | UAI saisi qui appartient déjà à un autre locataire | Refus sans révéler lequel ; procédure opérateur (erreur de saisie, scission, reprise d'un client) |
| CL-O9 | Faute de frappe sur la lettre clé de l'UAI | Refus immédiat à la saisie (contrôle de clé) ; à l'adresse, 404 générique |
| CL-O10 | L'UAI de référence ferme (fusion de l'école et du collège en un nouvel UAI) | Nouvelle référence obligatoire avant fermeture ; ancienne adresse alias à vie |
| CL-O11 | Le ministère attribue un nouvel UAI (changement de nature) | Nouvel `ent_uai`, l'ancien fermé à la date ; classes de l'année suivante sur le nouveau ; adresse ancienne alias |
| CL-O12 | Changement d'adresse canonique avec SSO Google configuré | Écran : liste des fournisseurs dont l'adresse de retour doit changer ; SSO indisponible tant que ce n'est pas fait, connexion par passkey toujours possible |
| CL-O13 | Session ouverte sur l'ancienne canonique au moment du changement | Continue jusqu'à expiration (≤ 2 h) ; la requête suivante sur l'ancienne adresse est redirigée et redemande la connexion (passkey en un geste, D-011) |
| CL-O14 | Lien d'e-mail ancien `lpnb.scolia.fr/…` après la bascule | 308 vers la canonique ; chemin traduit si connu, sinon accueil avec message |
| CL-O15 | Un UAI quitte le groupe (vente, changement d'OGEC) | Scission O9 ; en attendant : procédure manuelle, aucune donnée déplacée sans accord écrit des deux parties et du DPO |
| CL-O16 | Deux groupes fusionnent | Fusion O9 ; personnes rapprochées par l'opérateur, jamais automatiquement si douteux |
| CL-O17 | Cité scolaire publique, deux EPLE distincts | Un locataire seulement avec un accord de responsabilité conjointe validé [Q-O4] ; sinon deux locataires + réseau |
| CL-O18 | Locataire mono-UAI | Aucune différence visible : adresse = son UAI, pas de mot « groupe » |
| CL-O19 | Personne qui tape `www.0999001a.scolia.fr` ou ajoute un port | Normalisation : `www.` non servi (404 générique) [R : pas de magie] ; port retiré |
| CL-O20 | Requête avec `Host` d'un alias et `X-Forwarded-Host` d'un autre locataire, sans proxy de confiance | `X-Forwarded-Host` ignoré (existant) ; redirection selon `Host` |
| CL-O21 | Cérémonie passkey commencée sur `app.scolia.fr`, terminée sur une canonique | Refusée (défi lié à l'hôte) |
| CL-O22 | Appli mobile d'une famille dont l'établissement change d'adresse canonique | Aucun effet (API sur `app.scolia.fr`, locataire dans la session) |
| CL-O23 | Support ScolIA qui doit aider un groupe | Autorisation de support du 28 (bornée, consentie) sur l'adresse canonique ; jamais via le portail |
| CL-O24 | Recherche publique « Saint-Joseph » | Liste des locataires qui ont accepté d'apparaître, avec ville et UAI ; aucun locataire masqué n'est révélé |
| CL-O25 | Démo et labo | Adresses UAI fictives réservées (préfixe à fixer, ex. codes `0999…`) qui ne peuvent pas être déclarées en production [Q-O3] |
| CL-O26 | Hors contrat sans UAI encore attribué (création récente) [H] | UAI facultatif ? Sinon adresse canonique provisoire : **seul cas** où un libellé non-UAI est attribué (« nouvel établissement »), remplacé par l'UAI dès qu'il existe [Q-O6] |

---

## 12. Sécurité et RGPD (synthèse)

- La frontière forte reste **le locataire** (RLS forcée, une variable, déjà en place et testée) ; à l'intérieur, la séparation par UAI est une portée applicative, testée par la matrice de séparation (`ent-api/tests/separation.rs`) étendue aux portées UAI.
- Sous-domaines du même site : contrôle d'origine strict, aucun CORS entre sous-domaines, aucun contenu tiers ni fichier déposé servi sous `scolia.fr` (D-013 ; les fichiers passent par des URL pré-signées sur l'hôte de stockage objet).
- Aucune réponse d'alias ne pose de cookie ; aucune donnée d'un locataire n'est servie sur le portail.
- Responsabilité de traitement : un locataire multi-UAI suppose une base légale de partage (§5) ; le registre du locataire (socle 27) liste UAI et entités juridiques. **À faire valider par le DPO** avant la première mise en service multi-UAI.

---

## 13. Questions pour Florian

| # | Question | Options | Recommandation |
|---|---|---|---|
| Q-O1 | Valider l'option O-4 + portail neutre (cloisonnement par groupe, adresse par UAI renvoyant à l'UAI de référence) | O-4 / O-3 / O-2 | **O-4 + portail** |
| Q-O2 | `lpnb.scolia.fr` après la cohabitation : alias historique ou canonique ? | alias / canonique | **alias** (cohérent avec « pas de diminutif ») |
| Q-O3 | Codes UAI fictifs pour la démo et le labo | plage fictive réservée / vrais codes du groupe | **plage fictive** (jamais de vrai UAI hors production) |
| Q-O4 | Groupes publics (cité scolaire à deux EPLE) : un locataire possible ? | oui avec accord DPO / non | **oui sous accord écrit et avis DPO**, sinon deux locataires |
| Q-O5 | Exploitation (certificat et DNS joker, HSTS) : qui, quand ? | Florian / plus tard | **Florian, avant la démo** pour l'adresse canonique de démo |
| Q-O6 | Établissement sans UAI (création récente, hors contrat) | adresse provisoire / attendre l'UAI | **adresse provisoire** remplacée dès l'UAI |
| Q-O7 | Recherche publique d'établissement sur le portail | par défaut visible / par défaut masqué | **visible** (nom, ville, UAI : informations publiques), masquable |

---

## 14. Vérifications faites et restant à faire

**Faites** : lecture de D-012, D-013, D-034, D-035, D-055, D-056 ; `05_STRUCTURE.md` (§5, §9.1, §13, §22) ; `04_ADMINISTRATIF.md` §8.2 ; `28_MISE_EN_SERVICE.md` (§2.1, §6.1, MSE-101 à 107, Q-MSE3) ; `02_ELEVE.md` Q-E13 ; code Rust cité au §1.1 ; lecture ciblée du Next.js citée au §1.2 (aucune modification). Rien n'a été compilé ni exécuté.

**[NV] à vérifier avant les lots concernés** : algorithme de la lettre clé UAI (O1) ; présence et forme de l'UAI dans les fichiers SIECLE/BEE et STS (O4, O7) ; modalités de sortie/entrée d'UAI dans SIECLE pour un passage collège → lycée du même groupe (CL-O3) ; prise en charge des jokers dans les liens universels iOS et App Links Android (O5) ; incompatibilité Public Suffix List / `rp_id` WebAuthn (O3) ; attributs UAI dans ÉduConnect et le GAR (O6) ; statut juridique des cités scolaires (Q-O4).
