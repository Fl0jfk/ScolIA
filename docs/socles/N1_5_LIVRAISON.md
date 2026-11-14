# Socle N1 — lot N1-5 : reprise R9 des données du Next.js (livraison)

Conception suivie : `docs/socles/N1_DEMANDES.md` §29 (reprise), §31 (migrations), §37, §38 (D-061), DEM-341 à DEM-344, DEM-349, DEM-364, DEM-365,
CDM-50, R-DM49 — sur la branche `ent-mc0hoe`.
Code : `ent-db/src/demandes_reprise.rs` (outil de reprise, guichet), garde d'écriture dans `demandes.rs` (dépôt et `charger_pour_acces`).
Migrations **3048** (`demandes_n1_5_reprise` : lot, correspondances, demandeurs, propositions) et **3049** (`demandes_n1_5_bascule` : guichet,
garde de rapprochement, blocage des séances d'une HSE reprise). Tests : `tests_demandes_n1_5.rs`.

**Aucun accès à la base ni au stockage du Next.js.** L'outil lit le **texte** d'un export anonymisé que Florian fournira plus tard ; il est
codé et testé avec un **jeu fictif** au format ci-dessous, jamais avec des données réelles.

## 1. Format de l'export attendu (version 1)

Un seul document JSON (UTF-8, 64 Mio au plus). C'est la forme des tables et fichiers du Next.js (`db/schema-ent-relational.ts`,
`demandes-hse/index.json`, `photocopies-couleur/index.json`), **anonymisée de façon cohérente** : les noms, adresses et identifiants sont
remplacés par des pseudonymes stables (un même agent a la même adresse pseudonyme partout), les adresses restent **de forme d'adresse**
(l'outil ne retrouve un agent que par adresse exacte, sans casse). Les clés sont celles du Next.js : `snake_case` pour `request`,
`camelCase` pour les HSE et les photocopies (c'est ainsi qu'ils sont stockés).

```json
{
  "format": "scolia-reprise-nextjs",
  "version": 1,
  "comptes": [ { "user_ref": "u_12", "user_id": "<uuid du compte Rust ou null>" } ],
  "requests": [ {
      "id": "req_…", "created_at": "2026-11-02T09:00:00Z", "updated_at": "…", "status": "NOUVELLE|EN_COURS|EN_ATTENTE|TERMINEE",
      "category": "", "subject": "…", "description": "…",
      "requester_user_id": "u_12 ou null", "requester_email": "adresse pseudonyme ou vide",
      "assigned_unit": "maintenance", "assigned_email": "agent@…", "assigned_route_id": "maintenance", "purge_at": "…ou null",
      "attrs":       [ { "path": "gravite", "value": "gene" } ],
      "comments":    [ { "id": "…", "at": "…", "by_name": "…", "by_user_id": "u_3 ou null", "body": "…", "visible_demandeur": false } ],
      "history":     [ { "at": "…", "action": "Assignée" } ],
      "attachments": [ { "id": "…", "file_name": "…", "s3_key": "…", "content_type": "…", "comment_id": "ou null" } ] } ],
  "routing": { "tasks": [ { "id": "maintenance", "label": "Maintenance" } ] },
  "org":     { "units": [ { "id": "maintenance", "label": "…", "managerEmails": [], "memberEmails": [], "taskIds": [], "active": true } ] },
  "hse":     [ { "id": "…", "createdAt": "…", "updatedAt": "…", "status": "EN_ATTENTE|ACCEPTEE|REFUSEE|ANNULEE",
                 "createdBy": { "userId": "u_12", "name": "…", "email": "…" }, "resumeDemande": "…", "motif": "…",
                 "nombreHeures": 4.0, "classe": "4B", "details": "…",
                 "decidedBy": { "userId": "u_3", "name": "…" }, "decidedAt": "…", "acceptancePdfPath": "…" } ],
  "photocopies": [ { "id": "…", "createdAt": "…", "status": "EN_ATTENTE|ACCEPTEE|PRETE|REFUSEE", "typeImpression": "COULEUR|NOIR_BLANC|null",
                 "createdBy": { "userId": "u_12", "name": "…", "email": "…" }, "motif": "…", "classesOuMatiere": "…",
                 "nombrePhotocopies": 120, "documents": [ { "key": "…", "fileName": "…", "contentType": "application/pdf" } ],
                 "decidedAt": "…" } ]
}
```

- **`comptes`** est la **table de passage des comptes** du socle 28 (mise en service) : clé du Next.js → compte Rust. C'est la **seule** façon de
  retrouver un demandeur ou un décideur. Un `user_id` nul, ou un compte sans fiche de personne dans l'établissement, vaut « non retrouvé ».
- Les champs absents prennent leur valeur vide ; un champ inconnu est ignoré ; un document qui n'est pas de ce format (format, version, JSON)
  est **refusé avant tout traitement**, sans rien écrire.
- Horodatages : RFC 3339, ou un jour `AAAA-MM-JJ`. Les jours dérivés (conservation, début d'une HSE) sont ceux de **l'établissement**.
- Questions ouvertes sur le format (à confirmer quand l'extraction sera écrite) : l'export doit porter `visible_demandeur` s'il veut qu'un
  commentaire soit lisible du demandeur (le Next.js n'a pas cette notion : par défaut **tout est repris interne**) ; le Next.js ne stocke pas
  la période d'une HSE ni le nombre de pages d'un tirage (voir §3).

## 2. Ce qui est fait (implémenté et testé en labo local)

| Sujet | Où | Test |
|---|---|---|
| Lecture d'un export ; refus si format, version ou JSON invalide ; rien d'écrit | `reprendre_for_actor` | `t_reprise_droit_et_export_refuses` |
| Droit `demandes.reprise` (administrateur ; la direction ne l'a pas, D-017) pour reprendre, lire le rapport, régler le guichet, rapprocher | `controle_reprise` | `t_reprise_droit_et_export_refuses`, `t_guichet_lecture_seule` |
| Demandes ouvertes : nature déduite de la route, file par le routage de N1-1 (corbeille et route inconnue → file de tri), statut (`NOUVELLE`→déposée, `EN_COURS`→prise si l'agent est retrouvé par l'adresse de sa fiche, sinon remise en file, `EN_ATTENTE`→en attente d'un tiers), échéances recalculées au jour de la reprise, historique en faits `reprise_historique`, commentaires **internes** par défaut, description, valeurs typées quand le champ est déclaré par la nature (sinon note interne) | `reprendre_requete` | `t_reprise_demandes_ouvertes_et_closes` |
| Demandes closes : archive en lecture seule (statut `close`, issue `traitee`), conservation = jour d'établissement de `purge_at` sans prolongation ; purge déjà échue → ignorée ; pas de date de purge → à vérifier | idem | idem |
| **Rejouable** : une ligne de correspondance par objet source (`request`, `demandes_hse`, `photocopies`, `*_piece`, `org`, `routing`) ; une passe rejouée sur le même export ne crée rien (demandes, faits, personnes, numéros), note le passage, retente seulement les « à vérifier » ; l'empreinte SHA-256 de l'export reconnaît un export déjà lu | `passe_objet`, `noter` | `t_reprise_rejouable` |
| Une erreur sur un objet ne bloque pas les autres : transaction par objet, classé « à vérifier » (`erreur_technique`), détail au journal technique seulement | `passe_objet` | (voir limites) |
| **Demandeur** (NJ-84, NJ-88) : table de passage seulement ; sinon une personne de reprise « à rapprocher » (adresse **masquée**, jamais en clair, même adresse = même ligne) ; **aucun rattachement par e-mail** ; aucun accès pour la personne dont l'adresse « correspond » | `demandeur_de_reprise` | `t_reprise_demandeur_sans_email` |
| Rapprochement par le secrétariat (liste « demandeurs à rapprocher », puis rattachement des demandes reprises à la personne retrouvée, fait `rapprochement` par demande, chaîne intacte) ; hors de ce chemin, le demandeur ne change **jamais** (R-DM4 maintenue en base) | `rapprocher_demandeur_for_actor`, garde de 3049 | idem |
| **HSE** de l'année : `EN_ATTENTE`→`en_validation` (décision à refaire dans ScolIA) ; `ACCEPTEE`→close, issue accordée, volume accordé et décideur **seulement s'il est retrouvé** (jamais une décision attribuée à autrui), marquée « transmise hors ScolIA » ; `REFUSEE`, `ANNULEE`→archive ; année précédente → ignorée ; volume absent ou invalide → à vérifier ; bénéficiaire = l'agent du demandeur (table de passage, à défaut adresse de sa fiche d'agent) ; PDF d'acceptation par le port du socle 7 | `reprendre_hse` | `t_reprise_hse_cdm50` |
| **CDM-50, zéro heure au 15** : aucune séance créée ; déclencheur de base qui **interdit toute séance** sur une HSE reprise comme transmise (donc jamais d'attestation, jamais d'appel au port du socle 15) ; compteur `heures_creees_au_15` du lot contraint à 0 | migration 3049, 3048 | `t_reprise_hse_cdm50` |
| **Photocopies** ouvertes seulement (`EN_ATTENTE`, `ACCEPTEE`) ; `PRETE` et `REFUSEE` ignorées (perdables, T0) ; couleur→validation par la direction ; PDF par le port du socle 7 | `reprendre_photocopie` | `t_reprise_photocopies_ouvertes` |
| **Organisation** (`requests-routing`, `requests-org`) : **proposition** de files (une par unité active), de natures (par tâche), de désignations **datées** (responsables et membres retrouvés par adresse), d'adresses non retrouvées (masquées) ; statut `proposee`, jamais appliquée ; rejouable | `proposer_organisation` | `t_reprise_organisation_en_proposition` |
| **Rapport** (DEM-343) : comptages par source, décision et raison ; les deux critères du go/no-go ; pièces non reprises ; demandeurs à rapprocher ; échantillon de 20 demandes reprises (`source_id`, numéro, nature, statut) ; durée mesurée | `rapport_du_lot`, `rapport_reprise_for_actor` | `t_reprise_demandes_ouvertes_et_closes`, `t_reprise_droit_et_export_refuses` |
| **Guichet en lecture seule** (DEM-349, DEM-365) : réglage par l'administrateur (raison `bascule`, `incident` ou `maintenance`), journalisé ; dépôt et **tous les gestes d'écriture** (porte unique `charger_pour_acces` avec verrou) refusés proprement par une phrase claire (409) ; la lecture reste libre ; la reprise de **bascule** exige le guichet en lecture seule | `regler_guichet_for_actor`, `etat_guichet` | `t_guichet_lecture_seule` |
| R-DM49 : la reprise ne publie **aucun** événement ni avis | — | `t_reprise_demandes_ouvertes_et_closes` |
| D-036 : aucune colonne JSON, tableau, binaire, adresse ou téléphone dans 3048-3049 ; RLS par `ent_rls_poser` ; registre et compte RLS à jour | migrations | `t_tables_n1_5_sans_json_ni_adresse`, `rls_politiques_presentes_et_forcees` |

### Critères de fin du plan (§29.2) — où ils en sont

| Critère | État |
|---|---|
| 100 % des demandes ouvertes reprises et attribuées à une file | **testé** sur jeu fictif (`RapportReprise::requetes_ouvertes == requetes_ouvertes_reprises`) ; à vérifier sur le vrai export |
| 0 heure HSE créée au 15 par la reprise | **testé** (compteur contraint, déclencheur, port jamais appelé) |
| 0 pièce illisible | **non atteint en l'état** : le socle 7 n'est pas versé, donc **aucune pièce n'est copiée** (`PiecesAbsentes`) ; chaque pièce est listée « à vérifier » et le rapport dit `go_no_go() == false`. Atteint dès que le socle 7 branche `PortPiecesReprise` (testé avec un port fictif) |
| Répétition de Noël et du 15 janvier « sans écart » | **non faisable sans export** ; l'outil, le rapport et le rejeu sont prêts |
| 20 demandes ouvertes relues | geste **humain** ; l'échantillon est dans le rapport |

## 3. Choix et hypothèses à confirmer (hypothèses, pas des faits)

- **`EN_ATTENTE` du Next.js → « en attente d'un tiers »** (et non « en attente du demandeur ») : l'ancienne application ne dit pas qui est attendu ; ce statut ne déclenche aucune relance au demandeur. À confirmer par Florian.
- **Nature déduite de la route** : `maintenance*`→`maintenance.intervention` ; `comptabilite*`→`comptabilite.demande_interne` ; `admin*` et `accueil*`→`secretariat.demande_interne` ; `rh*`→`rh.demande_agent` ; `cpe_*` et `vie_scolaire*`→`vie_scolaire.demande_interne` ; le reste (dont `corbeille`)→ file de tri. Les vraies routes de l'établissement seront comparées à cette table au premier export ; une route mal reconnue part au tri, jamais perdue.
- **HSE sans période** : le Next.js ne stocke pas de période ; la reprise prend du jour de la demande (au plus tôt le début de l'année) à la fin de l'année scolaire. Le **motif** est repris en texte (jamais en code), le type d'heure est `hse_autre` ; la classe en note interne. Le volume est converti en quarts d'heure (4 h = 16).
- **Photocopies** : le Next.js ne stocke pas le nombre de pages ni de « pour le » : la reprise compte 1 page par exemplaire (note interne le disant) et **pour le jour de la reprise** ; les copies totales sont celles de l'ancienne demande. La file d'impression doit avoir un site (sinon, si l'établissement a plusieurs sites, la demande est « à vérifier » : `site_introuvable`, on ne devine pas).
- **Agent retrouvé par adresse** pour la prise en charge, les HSE et l'organisation : **personnels seulement**, adresse exacte (sans casse) de la fiche de personne d'un agent. Jamais pour un demandeur.
- **Échéances** d'une demande ouverte reprise : le décompte **repart du jour de la reprise** (délais de la nature, jours ouvrés du service). Une demande reprise n'est donc pas « en retard » le jour de l'ouverture ; à confirmer par Florian (alternative : conserver l'ancienne date de dépôt).
- **3049** : l'inventaire du plan attribuait 3049 aux exclusions et restrictions (passe personas, N1-2) ; elle est ici utilisée par ce lot (consigne du pilote : plage 3048–3049). Si la passe personas est codée plus tard, elle prendra un autre numéro de la plage réservée (3050–3055).

## 4. Limites connues (à ne pas présenter comme faites)

- **Aucune route HTTP, aucun écran** : l'outil est un service (`PgEnt`) à appeler depuis un binaire ou une commande d'administration (non écrits : l'export n'existe pas encore). Le geste « lecture seule » côté **Next.js** est celui de Florian.
- **Socle 7 absent** : aucune pièce (pièces de demande, PDF d'acceptation HSE, PDF de photocopies) n'est copiée ; antivirus et contrôle de type viendront avec le port. Le PDF d'acceptation n'est donc **pas** déposé au coffre de l'agent.
- **Avis de reprise unique par personne (DEM-364)** non fait : la reprise n'envoie rien (testé) ; le mécanisme d'envoi du jour d'ouverture (8 h, SN1-16, enveloppe neutre, aucun avis à un demandeur non rapproché) reste à écrire avec le socle 8.
- **Propositions** : lues par le rapport (comptes) et la table ; pas d'écran de relecture ni de geste « retenir / rejeter » ; la conversion en files, règles et missions se fait à la main après relecture.
- **Retour arrière** : pas de suppression ; la reprise est idempotente et le Next.js reste intact (§29.2). Une reprise de répétition se « défait » en réinitialisant la base de répétition.
- **Deux exports successifs** (Noël puis 15 janvier) : une demande déjà reprise n'est **pas réécrite** même si son statut a changé dans l'export suivant (l'écart n'est pas détecté) ; à traiter sur une base de répétition remise à zéro, ou à ajouter si Florian veut une reprise incrémentale.
- Une erreur technique sur un objet le classe « à vérifier » sans qu'un test la provoque (le chemin est écrit, pas exercé).
- Le critère « 20 demandes relues », la relecture des propositions et la lisibilité des pièces restent des gestes humains.

## 5. Vérification de N1-3 et N1-4 (reprise de la session précédente)

Voir la section « Vérification » du message de livraison au pilote : l'état de chaque suite est donné avec le SHA exact de la tête et le résultat
de l'unique CI.
