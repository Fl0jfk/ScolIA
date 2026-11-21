# Portabilité, exports autonomes et gouvernance des données par établissement

Statut : **proposé, à valider par Florian** (2026-11-10, branche `ent-architecture-pra`). Réponse au §17 de `docs/missions/MISSION_PRA_SAUVEGARDES.md`. Documentation seulement, aucun code.
Conventions : **[F]** fait vérifié dans le code ou un document ; **[H]** hypothèse ; **[R]** recommandation ; **[Q]** question ouverte.
Liens : `docs/PRA_SAUVEGARDES.md` (moteur d'extraction partagé §11 et §16.3, registre E12), `docs/ROLES_ET_DROITS.md` (catalogue d'actions D-017), D-009/D-011/D-013 (passkeys, sessions, identité), D-022 (exports officiels), D-036 (JSON), socle RGPD et socle 28 de `docs/SOCLES.md` (« export de réversibilité en fin de contrat »).

---

## 1. Ce que c'est, ce que ce n'est pas
- **Export de portabilité** : l'établissement récupère **lui-même**, sans intervention de l'éditeur, une copie complète et **compréhensible** de ses données, exploitable sans ScolIA et **réimportable** dans des conditions documentées.
- **Différent des sauvegardes techniques** (`PRA_SAUVEGARDES.md`) : une sauvegarde est globale, chiffrée pour l'opérateur, au format PostgreSQL ; un export est **par établissement**, lisible, remis à l'établissement.
- **Différent des exports officiels** (LSU, LSL, Notanet, Parcoursup, ONDE — D-022) : ceux-ci restent des actions métier ordinaires du catalogue, hors de ce dispositif.
- **Différent du droit d'accès ou de portabilité d'une personne** (RGPD art. 15 et 20) : procédure particulière (§4).

## 2. Audit de l'existant (2026-11-10)

| Brique | État [F] | Utilité pour les exports |
|---|---|---|
| Séparation des établissements | `etablissement_id` sur 317/329 tables ; RLS sur ≈ 63 tables, mode `off` par défaut ; `TenantTx` | l'extracteur lit **sous `TenantTx` en mode strict** : il ne peut pas lire un autre établissement sur les tables couvertes ; contrôle de fermeture pour les autres (PRA §11.3) |
| Rôles et droits | catalogue d'actions avec portée (D-017), rôles créés par l'établissement, cumul, garde-fou « pas d'escalade », administrateur indélébile ; `exports.officiels`, `dossier_admin.exporter`, `livrets.exporter` | ajouter des actions dédiées (§5.1) ; aucune action actuelle ne couvre un export complet |
| Délégations | `ent_delegation` (172) : datée, fin obligatoire, délégant ≠ délégué (`CHECK`), `droits JSONB` (à normaliser, D-036) | suppléance des approbateurs (§5.4) |
| Passkeys | `webauthn-rs`, passkey par personne liée à `scolia.fr` (D-013), obligatoire à terme (D-009) | authentification forte des deux validations |
| Sessions | `assurance` ∈ {`mot_de_passe`, `passkey`, `sso`}, `portee`, `client`, `appareil`, `ip_creation` (095) | savoir si la session a été ouverte par passkey ; **manque** : l'instant de la dernière authentification forte |
| Réauthentification | par **mot de passe** sur routes sensibles, compteur `reauth:<user_id>` | **insuffisant** : il faut une réauthentification **par passkey** avec vérification de l'utilisateur (`userVerification = required`) |
| Personne / compte | personne distincte du compte (D-020) ; identité unique (D-013) | empêcher qu'une **même personne** valide deux fois via deux comptes |
| Journal | `ent_journal_modif` en ajout seul (déclencheur 091) ; `TRUNCATE` non bloqué ; écriture « best-effort » | pour les exports, l'écriture du journal doit être **dans la même transaction** que la transition d'état (pas « best-effort ») |
| Fichiers | clés `{etablissement}/{espace}/{id}.{ext}`, `key_belongs_to_etab`, URL pré-signées courtes ; `BYTEA` pour 2 tables ; disque local source de vérité | export des fichiers originaux ; sortie des `BYTEA` (E5) |
| Chiffrement applicatif | IBAN, santé, psy chiffrés (`ENT_SECRETS_KEY`) | l'export déchiffre seulement ce que la catégorie autorise (§4) |

## 3. Architecture proposée [R]

```
Demande (initiateur, passkey récente) ──► en attente ──► Approbation (2e personne, passkey récente)
        │                                    │                     │
        │ révocation / expiration (48 h)     │                     ▼
        ▼                                    ▼            Tâche d'extraction (worker, établissement X seul)
     révoquée / expirée                   refusée                  │  ent-extraction : instantané REPEATABLE READ,
                                                                   │  TenantTx(X) strict, registre E12, fermeture
                                                                   ▼
                                                    Archive chiffrée (bucket « exports », hors réplication)
                                                                   │  manifeste signé, empreintes
                                                                   ▼
                                       Lien de téléchargement court, nominatif ──► purge à J+7
```
- **Un seul moteur** (`ent-extraction`, PRA §16.3) pour la restauration sélective, les tests de restauration et les exports. Le format **technique** est le même ; l'export y ajoute une **vue métier** lisible.
- L'extraction n'est lancée **que** par le worker, à partir d'une demande à l'état `approuvee` : aucune route ne produit un export complet de façon synchrone. Il n'existe **aucune** route « administrateur technique » qui crée une demande approuvée.
- Concurrence : une seule demande active par établissement et par catégorie « complète » (index unique partiel) ; transitions d'état par `UPDATE … WHERE etat = <attendu>` dans une transaction (une approbation répétée ou simultanée échoue proprement).

## 4. Classement des exports

| Classe | Exemples | Règle |
|---|---|---|
| **Approbation simple** (une personne habilitée, passkey récente) | export d'une liste ou d'un tableau d'un domaine non sensible et de périmètre restreint (EDT, structure, paramétrage, liste d'une classe pour une sortie, statistiques agrégées sans nom) ; exports officiels D-022 (hors dispositif, déjà réservés aux administratifs) | journalisé ; plafonds de fréquence |
| **Double approbation** (deux personnes distinctes) | **export complet de l'établissement** ; export complet d'un domaine nominatif : élèves et responsables, dossier administratif, vie scolaire et discipline, évaluations et bulletins, **finances et comptabilité** (IBAN en clair), messagerie, documents et pièces jointes **en masse**, journaux d'audit ; **export périodique planifié** (approuvé une fois, reconfirmé chaque année et à tout changement de destination) | §5 |
| **Procédure particulière** | **santé, infirmerie, PAI, psychologue** (module désactivé tant qu'il n'y a pas d'HDS, A7 ; ensuite : archive séparée, clé distincte, approbation par la direction **et** le responsable santé, jamais incluse par défaut dans l'export complet) ; **protection** (adresses protégées, interdictions de contact, `ent_adresse_protegee`, `ent_eleve_interdiction`) : exclues par défaut, incluses seulement sur décision motivée ; **départ de la plateforme / réversibilité finale** (§10) ; **droits d'une personne** (accès, portabilité, art. 15/20) : extrait individuel par la procédure du socle RGPD, délai d'un mois (A13) ; **réquisition d'une autorité** : hors plateforme, procédure juridique | décrite au cas par cas, toujours journalisée |

Chaque table reçoit sa catégorie dans le **registre de classement** (E12, `PRA_SAUVEGARDES.md` §18.4) : un export complet contient toutes les tables de catégorie `simple` et `double`, jamais `particuliere` sans demande explicite ; une table non classée **bloque** l'export (et la CI).

## 5. Validation à deux personnes

### 5.1 Actions du catalogue (D-017) [R]
- `donnees.export.initier` — portée établissement ;
- `donnees.export.approuver` — portée établissement ;
- `donnees.export.planifier` — portée établissement (exports périodiques) ;
- `donnees.export.sante` — procédure particulière, en plus des précédentes.
Modèles par défaut : **direction** et **chef d'établissement** : initier + approuver ; **responsable administratif** : initier ; tout autre rôle seulement par octroi explicite. Garde-fou : ces actions sont **« sensibles »** — leur octroi est lui-même journalisé et notifié à tous les détenteurs actuels ; on ne peut pas se les octroyer (règle « pas d'escalade » existante).

### 5.2 Règles d'identité (contrôlées en base, pas seulement dans le code)
1. `approuve_par_user ≠ initie_par_user` (`CHECK`).
2. **Personnes distinctes** : `approuve_par_personne ≠ initie_par_personne` (D-020) — empêche une même personne avec deux comptes.
3. **Passkeys distinctes** : l'identifiant de credential de l'approbation ≠ celui de l'initiation (défense en profondeur ; une passkey appartient à une personne, D-013).
4. Une approbation par demande (`UNIQUE (demande_id)`) ; une décision est définitive.
5. Plusieurs sessions ou appareils d'une même personne ne changent rien : la règle porte sur la **personne**, pas la session.
6. Au moins **deux** personnes détentrices de `donnees.export.approuver` ou `initier` doivent exister pour que la double approbation soit possible ; sinon l'interface l'explique et propose la suppléance (§5.4).

### 5.3 Authentification
- Session ouverte par **passkey** (`assurance = 'passkey'`) **et** cérémonie WebAuthn de **réauthentification** avec vérification de l'utilisateur dans les **5 minutes** précédant l'initiation, puis l'approbation. À ajouter : `ent_session.authentification_forte_le TIMESTAMPTZ` mis à jour par chaque cérémonie réussie, et une route de réauthentification par passkey (le mot de passe ne suffit pas).
- Comptes sans passkey (familles hors sujet ici ; personnels pendant la transition D-009) : ne peuvent ni initier ni approuver.
- Le SSO (`assurance = 'sso'`) ne suffit pas seul : réauthentification passkey exigée.

### 5.4 Délégation, absence, révocation, remplacement
- **Suppléance** par `ent_delegation` : l'approbation peut être déléguée pour une période courte (≤ 30 jours), par une personne qui détient le droit, à une personne **habilitée compatible** (personnel de direction ou administratif). La délégation de `donnees.export.*` est notifiée à tous les détenteurs. Un délégué **ne peut pas approuver une demande initiée par son délégant** s'il tient son droit de cette seule délégation [R] (sinon le délégant valide « par procuration » sa propre demande).
- **Absence** (établissement avec une seule personne de direction) : désignation **à l'avance** d'un second approbateur extérieur à l'équipe quotidienne mais lié à l'établissement (président de l'organisme de gestion, adjoint, ou personne désignée par l'autorité de tutelle), avec un compte et une passkey. ScolIA ne fait **jamais** office de second approbateur.
- **Révocation** : l'initiateur (ou tout détenteur de `approuver`) peut révoquer une demande tant que l'extraction n'a pas commencé ; un approbateur qui perd son droit (fin d'accès, `ent_membership.etat`, migration 350) avant l'exécution **annule** l'approbation (contrôle au démarrage de la tâche).
- **Remplacement** des approbateurs : changement de rôle ordinaire (D-017), journalisé et notifié.

### 5.5 Administrateur technique
Il ne peut pas contourner : pas de route, pas de drapeau, pas de rôle applicatif « tout voir ». **Limite assumée** : l'opérateur de la plateforme qui détient l'accès propriétaire à la base ou aux sauvegardes **peut techniquement** extraire des données (c'est le PRA). Atténuations : accès infrastructure nominatif et journalisé (Audit Trail du fournisseur), procédure à deux personnes côté éditeur pour toute restauration, engagement contractuel (sous-traitant, RGPD art. 28), et **aucune** remise de données à un tiers hors de la procédure du §4.

## 6. Modèle de données (normalisé, D-036) [R]

| Table | Colonnes principales |
|---|---|
| `ent_export_demande` | `id`, `etablissement_id`, `classe` (`simple`/`double`/`particuliere`), `perimetre` (`complet`/`domaines`), `motif`, `etat`, `initie_par_user`, `initie_par_personne`, `initie_credential_id`, `initie_le`, `expire_le` (initiation + 48 h), `revoquee_par`, `revoquee_le`, `cree_le`, `modifie_le` |
| `ent_export_demande_domaine` | `demande_id`, `etablissement_id`, `domaine` (une ligne par domaine demandé) |
| `ent_export_approbation` | `demande_id` (unique), `etablissement_id`, `decision` (`approuvee`/`refusee`), `par_user`, `par_personne`, `credential_id`, `delegation_id` (facultatif), `motif`, `le` ; `CHECK` §5.2 par contrainte et déclencheur |
| `ent_export_archive` | `demande_id`, `etablissement_id`, `cle_objet`, `taille`, `sha256`, `format_version`, `instantane_le`, `migration_tag`, `expire_le`, `purgee_le` |
| `ent_export_telechargement` | `archive_id`, `etablissement_id`, `par_user`, `le`, `ip`, `appareil` |
| `ent_export_planification` | `etablissement_id`, `frequence`, `destination` (type + référence), `approuvee_le`, `reconfirmer_avant` |

États de `ent_export_demande` : `en_attente` → `approuvee` | `refusee` | `expiree` | `revoquee` ; `approuvee` → `en_cours` → `prete` | `echec` ; `prete` → `telechargee` (au moins une fois) → `purgee`. Toutes les tables sous RLS, toutes avec `etablissement_id`, toutes classées au registre (catégorie `double` : la liste des exports fait partie du journal).

**Journal** : chaque transition écrit `ent_journal_modif` (`domaine = 'export'`) **dans la même transaction** ; aucune transition sans entrée de journal. Les tables d'export ne sont modifiables que par les transitions prévues (déclencheur de contrôle des transitions).

## 7. Format de l'export [R]

### 7.1 Contenu de l'archive
```
scolia-export-<etablissement>-<date>/
  LISEZMOI.html              explication en français : contenu, catégories exclues et pourquoi, comment ouvrir, comment vérifier
  manifeste.json             voir 7.2
  manifeste.json.sig         signature Ed25519 du manifeste (clé publique publiée par ScolIA)
  dictionnaire/
    tables.csv               table, domaine, description, nombre de lignes
    colonnes.csv             table, colonne, type, description, valeurs possibles, unité
    relations.csv            table, colonne → table cible (clés étrangères)
  metier/                    VUE LISIBLE (tableur) : un CSV par objet métier, colonnes en français,
    eleves.csv               codes traduits en libellés, dates JJ/MM/AAAA, UTF-8 avec BOM, séparateur « ; »
    responsables.csv
    inscriptions.csv  notes.csv  absences.csv  factures.csv  …
  technique/                 VUE COMPLÈTE ET RÉIMPORTABLE : un fichier JSON Lines par table,
    ent_eleve.jsonl          une ligne = un enregistrement, types conservés (UUID, dates ISO 8601,
    …                        montants en centimes), identifiants stables
  fichiers/<espace>/<uuid>.<ext>   documents originaux, noms d'origine dans technique/ (colonne nom_fichier)
```
- **Pourquoi deux vues** : le CSV « métier » sert à l'établissement (Excel, LibreOffice) et à un autre éditeur ; le JSON Lines « technique » garantit la complétude et le réimport (le JSON est ici un **format d'échange**, compatible D-036). Pas de dump PostgreSQL.
- Champs chiffrés en base (IBAN…) : **en clair** dans l'archive seulement pour la catégorie qui les autorise (finances, double approbation) ; sinon masqués (`FR76 **** **** 1234`) avec la mention du masquage dans le manifeste.
- Comptes : identifiants, noms, adresses de connexion des personnes rattachées ; **jamais** de condensat de mot de passe, de passkey, de jeton, de session.

### 7.2 Manifeste (`manifeste.json`)
`format` (`scolia-export/1`), `etablissement` (id, nom, UAI), `demande_id`, `initiateur`, `approbateur` (noms et fonctions), `instantane_le` (instant de la transaction cohérente), `genere_le`, `version_logiciel` (`git_sha`), `migration_tag`, `categories_incluses`, `categories_exclues` (avec raison), `fichiers` : pour chacun `chemin`, `taille`, `sha256`, `lignes` (pour les tables), `objets_manquants` (référencés en base mais absents du stockage : liste, jamais silencieux), `controles` (fermeture des références : résultat). La vérification d'intégrité se fait **sans ScolIA** : `sha256sum` + vérification de signature avec la clé publique (procédure dans `LISEZMOI.html`).

### 7.3 Chiffrement de l'archive
[R] **ZIP chiffré AES-256** (format WinZip AE-2, ouvrable avec 7-Zip, libre et gratuit), produit en Rust (crate `zip`, MIT, à revoir par `cargo deny`). Phrase de passe aléatoire forte, **affichée une seule fois** à l'initiateur au moment du téléchargement et **jamais stockée** en clair ; l'approbateur est notifié du téléchargement. Alternative étudiée : `age` (BSD-3, crate `rage` MIT/Apache) — plus moderne mais moins accessible au personnel d'un établissement. **[Q-EXP1]** Florian : ZIP AES (accessible) ou `age` (plus sûr, outil à installer) ?

### 7.4 Réimport
- **Dans ScolIA** (même plateforme après une erreur, ou autre instance — réversibilité inverse, arrivée d'un établissement) : par `ent-extraction` en mode lecture du format technique, identifiants conservés, **dans un établissement vide** ou par fusion contrôlée (rapprochement comme `ent-reprise`, `TRANSITION_T0_INVENTAIRE.md`) ; comptes recréés **sans secret** (réinvitation et nouvelle passkey). Version de format différente : convertisseur fourni pour les versions N-1 et N-2.
- **Dans un autre logiciel** : CSV métier + dictionnaire ; pas de garantie d'import automatique (dépend du logiciel cible) ; c'est la limite documentée.
- **Limites** : les historiques internes techniques (files d'envoi, sessions) ne sont pas exportés ; les fichiers supprimés ne le sont pas ; les données d'autres établissements partagées par une personne ne le sont jamais.

## 8. Sécurité, rétention, abus

| Règle | Valeur proposée [R] |
|---|---|
| Expiration d'une demande non approuvée | 48 h |
| Lien de téléchargement | nominatif (session de l'initiateur ou de l'approbateur), réauthentification passkey, URL pré-signée **5 min**, 3 téléchargements au plus |
| Rétention de l'archive | **7 jours** puis suppression automatique (objet + trace `purgee_le`) ; bucket « exports » **exclu** de la réplication et des sauvegardes (sinon l'archive survivrait 35 jours ailleurs) |
| Fréquence | 1 export complet par 7 jours et par établissement ; 20 exports simples par jour et par personne |
| Alertes | export complet demandé hors heures ouvrées, téléchargement depuis un nouvel appareil ou un pays inhabituel, demandes répétées refusées, octroi récent des droits d'export suivi d'une demande → notification à **toutes** les personnes de direction et au tableau de sécurité de l'opérateur |
| Notifications | à l'initiation, à l'approbation ou au refus, à la fin, au téléchargement, à la purge : à toutes les personnes détentrices de `donnees.export.*` |
| Périmètre | extraction sous `TenantTx(X)` strict + contrôle de fermeture ; toute référence hors de X **fait échouer** l'export (et alerte l'opérateur : c'est un défaut de cloisonnement) |
| Journal | non modifiable par les utilisateurs (ajout seul) ; durée de conservation à valider par le DPO (A11 : 6 mois à 1 an pour les journaux d'accès ; [R] **durée plus longue pour les exports** — 3 ans ? — à demander) |

## 9. RGPD [à valider DPO]
- L'établissement est **responsable de traitement** ; ScolIA est **sous-traitant** (art. 28) : l'export complet est la mise en œuvre de l'obligation de restitution (art. 28.3.g) et de la réversibilité contractuelle ; il est remis au responsable de traitement lui-même, pas à un tiers.
- L'export est un **traitement** : mentionné au registre de l'établissement et dans l'AIPD (A15).
- **Minimisation** : santé et protection exclues par défaut ; secrets jamais exportés ; masquage des champs non nécessaires selon catégorie.
- Le droit à la **portabilité** (art. 20) appartient aux **personnes** (familles, élèves majeurs, personnels) : procédure individuelle distincte (§4), pas l'export d'établissement.
- **Conservation** : une archive téléchargée échappe à ScolIA ; le `LISEZMOI` rappelle à l'établissement ses obligations (stockage chiffré, durée, destruction).

## 10. Réversibilité et restitution
- **Sans intervention de l'éditeur** : la direction lance l'export complet (double approbation) à tout moment, y compris pendant le préavis de fin de contrat.
- **Export périodique planifié** [R] : mensuel, déposé chez l'établissement (bucket S3 lui appartenant, ou SFTP) ou mis à disposition 7 jours ; approuvé une fois par deux personnes, **reconfirmé chaque année** et à tout changement de destination ; échec ou destination injoignable → alerte à la direction.
- **Départ** : procédure particulière : export final, vérification par l'établissement (manifeste), puis suppression des données de l'établissement dans la production à la date convenue (ex. fin de contrat + 3 mois), **attestation de suppression** ; suppression effective dans les sauvegardes **à l'expiration de leur rétention** (35 j / 12 mois), écrite dans le contrat. Sous réserve des obligations de conservation et d'archives (B10 : visa des archives dans le public).
- **Contenu, formats, limites et réimport** : ce document, §7, publié aux établissements sous forme de notice.

## 11. Décisions à prendre avant l'implémentation
Techniques : D-041 (double validation, réauthentification passkey récente, actions du catalogue) — **maintenant**, car elle ajoute une colonne de session et des actions au catalogue ; D-037 (moteur d'extraction unique et registre) — **maintenant** ; D-046 (format d'export, chiffrement de l'archive, Q-EXP1) — avant le lot.
Réglementaires (DPO / juriste) : durée de conservation du journal des exports ; inclusion des IBAN en clair ; clause de réversibilité et délai de suppression après départ ; procédure santé (HDS, A7/B9) ; archives publiques (B10) ; rôle de l'approbateur extérieur en cas d'absence.

## 12. Lots proposés (pour plus tard, après décisions)
1. **EXP-0** : registre de classement des tables + test CI (sert aussi au PRA) — peut commencer tout de suite.
2. **EXP-1** : `ent-extraction` (lecture cohérente, fermeture, format technique, manifeste signé) + tests d'isolation dans la matrice.
3. **EXP-2** : réauthentification par passkey (`authentification_forte_le`), actions `donnees.export.*`.
4. **EXP-3** : demandes, approbations, états, journal transactionnel, notifications, expiration.
5. **EXP-4** : archive chiffrée, bucket d'exports, liens courts, purge.
6. **EXP-5** : vue métier CSV + dictionnaire + `LISEZMOI`.
7. **EXP-6** : réimport dans un établissement vide ; exports planifiés.
