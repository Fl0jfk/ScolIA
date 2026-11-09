# Interopérabilité nationale — ce qu'un ENT doit faire pour le public et le privé sous contrat

**Date :** 2026-11-05 — **Statut :** étude (recherche et conception). Aucun code écrit. Aucune décision prise : les décisions à soumettre sont listées au §15.
**Contexte :** D-019 (socle 2 vise public **et** privé dès maintenant), lot S2-15 et actions IOP-01→07 de `docs/SOCLE2_DOMAINES.md`, D-009 (passkey obligatoire), D-012/D-013 (sous-domaine par établissement, identité unique), D-017 (rôles paramétrables), D-020 (personne ≠ compte).

> **Vérification du 2026-11-09** (`docs/socles/VERIF_09_SOURCES.md`, qui prime sur ce document) : l'arrêté du 7 septembre 2026 a été lu (art. 1–4, annexe 4). Corrections principales :
> - « exigence n° 6 » (ÉduConnect et Agents) et « exigence n° 15 » (GAR) sont **fausses**. Ce sont les critères **4** et **5**, et **7** pour le GAR, de l'annexe 4. Le n° 15 porte sur l'annuaire fédérateur.
> - Le privé sous contrat n'est pas visé.
> - Dans le public, ScolIA est un « service outillant les missions de vie scolaire » : ÉduConnect sous 10 mois, Agents et GAR sous 22 mois, AAF sous 34 mois.
> - Le raccordement au GAR est demandé par un **porteur public** via la DANé, pas par l'éditeur. OIDC est admis par le GAR depuis 2022.
> - La France a le code pays `99100`.
> - Le BCN n'a pas de licence publiée.

## 0. Méthode, conventions et limites (à lire d'abord)

**Limite majeure.** La politique réseau de cette session **refuse** les hôtes officiels (réponse 403 du proxy, vérifiée le 2026-11-05) : `eduscol.education.fr`, `eduscol.education.gouv.fr`, `www.education.gouv.fr`, `gar.education.fr`, `www.legifrance.gouv.fr`, `doctrine-technique-numerique.forge.apps.education.fr`, `www.data.gouv.fr`, ainsi que les sites académiques et la presse spécialisée. **Aucun PDF officiel n'a pu être lu en entier.** Les faits ci-dessous viennent des **extraits** que le moteur de recherche renvoie de ces documents (titre, URL, phrases citées). C'est une source réelle mais partielle : le contexte autour d'une phrase, les tableaux et les annexes n'ont pas été vus.
→ Pour lever la limite : autoriser ces domaines dans les paramètres réseau de l'environnement (menu de l'environnement → *Edit* → *Network access* → *Allowed domains*), puis relancer une passe de vérification ciblée sur les points marqués **[NV]**.

**Conventions.**
- **[S]** fait sourcé : extrait d'un document officiel ou académique (URL citée, consulté le 2026-11-05 via moteur de recherche).
- **[S-presse]** fait rapporté par la presse spécialisée (Café pédagogique, Localtis/Banque des territoires) : à confirmer sur le texte.
- **[S-tiers]** documentation d'un éditeur ou d'un tiers (Index Éducation, Skolengo, SACoche…) : indicatif.
- **[F]** fait vérifié dans le dépôt (chemin cité).
- **[H]** hypothèse (mémoire ou déduction) ; **[NV]** non vérifié, à vérifier en priorité ; **[R]** recommandation.
- Obligation : **Oui** / **Non** / **Selon** (précisé).

## 1. Constats majeurs

1. **Le cadre est devenu réglementaire pour le public.** Le décret n° 2025-1165 du 5 décembre 2025 a créé les articles **R. 421-78-3 et R. 421-78-4** du code de l'éducation (section 3 ter « Cadre de référence du numérique pour l'éducation ») : les systèmes, services et outils numériques des **collèges et lycées publics** doivent être conformes aux référentiels de **sécurité, d'interopérabilité et de numérique responsable** de la doctrine technique. L'**arrêté du 7 septembre 2026** (JO du 23 septembre 2026) définit ce cadre ; il comporte 4 annexes, dont un tableau de critères de conformité **avec des délais selon le type de service**. [S] [S-presse]
2. **Usage obligatoire des services socles (public).** L'arrêté exige que les services numériques pour l'éducation utilisent les services socles **pour l'authentification** (exigence n° 6 : ÉduConnect pour élèves et responsables, guichet Agents pour les personnels) et **pour l'accès aux ressources** (exigence n° 15 : GAR). [S] (extrait Légifrance via moteur) — texte intégral **[NV]**.
3. **Conséquence directe pour ScolIA (public) :** une connexion locale (mot de passe + passkey) **ne peut pas être la seule voie** pour les élèves, responsables et personnels d'un établissement public. Il faut une fédération ÉduConnect et Agents. Cela entre en tension avec D-009 (passkey obligatoire) et D-013 (identité unique `scolia.fr`) : décision à prendre (§15, décision 1).
4. **Protocoles.** Le référentiel d'interopérabilité (3ᵉ version, annexe 2 de l'arrêté de 2026) donne **OIDC comme standard cible** et marque SAML/OAuth2 comme « fin de vie » dans au moins une ligne de son tableau ; les **contrats d'interface ÉduConnect sont fournis sur demande, pendant les travaux de raccordement** (« accrochage »). [S] Le protocole effectivement ouvert aujourd'hui à un nouvel ENT par ÉduConnect et les guichets Agents (SAML 2.0 et/ou OIDC) **n'a pas pu être confirmé** [NV].
5. **GAR : l'ENT est fournisseur d'identité ET alimente le GAR.** Le projet ENT génère l'assertion **SAML 2.0** (attributs dont `idEnt`, `GARPersonIdentifiant`, `UAI`) ; le contrat d'interface SSO 8.2 (avril 2025) traite aussi des métadonnées **OIDC**. L'ENT dépose une **archive de fichiers XML** conformes à `GAR-ENT-2D.xsd` (2ᵈ degré) ou `GAR-ENT-1D.xsd` (1ᵉʳ degré), contrôlés par le GAR (nommage, MD5, XSD) ; au moins cinq types de fichiers, dont « Eleve ». [S] Documentation partiellement en **diffusion restreinte**.
6. **Le GAR est ouvert au privé sous contrat** (arrêté du 18 décembre 2017 : écoles, EPLE, établissements privés sous contrat, agricoles privés sous contrat). [S] **ÉduConnect dans le privé sous contrat : sur la base du volontariat.** [S] L'arrêté de 2026 vise, d'après la presse, les **collèges et lycées publics** ; son application au privé sous contrat n'apparaît pas dans les extraits [NV].
7. **Alimentation de l'annuaire (public).** Dans le public, l'annuaire de l'ENT est alimenté (quotidiennement) par l'**AAF** (annuaire académique fédérateur), lui-même généré depuis **SIECLE** et **STS-web**, par fichiers XML (élèves, personnels, établissements, responsables), sous **convention** entre l'éditeur et la DSI académique. [S] Ce n'est **pas** un import SIECLE direct par l'établissement, contrairement à l'hypothèse IOP-01.
8. **Privé sous contrat : sens inverse.** Les établissements privés utilisent un **logiciel privé agréé** qui **envoie** les dossiers élèves dans **SIECLE BEE** (XML zippé, menu « Suivi des échanges / Logiciel privé – Dossiers »), désormais via une synchronisation (NetSynchro / SiecleSynchro, en cours de fusion dans **OMOGEN**). L'éditeur doit être **agréé par le ministère**. [S] Pour un client privé, ScolIA doit donc être **émetteur vers BEE**, pas seulement importateur.
9. **Livrets et examens : imports XML réservés aux éditeurs validés.** LSU (collège) et LSL (lycée) acceptent des fichiers **XML conformes à un XSD** produits par des **éditeurs validés** ; pour Parcoursup, la remontée des notes depuis un logiciel certifié passe par un **WebService** depuis la session 2025, avec le même fichier que le LSL. Liste des éditeurs validés et inscription : `communication-editeurs@education.gouv.fr`. [S] Nos exports actuels sont des CSV maison [F] (`domain-eleve/src/lsun_export.rs`, `lsl_export.rs`) : **non utilisables tels quels**.
10. **Op@le ne concerne pas un ENT** au sens strict : c'est le système financier et comptable des **EPLE publics** (96 % des EPLE au 1ᵉʳ janvier 2026 selon une source syndicale). Les interfaces existantes visent les logiciels de **restauration** (calculateur CRDC ouvert à Ianord et Turboself depuis le 1ᵉʳ janvier 2026, porte-monnaie restauration en développement) et la facturation GRETA. [S] [S-tiers] Hors périmètre ENT ; le privé (OGEC) n'est pas concerné.

## 2. Cadre réglementaire et documentaire

| Texte / document | Nature | Portée | Ce qu'on en sait | Source |
|---|---|---|---|---|
| Décret n° 2025-1165 du 5 déc. 2025 | Décret (code de l'éducation, art. R. 421-78-3 et R. 421-78-4) | Collèges et lycées **publics** | Conformité obligatoire aux référentiels sécurité, interopérabilité, numérique responsable fixés par arrêté | https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000052993632 [S] |
| Arrêté du 7 sept. 2026 « cadre de référence du numérique pour l'éducation » | Arrêté (JO 23 sept. 2026) | Collèges et lycées publics (presse) ; privé **[NV]** | 4 annexes : (1) cadre général de sécurité, dont une **POSN** (politique opérationnelle de sécurité numérique) commune avec les équipes sécurité du ministère et des collectivités ; (2) référentiel d'interopérabilité (3ᵉ version) ; (3) numérique responsable ; (4) tableau des critères de conformité **avec délais selon le type de service**. Exigence n° 6 (authentification par les services socles), n° 15 (GAR). Définit l'ENT, distinct d'un service de vie scolaire | https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000054883370 [S] ; https://www.cafepedagogique.net/2026/09/24/numerique-a-lecole-un-nouveau-cadre-de-reference-national/ [S-presse] ; https://www.banquedesterritoires.fr/numerique-pour-leducation-le-ministere-detaille-le-cadre-de-reference [S-presse] |
| Doctrine technique du numérique pour l'éducation | Cadre de référence DNE (v1 mai 2023, juillet 2024, 2025, **2026**) | « Acteurs publics et privés » proposant des services numériques pour l'éducation, 1ᵉʳ et 2ᵈ degrés | Services socles : ÉduConnect, guichets Agents, GAR, DNMA, SCOPE, CRDC ; ÉduGAR (médiacentre sans portail territorial) ; chapitre 4 : identification/authentification | https://www.education.gouv.fr/sites/default/files/document/439539.pdf ; https://doctrine-technique-numerique.forge.apps.education.fr/doctrine-technique/ ; https://www.villes-internet.net/site/doctrine-technique-du-numerique-pour-leducation-la-version-2026-est-publiee/ [S] |
| Référentiel d'interopérabilité des services numériques pour l'éducation | Annexe 2 de l'arrêté (3ᵉ version) | Services publics ou privés | OIDC cible ; appels d'API en OAuth2/OIDC ; contrats ÉduConnect sur demande lors du raccordement | https://www.education.gouv.fr/media/226372/download ; https://eduscol.education.gouv.fr/sites/default/files/document/referentiel-interoperabilite-numerique-educationv072024pdf-106056.pdf [S] |
| SDET (schéma directeur des ENT) | Document de référence (doc. principal **2025**, annexe opérationnelle **2024**) | Projets ENT (collectivités, académies) | Peut être **annexé aux marchés publics** ENT ; deux modèles d'identité (ENT IdP, ou délégation aux guichets Agents et à ÉduConnect) ; réversibilité ; RGAA ; RGPD | https://www.education.gouv.fr/sites/default/files/2025-07/document-principal-du-sdet-version2025-441531.pdf ; https://www.education.gouv.fr/sites/default/files/2025-04/annexe-op-rationnelle-du-sdet-version-2024-439563.pdf [S] |
| Arrêté du 18 déc. 2017 (GAR) | Traitement de données « GAR » | Écoles, EPLE, **privé sous contrat**, agricole privé sous contrat, lycées maritimes, écoles militaires | Durée de conservation des données d'identité : présence de l'enseignant (extrait) | https://www.legifrance.gouv.fr/loda/id/JORFTEXT000036249969 ; https://gar.education.fr/cadre-juridique/ [S] |
| Arrêté du 30 nov. 2006 (ENT) | Création du traitement « ENT » (modifié le 13 oct. 2017 d'après un éditeur ; acte RU-003 CNIL) | ENT | Statut actuel (abrogé ? consolidé ?) **[NV]** ; RU-003 : mentions d'information sur la page d'accueil et à la création de compte | https://www.legifrance.gouv.fr/jorf/id/JORFTEXT000000427578 [S] ; https://edifice.io/app/uploads/2023/09/Conformite-RGPD-2023.pdf [S-tiers] |

**Lecture [R].** Pour un client **public**, il faut viser la conformité à l'arrêté de 2026 (annexe 4) et au SDET, qui sera exigée par l'acheteur (collectivité ou établissement). Pour un client **privé sous contrat**, rien d'obligatoire n'a été trouvé hors du lien avec SIECLE ; mais l'ENT est vendu à des établissements qui, eux, utilisent ÉduConnect et le GAR **s'ils le veulent** : il faut donc construire **les mêmes briques**, activables par établissement.

**Question ouverte majeure [NV] :** l'arrêté de 2026 distingue-t-il « ENT » et « service de vie scolaire » (Pronote, École Directe) avec des exigences différentes ? ScolIA est les deux à la fois. Le classement de ScolIA conditionne les délais de l'annexe 4.

## 3. SDET et cadre de référence : exigences connues

| Domaine | Exigence (ce qui est sourcé) | Oblig. | Source | Impact ScolIA |
|---|---|---|---|---|
| Statut | SDET = document de référence, intégrable en annexe des marchés ENT ; pas de procédure de « label » trouvée [NV] | Selon acheteur | SDET 2024/2025 [S] | Matrice de conformité à produire pour chaque appel d'offres public |
| Identité | Deux modèles : ENT fournisseur d'identité, ou délégation aux guichets Agents (personnels) et ÉduConnect (élèves, responsables) qui « propage les données d'identité à la solution ENT » | Oui (public, via exigence n° 6 de l'arrêté) | SDET 2025 doc. principal [S] | Lot IOP-L5 |
| Jointure | Clé de jointure portée par l'attribut `ENTPersonJointure` (cité pour le guichet de l'enseignement agricole, vecteur `FrEduVecteur`) ; l'ENT transmet aux services tiers un identifiant unique par utilisateur, non lié à l'identité, servant de clé de jointure | Oui | Annexe opérationnelle SDET [S] | Table d'identifiants externes ; pseudonymes par service |
| Alimentation | Fichiers XML d'alimentation de l'annuaire ENT : catégories `EtabEducNat`, `Eleve`, `PersEducNat`, fichier `InfosEducNat` | Oui (public) | SDET annexe 4 alimentation (2022, appel à commentaires) https://eduscol.education.gouv.fr/sites/default/files/document/sdetannuaireent2dannexe4alimentationmen-autressiv65appelacommentaires2022pdf-94605.pdf [S] | Lot IOP-L2 (format AAF) |
| Réversibilité | Au minimum : clause de réversibilité, **plan de réversibilité**, **liste des données éligibles à la reprise** ; préparer la reprise par le nouveau titulaire | Oui (marchés) | Annexe opérationnelle [S] | Export complet par établissement, formats documentés (lot IOP-L11) |
| Accessibilité | RGAA 4.1 ; modules de l'éditeur : **au moins 75 % des critères applicables** ; **page de déclaration d'accessibilité** accessible depuis toutes les pages | Oui | SDET (version non identifiée dans l'extrait) [S] | Design system Leptos ; audit RGAA par module ; déclaration |
| RGPD | Le SDET aide les responsables de traitement à respecter le RGPD ; **minimisation** : ne transmettre que les données nécessaires ; recommandations ANSSI et RGPD sur l'authentification | Oui | SDET [S] | Cohérent avec D-013 ; à formaliser par flux (GAR, ÉduConnect) |
| Sécurité | POSN commune avec les équipes sécurité du ministère et des collectivités | Oui (public) | Arrêté 2026 annexe 1 [S] | Interlocuteur sécurité, procédure d'incident partagée |
| Hébergement | Exigences (France/UE, SecNumCloud, HDS…) **non trouvées** dans les extraits | [NV] | — | Ne rien supposer ; Scaleway (France) reste compatible en première analyse [H] |
| Services | Liste des services socles/optionnels de l'ENT **non obtenue** | [NV] | — | À lire dans le SDET 2025 |

## 4. Identité : ÉduConnect (élèves, responsables) et guichets Agents (personnels)

### 4.1 Faits

- ÉduConnect = « guichet national » et « service socle qui fournit les services d'identification, d'authentification et de gestion des comptes » pour **les élèves et leurs représentants légaux** ; les **guichets d'authentification académiques Agents** couvrent les personnels. [S] (annexe opérationnelle SDET 2024)
- Déployé dans **toutes les écoles, collèges et lycées publics depuis décembre 2020** ; **privé sous contrat : volontariat**. [S] (supports académiques ÉduConnect)
- Activation responsable : être référencé comme responsable légal dans la base élèves de l'établissement (SIECLE/ONDE) avec téléphone ou e-mail à jour ; code SMS valable 30 min ; **FranceConnect** possible ; un compte responsable vaut pour tous les enfants ; compte élève distinct. [S] https://educonnect.education.gouv.fr/educt-aide/guides/EduConnect_Guide2D_Administration_Comptes.pdf
- Le compte ÉduConnect dépend de la qualité des données SIECLE (nom, prénom, date de naissance, **INE**). [S]
- Projets ENT existants : connexion ÉduConnect réalisée par fédération (ex. Toutatice via Shibboleth, « compatible CAS, SAML2 et OpenID Connect ») ; point d'entrée observé `educonnect.education.gouv.fr/idp/profile/SAML2/Redirect/SSO`, donc **SAML 2.0 en production**. [S] (JRES 2019 ; URL publique)
- Contrats d'interface ÉduConnect : **sur demande lors de l'« accrochage »** ; OIDC = standard cible du référentiel d'interopérabilité. [S]

### 4.2 Non vérifié

- Liste exacte des attributs ÉduConnect (format de `FrEduVecteur`, présence de l'INE ou d'un identifiant SIECLE, de l'UAI, des liens responsable → élèves) [NV]. [H] : `FrEduVecteur` serait de la forme `profil|nom|prénom|identifiant|UAI`, avec un vecteur par enfant pour un responsable.
- Protocole proposé à un **nouveau** fournisseur de service : SAML 2.0 seulement, ou OIDC déjà disponible ? [NV]
- Conditions d'éligibilité au raccordement (projet ENT reconnu par l'académie ? convention ? éditeur seul ?) [NV].
- Guichets Agents : protocole, attributs, raccordement **académie par académie** ou guichet national [NV] ; cas des **maîtres du privé sous contrat** (agents de l'État : ont-ils accès au guichet Agents ?) [NV].

### 4.3 Articulation avec notre architecture

| Point | Existant [F] | Conséquence |
|---|---|---|
| `ent-oidc` | OIDC seul, **RS256 seul**, PKCE, JWKS par `kid` (`ent-oidc/src/lib.rs`) ; rattachement local par `ent_oidc_identity (provider_id, subject)` (`ent-db/src/oidc_repo.rs`) ; première connexion rattachée seulement si `email_verified == true` (`ent-db/src/service.rs:3010`) | Réutilisable **si** ÉduConnect/Agents offrent OIDC. Le rattachement par e-mail vérifié **ne marchera pas** pour ÉduConnect (élèves sans e-mail ; e-mail non garanti) : il faut un rattachement **par clé de jointure** issue de l'import (SIECLE/AAF) |
| SAML | Absent | Si SAML obligatoire : nouvelle brique fournisseur de service SAML (signature XML, métadonnées, chiffrement d'assertion). Crate Rust candidate à évaluer (ex. `samael`) : **licence, maintenance, dépendances C (xmlsec/libxml2) à vérifier** avant toute adoption [H] ; alternative : passerelle SAML→OIDC éprouvée en service séparé [H] |
| Passkey obligatoire (D-009) | Passkey facultative ; si enrôlée, le SSO ne la contourne pas | Exiger une passkey **après** ÉduConnect double l'authentification et contredit l'esprit de l'exigence n° 6 ; décision nécessaire (§15, décision 1) |
| Identité unique `scolia.fr` (D-013) | Compte par e-mail (`ent_user`), accès par établissement (`ent_membership`) | Une identité ÉduConnect est **nationale** (un responsable, plusieurs enfants dans plusieurs établissements) : cohérent avec D-013, mais le rattachement doit rester **par établissement** (jointure issue de l'import de chaque établissement) pour garder l'absence de visibilité entre établissements |
| Personne ≠ compte (D-020) | Personne responsable distincte du compte | Bon socle : la **personne** porte les identifiants nationaux (INE, identifiant SIECLE, clé de jointure) ; le **compte** porte les moyens de connexion (passkey, ÉduConnect, Agents, Google/Microsoft) |
| Provisionnement | IOP-06 : « aucun provisionnement implicite » | Confirmé [R] : la connexion ÉduConnect ne crée rien ; elle rattache à une personne déjà importée (sinon refus avec message « contactez l'établissement ») |

## 5. GAR (gestionnaire d'accès aux ressources)

### 5.1 Faits
- Le GAR gère les autorisations d'accès aux ressources numériques présentées dans les **médiacentres** des ENT (ou ÉduGAR sans portail territorial) ; plus de 9 400 collèges et lycées rattachés à 38 projets ENT à la rentrée 2024 ; 99 % des établissements publics (novembre 2025). [S]
- **SSO** : le projet ENT génère l'assertion **SAML 2.0** contenant les attributs de l'utilisateur, dont `idEnt` (identifiant du projet ENT), `GARPersonIdentifiant` (identifiant interne de l'utilisateur), `UAI` (établissement). Le contrat SSO 8.2 traite la mise en cache des métadonnées ENT **OIDC et SAML** et liste les éléments techniques à fournir par les projets ENT utilisant **OIDC**. [S] https://gar.education.fr/wp-content/uploads/2025/04/GAR_RTFS_8.2_Contrat_SSO_ENT.pdf ; https://gar.education.fr/wp-content/uploads/2025/04/GAR-RTFS-V8.2-ReferentielTechnique_ENT.pdf
- **Alimentation** : archive déposée par l'opérateur ENT, traitée par le service de collecte du GAR ; contrôles de nommage, **MD5** et validité **XSD** ; `GAR-ENT-2D.xsd` (2ᵈ degré), `GAR-ENT-1D.xsd` (1ᵉʳ degré), fournis dans le paquet de documentation RTFS V7.2 ; au moins cinq types de fichiers, dont « Eleve » (`GAREleve`). [S]
- Les **affectations** de ressources sont faites par les établissements dans la **console d'affectation** du GAR. [S] https://gar.education.fr/etablissements-et-ecoles/faq-console-daffectation/
- Une partie des documents est en **diffusion restreinte** ; un référentiel administratif et juridique daté du 12 juin 2026 existe. [S] https://gar.education.fr/wp-content/uploads/2026/06/GAR-ReferentielAdminJuridique_FR_20260612.pdf
- Ouvert au **privé sous contrat** (arrêté 2017). [S]

### 5.2 Non vérifié
Liste complète des fichiers (établissements, enseignants, groupes, responsables d'affectation, matières, MEF…) [H : `Etab`, `Eleve`, `Enseignant`, `Groupe`, `RespAff`, `PersEducNat` — **noms non confirmés**] ; fréquence (quotidienne ? complet/différentiel ?) ; canal de dépôt [NV] ; nomenclatures des profils (ex. `National_ELV`, `National_ENS`, `National_DOC`) [H] ; procédure de raccordement (convention, recette sur environnement d'intégration — un sous-domaine `test-gar.education.fr` existe [S]) [NV] ; rôle d'un ENT de **privé** isolé, hors projet territorial [NV].

### 5.3 Impact
- ScolIA devient **fournisseur d'identité** (SAML 2.0, et OIDC si accepté) pour un service tiers : brique nouvelle, plus sensible que le simple SSO entrant (clés de signature, rotation, métadonnées publiées).
- `GARPersonIdentifiant` : identifiant **opaque et stable** par personne et par projet ENT → table d'identifiants externes, jamais l'UUID interne ni l'e-mail [R].
- `idEnt` : un projet ENT = ? ScolIA multi-établissements, multi-académies : un `idEnt` par client, par académie ou unique — à demander au GAR [NV]. Lien direct avec D-012/D-013.
- Médiacentre : écran listant les ressources affectées (données renvoyées par le GAR) [H sur l'API exacte].

## 6. Annuaire et imports

| Flux | Sens | Public | Privé sous contrat | Format / canal | Source | Obligation |
|---|---|---|---|---|---|---|
| **AAF** (SIECLE + STS-web → AAF → ENT) | Entrant | Voie normale de création des comptes ENT, alimentation chaque soir | [NV] (le privé n'est en général pas dans l'AAF) | XML : `ENT_<RNE>_Complet_<date>_<Type>_0000.xml` (élèves, personnels, établissements, responsables) ; export autorisé par **convention** éditeur–DSI académique | Guides académiques Aix-Marseille, Lille, Dijon (EOLE), Grenoble [S] | Oui (public) pour créer les comptes et faire la jointure ÉduConnect/Agents |
| **SIECLE BEE** → logiciel | Entrant | Exports XML de BEE (élèves, nomenclatures, structures) utilisés par les logiciels d'EDT/vie scolaire | Possible | XML ; voie OMOGEN annoncée pour 2026-27 | Index Éducation (EDT/PRONOTE) [S-tiers] ; académie de Nice (fusion SiecleSynchro/NetSynchro dans OMOGEN) [S] | Selon |
| Logiciel privé → **SIECLE BEE** | **Sortant** | — | **Voie normale** : dossiers élèves (+ liens élèves-groupes) envoyés à BEE ; éditeur **agréé** ; XML zippé ; rejets corrigés dans le logiciel éditeur | NetSynchro / OMOGEN, certificat installé dans la solution éditeur | Guide BEE EPHC (juin 2022) ; guide « bonnes pratiques scolarité et vie scolaire **privé** » 2025 (Toutatice) ; présentation NetSynchro [S] | **Oui pour un client privé** si ScolIA remplace Charlemagne (remontée des effectifs) |
| **STS-web** | Entrant / sortant | Logiciel d'EDT → STS-web (un seul XML structure + services, avant la rentrée) ; STS-web → `sts_emp_<RNE>.xml` | Enseignants du privé sous contrat dans STS-web [H] | XML | Index Éducation EDT [S-tiers] ; académies de Lyon, Bordeaux [S] ; annonce OMOGEN rentrée 2026 (Skolengo) [S-tiers] | Selon (si ScolIA fait l'EDT) |
| **ONDE** (1ᵉʳ degré) | Sortant (privé) | Annuaire via AAF | « Interface école » : remontée des dossiers élèves des écoles **privées** depuis un logiciel éditeur validé | Fichier d'export (format [NV]) | Eduscol interfaces éditeurs [S] ; procédure Charlemagne [S] | Hors socle 2 (Q8) |
| **BCN** | Entrant | Nomenclatures (MEF, `N_MATIERE_ENSEIGNEE`…) | Idem | Tables BCN ; webservices d'extraction depuis 2015 ; documentation `bcn.depp.education.fr` ; licence (Etalab 2.0 ?) [NV] | DEPP / BO 2017 [S] | Oui de fait (codes exigés par STS-web, LSL, GAR) |

**Point d'attention [S] :** une matière, une modalité ou une classe sans code reconnu par STS-web bloque l'export ; les heures de vie de classe sont à retirer avant export. → Notre référentiel doit porter les codes nationaux, pas des libellés libres.

## 7. Exports réglementaires

| Export | Données | Format | Condition | Source | État ScolIA |
|---|---|---|---|---|---|
| **LSU** collège | Bilans périodiques et de fin de cycle | XML conforme XSD ; 1ᵉʳ degré : `import-lsun-{timestamp}.xml`, schéma `import-bilan-1d.xsd` ; 2ᵈ degré : imports éditeurs acceptés depuis LSU 16.4 ; synthèse d'import (succès / alertes / rejets) ; un import éditeur est « supposé complet » | **Éditeur validé** | Spécification éditeurs LSUN 1ᵉʳ degré v1.22 (Toutatice) ; fiches LSUN Créteil [S] ; SACoche [S-tiers] | CSV maison [F] `lsun_export.rs` |
| **LSL** lycée | Notes, appréciations par discipline (CAP, bac général et technologique) | XML ; éditeur **accrédité** ; identifiant SIECLE de l'élève requis ; matières selon la **nomenclature LSL** | Éditeur validé (liste 2026 : APLIM/Charlemagne, CGI/OpenENT, Docapost/Index) | Eduscol LSL FAQ ; académie d'Aix-Marseille ; Index Éducation [S] | CSV maison [F] `lsl_export.rs` |
| **Parcoursup** | Notes (bulletins), initialise la fiche Avenir | **WebService** depuis la session 2025 ; même fichier que le LSL ; choix du logiciel déclaré par l'administrateur Parcoursup | Logiciel certifié | Académie de Lille (sessions 2023, 2026) ; DGER 2024 [S] | CSV [F] `parcoursup_export.rs` |
| Cyclades (examens) | Notes de livret depuis LSL | via LSL | — | Académie d'Aix-Marseille [S] | — |
| Notanet / DNB | — | [NV] | — | — | `notanet_export.rs` [F] |
| Affelnet | — | [NV] | — | — | — |

**Conséquence [R] :** sans **validation éditeur**, ScolIA ne peut pas alimenter LSU/LSL/Parcoursup ; l'établissement ressaisirait. C'est un prérequis commercial **public et privé** (les livrets concernent les deux). Démarche : `communication-editeurs@education.gouv.fr` [S].

## 8. Op@le et autres interfaces

- **Op@le** : système d'information financière des EPLE (programme MF²) ; BO hebdo n° 7 de 2026 sur les EPLE connectés ; vagues restantes janvier 2026 et janvier 2027 ; interfaces éditeurs : calculateur **CRDC** (droits constatés restauration) ouvert à Ianord et Turboself depuis le 1ᵉʳ janvier 2026, porte-monnaie restauration en développement, facturation GRETA ; convention Payfip pour les paiements. [S] https://www.education.gouv.fr/bo/2026/Hebdo7/MENF2600269J ; https://www.education.gouv.fr/programme-mf2-une-nouvelle-ere-pour-la-fonction-financiere-dans-les-etablissements-5333 [S] ; forum gestionnaires [S-tiers].
- **Pertinence** : **nulle pour l'ENT**, **réelle pour notre module restauration/facturation dans le public** : dans un EPLE, la facturation relève de l'agent comptable dans Op@le [H confirmé en substance]. ScolIA ne doit pas facturer dans un EPLE public ; il pourrait plus tard devenir un « logiciel de restauration » interfacé CRDC [R, hors socle 2].
- **SIRH** : aucune interface ENT trouvée ; les personnels arrivent par STS-web/AAF. [NV]
- **SCOPE, DNMA** : cités comme services socles par la doctrine [S] ; contenu et pertinence **non étudiés** [NV].

## 9. Matrice public / privé sous contrat

| Brique | Public (collège, lycée) | Privé sous contrat |
|---|---|---|
| ÉduConnect | **Obligatoire** (exigence n° 6, arrêté 2026) [S] | Volontariat [S] ; à proposer, activable |
| Guichet Agents | **Obligatoire** [S] | [NV] (maîtres du privé) |
| GAR | **Obligatoire** (exigence n° 15) [S] | Possible (arrêté 2017) [S] |
| Alimentation de l'annuaire | AAF (convention DSI académique) [S] | Saisie/inscriptions ScolIA + imports BEE [H] |
| SIECLE BEE | Lecture (via AAF) | **Écriture** : remontée des dossiers par logiciel agréé [S] |
| LSU / LSL / Parcoursup | Éditeur validé | Éditeur validé (Charlemagne est validé) [S] |
| Facturation | Op@le (agent comptable) — hors ScolIA | Cœur ScolIA (OGEC) [F] |
| Arrêté 2026 / SDET | S'impose [S] | Non établi [NV] ; argument commercial [R] |

## 10. Impacts sur notre modèle

- **Référentiel** : ajouter l'**UAI** de l'établissement (absent du schéma [F] : aucune colonne `uai` dans `ent-rust/migrations/`) ; codes **MEF** et **matières BCN** (REF-07, REF-14), **divisions/groupes** avec codes SIECLE/STS ; **nomenclature LSL** pour les matières de lycée ; tables de correspondance libellé local ↔ code national.
- **Identifiants externes** : table générique `personne_identifiant_externe (personne_id, etablissement_id, systeme, valeur, valide_depuis, valide_jusqua)` avec `systeme ∈ {INE, SIECLE_ELEVE_ID, SIECLE_RESP_ID, STS_ID, AAF_JOINTURE, EDUCONNECT, AGENTS, GAR_PERSON_ID}` [R] ; unicité par (système, valeur, établissement) ; l'INE reste un attribut de la personne élève (`ine.rs` [F] tolère 9–16 caractères : à resserrer sur le format national réel une fois vérifié [NV]).
- **Personnes (D-020)** : l'import AAF/BEE crée des **personnes** (élèves, responsables, personnels) ; un **compte** n'existe qu'à la première connexion (ÉduConnect, Agents ou invitation). Les responsables sans compte restent des personnes.
- **Rôles (D-017)** : les profils nationaux (élève, responsable, enseignant, documentaliste, direction…) servent à **pré-affecter un modèle de rôle** à l'import ; l'établissement garde la main. Le profil GAR transmis se **déduit** des rôles ScolIA (table de correspondance), jamais l'inverse [R].
- **Isolation (D-012/D-013)** : chaque import est **par établissement** (UAI ↔ tenant) ; une jointure ÉduConnect ne donne accès qu'aux établissements où la personne a été importée ; les identifiants GAR sont propres au projet ENT et ne servent qu'au GAR ; aucun identifiant national n'est exposé à un autre établissement. Tests de séparation (`ent-api/tests/separation.rs` [F]) à étendre aux imports et au SSO fédéré.
- **Journal** : tout lot importé/exporté tracé (`ent_import_lot`, `ent_export_trace`, prévus en D23 [F, conception]).

## 11. Lots proposés (remplacent et détaillent S2-15)

Effort : **S** ≤ 3 j, **M** 1–2 sem., **L** > 2 sem. [H]. « Bloqué » = dépend d'un document ou d'un agrément externe.

| Lot | Périmètre | Dépend de | Critère de fin | Effort | Bloqué par |
|---|---|---|---|---|---|
| **IOP-L0 Référentiel national** | UAI par établissement ; nomenclatures BCN (MEF, matières) importées et versionnées ; codes nationaux sur niveaux, matières, divisions, groupes ; table d'identifiants externes | S2-1, S2-2 | un établissement de test porte UAI, MEF et matières BCN ; contrainte d'unicité testée ; isolation testée | M | Accès BCN (format, licence) |
| **IOP-L1 Moteur d'import** | lots, lignes, prévisualisation, différences, rejets ligne à ligne, annulation (A18) ; jamais d'écrasement silencieux | IOP-L0 | import factice rejouable, idempotent, annulable ; tests de rejets | M | — |
| **IOP-L2 Import AAF (public)** | parseur XML AAF (élèves, responsables, personnels, établissements) ; création de **personnes** ; clés de jointure | IOP-L1 | fichiers d'exemple officiels importés sans perte ; rapprochement INE puis identité | M | **Spécification AAF + convention DSI** |
| **IOP-L3 Imports BEE / STS-web** | XML BEE (élèves, structures) ; `sts_emp` (services, EDT) ; suivi OMOGEN | IOP-L1 | idem, fichiers d'exemple réels anonymisés | M | Spécifications ; statut OMOGEN |
| **IOP-L4 Remontée vers BEE (privé)** | export des dossiers et liens élèves-groupes vers SIECLE BEE (NetSynchro/OMOGEN) | IOP-L3 | fichier accepté par BEE en environnement de qualification | L | **Agrément éditeur privé** |
| **IOP-L5 SSO entrant ÉduConnect + Agents** | fournisseur de service (OIDC via `ent-oidc` si disponible ; sinon brique SAML) ; rattachement **par jointure** à une personne importée ; pas de provisionnement ; choix passkey (décision 1) | IOP-L2 (public) ou IOP-L0 + saisie (privé) ; décision 1 | connexion de bout en bout sur l'environnement de test ÉduConnect ; refus d'un inconnu ; séparation entre établissements | L | **Contrat d'interface ÉduConnect (sur demande)** ; protocole |
| **IOP-L6 ScolIA fournisseur d'identité** | IdP SAML 2.0 (et OIDC si accepté) pour le GAR : métadonnées, signature, rotation des clés, attributs `idEnt`, `GARPersonIdentifiant`, `UAI`, profils | IOP-L0 | assertion validée par l'environnement d'intégration GAR | L | Contrat SSO GAR 8.2 complet |
| **IOP-L7 Alimentation GAR** | génération XML `GAR-ENT-2D.xsd`, archive, MD5, dépôt, lecture des rapports d'erreurs | IOP-L2/L3, IOP-L6 | archive acceptée sans rejet par le GAR (intégration) | M–L | XSD et canal de dépôt (diffusion restreinte) |
| **IOP-L8 Médiacentre** | écran Leptos des ressources affectées, ouverture via le GAR | IOP-L6, IOP-L7 | un élève ouvre une ressource de test | S–M | API GAR [NV] |
| **IOP-L9 LSU XML** | bilans périodiques et de fin de cycle (collège) au format XSD officiel | S2-7 | fichier importé dans LSU (qualification), synthèse sans rejet | M | **Validation éditeur** + XSD 2ᵈ degré |
| **IOP-L10 LSL + Parcoursup** | XML LSL, WebService Parcoursup, nomenclature LSL | S2-7 | remontée de test acceptée | L | **Accréditation éditeur** |
| **IOP-L11 Conformité** | matrice arrêté 2026 (annexe 4) + SDET ; POSN ; déclaration d'accessibilité RGAA ; plan et export de réversibilité ; mentions RGPD sur la page d'accueil | transverse | matrice remplie et revue, preuves liées | M (puis continu) | Texte intégral de l'arrêté |
| **IOP-L12 Veille Op@le / ONDE** | aucune réalisation ; note de veille | — | — | S | — |

**Ordre recommandé [R]** : L0 → L1 → L11 (en parallèle, dès maintenant) → demandes externes (§12) → L2/L3 → L5 → L6/L7/L8 → L9/L10 ; L4 seulement si un client privé abandonne Charlemagne. Les lots « bloqués » ne doivent **pas** démarrer avant réception des spécifications officielles : coder un format deviné reproduirait l’erreur des règles « citées de mémoire » signalée par D-019.

## 12. Questions à poser (DNE, ÉduConnect, GAR, rectorat)

1. **DNE / bureau des éditeurs** (`communication-editeurs@education.gouv.fr`) : procédure d'inscription d'un nouvel éditeur ; accès aux spécifications (AAF, BEE, STS-web, LSU 2ᵈ degré, LSL, Parcoursup, NetSynchro/OMOGEN) ; calendrier OMOGEN ; agrément « logiciel privé » pour BEE.
2. **Arrêté du 7 septembre 2026** : s'applique-t-il au privé sous contrat ? ScolIA (ENT + vie scolaire) est-il un « ENT » ou un « service de vie scolaire » au sens de l'arrêté ? Quels délais de l'annexe 4 ? Existe-t-il une procédure de vérification de conformité ?
3. **ÉduConnect** : un éditeur (et non une collectivité) peut-il se raccorder ? Pour le privé sous contrat ? Protocole disponible pour un nouveau raccordement (OIDC ?) ; attributs (INE ? identifiant SIECLE ? liens responsable-enfants ?) ; environnement de test ; durée.
4. **Guichets Agents** : national ou par académie ? protocole et attributs ; accès des maîtres du privé sous contrat.
5. **GAR** : un éditeur multi-académies est-il **un** projet ENT (`idEnt`) ou plusieurs ? ENT d'un seul établissement privé : possible ? accès aux documents en diffusion restreinte ; canal et fréquence de l'alimentation.
6. **Rectorat / DSI académique** : convention d'export AAF pour un nouvel ENT ; données transmises ; délai.
7. **Doublon d'authentification** : la doctrine admet-elle un second facteur propre à l'ENT (passkey) après ÉduConnect ? Une connexion locale de secours est-elle tolérée ?
8. **Hébergement** : exigences de localisation/qualification (SecNumCloud, HDS pour l'infirmerie) dans l'arrêté de 2026.

## 13. Hypothèses [H] de `SOCLE2_DOMAINES.md` : corrections

| Hypothèse (§ / action) | Constat | Source |
|---|---|---|
| §1.3 « ÉduConnect, GAR, SDET : condition d'entrée sur le marché public » | **Confirmé et renforcé** : ce n'est plus seulement un usage, c'est **réglementaire** (R. 421-78-3, arrêté du 7 sept. 2026, exigences n° 6 et 15) pour les collèges et lycées publics | Légifrance JORFTEXT000052993632, JORFTEXT000054883370 [S] |
| Q1 / D-019 « le public exige ÉduConnect, GAR, SDET, interface Op@le » | **Imprécis** : Op@le n'est **pas** exigé d'un ENT ; il concerne la comptabilité de l'EPLE et quelques logiciels de restauration | §8 [S] |
| IOP-01 « Importer SIECLE (élèves, responsables, structures) » | **Imprécis** : dans le public, l'ENT reçoit les données via l'**AAF** sous convention académique ; dans le privé, le logiciel **envoie** vers BEE. Deux flux distincts, dans des sens opposés | §6 [S] |
| IOP-02 « Importer STS-Web » | **Partiel** : STS-web reçoit aussi les services **depuis** le logiciel d'EDT ; les personnels arrivent dans l'ENT via l'AAF ; bascule OMOGEN annoncée en 2026-27 | §6 [S] [S-tiers] |
| IOP-06 « ÉduConnect / FranceConnect, fédération comme `ent-oidc` » | **Imprécis** : FranceConnect est une méthode d'activation **dans** ÉduConnect, pas un fournisseur à raccorder séparément ; ÉduConnect fonctionne en **SAML 2.0** en production, OIDC = cible ; rattachement par jointure, pas par e-mail | §4 [S] |
| IOP-07 « affectations transmises au GAR » | **Faux sur le sens** : les **affectations** sont saisies dans la console du GAR par l'établissement ; l'ENT transmet **l'annuaire** (élèves, enseignants, groupes…) et **l'identité** (assertion SAML) | §5 [S] |
| §1.3 / EVA-12 « LSU : import par fichier XML au format ministériel » | **Confirmé**, avec une condition oubliée : **éditeur validé** | §7 [S] |
| §1.3 « LSL, Parcoursup : remontées de notes » | **Confirmé et précisé** : même fichier ; Parcoursup par **WebService** depuis 2025 ; éditeur accrédité | §7 [S] |
| §1.3 « Comptabilité publique : facturation demi-pension/internat dans Op@le » | **Confirmé en substance** (Op@le, agent comptable) ; interfaces restauration en cours | §8 [S] |
| §1.3 « RGAA pour les services publics numériques » | **Confirmé et précisé** pour les ENT : ≥ 75 % des critères applicables pour les modules éditeur, déclaration d'accessibilité | §3 [S] |
| Q8 / IOP-03 ONDE | **Précisé** : pour le privé, « Interface école » vers ONDE par éditeur validé ; reste hors socle 2 | §6 [S] |
| `ine.rs` « format national réel ~11 » | Non vérifié ici [NV] | — |
| S2-15 « 4–6 semaines » | **Sous-estimé** : 5 lots L, plusieurs bloqués par des démarches externes de plusieurs mois [H] | §11 |

## 14. Risques

| Risque | Gravité | Parade [R] |
|---|---|---|
| Spécifications non publiques (diffusion restreinte, « sur demande ») : impossible d'avancer sur L2, L4–L10 sans démarche | Élevée | Lancer les demandes (§12) **maintenant** ; démarches de plusieurs mois [H] |
| Agréments éditeur (LSU, LSL, BEE) refusés ou lents | Élevée | Commencer par l'inscription éditeur ; viser un établissement pilote volontaire |
| Contradiction D-009 (passkey pour tous) / exigence n° 6 (ÉduConnect) | Élevée | Décision 1 |
| Dépendance SAML (XML signature) en Rust : faible maturité, failles classiques (enveloppement de signature) | Élevée | Brique isolée, revue sécurité, tests négatifs ; ou passerelle éprouvée ; vérifier licences (pas d'AGPL) |
| ScolIA IdP pour le GAR : gestion de clés de signature | Moyenne | Clés hors base, rotation documentée, journal |
| Modèle : identifiants nationaux mal isolés entre établissements (violation D-013) | Élevée | Identifiants externes **par établissement**, matrice de séparation étendue |
| Hypothèse « ENT multi-tenant d'un éditeur » non prévue par le GAR/ÉduConnect (pensés par projet territorial) | Moyenne | Question 5 |
| Bascule OMOGEN en cours : formats instables | Moyenne | Ne pas coder `sts_emp` avant confirmation |

## 15. Décisions à soumettre à Florian (ordre de priorité)

1. **Passkey et fédération nationale.** Dans un établissement public, accepter ÉduConnect (élèves, responsables) et le guichet Agents (personnels) **comme authentification suffisante**, sans passkey supplémentaire, et garder la passkey obligatoire pour les comptes locaux (privé, connexion directe) ? Alternative : passkey exigée en plus (friction, conformité douteuse). **Recommandation : oui, exception à D-009 pour les connexions fédérées nationales.**
2. **Lancer les démarches externes** (inscription éditeur DNE, demande des contrats ÉduConnect et GAR, question sur l'arrêté de 2026 et le privé). Qui les porte (Florian / structure juridique) ? **Recommandation : immédiatement ; c'est le chemin critique, pas le code.**
3. **Autoriser les domaines officiels** dans l'environnement de travail (`*.education.gouv.fr`, `eduscol.education.fr`, `gar.education.fr`, `legifrance.gouv.fr`, `forge.apps.education.fr`, `data.gouv.fr`) pour une passe de vérification des points [NV], en priorité le texte intégral de l'arrêté du 7 septembre 2026 et son annexe 4.
4. **Rattachement des identités fédérées par clé de jointure issue des imports** (et non par e-mail), sans provisionnement implicite. **Recommandation : oui.**
5. **Lots IOP-L0 (référentiel national) et IOP-L1 (moteur d'import) intégrés dès S2-1/S2-2**, puisqu'ils conditionnent le modèle. **Recommandation : oui.**
6. **Positionnement privé** : ScolIA remplace-t-il le logiciel de gestion (Charlemagne) et doit-il donc **remonter vers SIECLE BEE** (IOP-L4, agrément) ou cohabite-t-il au début ? **Recommandation : cohabitation au pilote, L4 ensuite.**
7. **Brique SAML** : accepter une dépendance Rust SAML (après analyse de licence et de sécurité) ou une passerelle séparée, si OIDC n'est pas disponible côté ÉduConnect/Agents. À trancher après la réponse à la question 3 du §12.
8. **Modèle multi-tenant face au GAR** (`idEnt` unique ou par client/académie) : à trancher après la réponse du GAR.
9. **Re-chiffrage de S2-15** : passer de 4–6 semaines à un ensemble de lots (§11), dont plusieurs conditionnés par des délais externes.

## 16. Sources consultées (2026-11-05, via extraits de moteur de recherche ; documents non lus intégralement)

Officielles : Légifrance JORFTEXT000052993632 (décret 2025-1165), JORFTEXT000054883370 (arrêté 7 sept. 2026), JORFTEXT000036249969 (GAR 2017), JORFTEXT000000427578 (ENT 2006) ; education.gouv.fr : SDET doc. principal 2025 (`2025-07/document-principal-du-sdet-version2025-441531.pdf`), annexe opérationnelle 2024 (`2025-04/annexe-op-rationnelle-du-sdet-version-2024-439563.pdf`), doctrine technique (`document/439539.pdf`), référentiel d'interopérabilité (`media/226372/download`, `2025-09/…-439551.pdf`), glossaire (`2025-09/…-439557.pdf`), journée des porteurs de projets ENT (19 nov. 2025, page 450481 et support 476244), BO 2026 hebdo 7 (Op@le), programme MF² ; eduscol : SDET 2024 (`sdetdocument-principalversion2024pdf-106014.pdf`, `sdetannexe-operationnelleversion2024pdf-106008.pdf`), annexe alimentation 2022 (`…94605.pdf`), page interfaces éditeurs (`/4920/…`), LSL FAQ ; gar.education.fr : RTFS 8.2 référentiel technique ENT, contrat SSO ENT 8.2 et 7.2, référentiel administratif et juridique (12 juin 2026), cadre juridique, FAQ console d'affectation, bilan CNIL 2022 ; ÉduConnect : guide d'administration des comptes 2ᵈ degré ; doctrine sur la forge (`doctrine-technique-numerique.forge.apps.education.fr`) ; DEPP BCN (`bcn.depp.education.fr`).
Académiques : Aix-Marseille, Lille, Dijon (EOLE), Grenoble, Bordeaux, Lyon, Nice, Rennes (Toutatice : guide privé 2025, spécification LSUN 1ᵉʳ degré v1.22), Créteil (fiches LSUN), Occitanie.
Presse / tiers (indicatif) : Café pédagogique (24 sept. 2026, 15 déc. 2025), Banque des territoires/Localtis, Villes Internet, Index Éducation (PRONOTE, EDT), Skolengo (rentrée 2026), SACoche, Edifice, JRES 2019.

**Non vérifié (récapitulatif)** : texte intégral et annexes de l'arrêté de 2026 ; application au privé ; attributs et protocole ÉduConnect pour un nouveau raccordement ; guichets Agents ; liste et fréquence des fichiers GAR ; formats AAF/BEE/STS actuels et OMOGEN ; XSD LSU 2ᵈ degré ; Notanet, Affelnet ; exigences d'hébergement ; services socles SCOPE et DNMA ; format de l'INE.
