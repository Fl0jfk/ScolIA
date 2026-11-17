# Lot R3 (MS4 d) — rapprochement des référentiels et des personnes, file « à décider »

Statut : **implémenté et testé en local (PostgreSQL 16)** sur l'export **fictif** au format Next.js (R0) et sur des jeux d'essai ; CI : une seule, en fin de lot (SHA et numéro de run transmis au pilote). **Jamais exécuté sur un export réel** : rien ici ne prouve « prêt pilote » ni « prêt production ». Non validé par Florian.
Branche `ent-socle-28-code-r3`, base `origin/ENT` (qui contient R2). Aucune base de production, aucun secret, rien du Next.js touché.

## 1. Ce que livre le lot

| Brique | Où | Rôle |
|---|---|---|
| Rapprochement pur | `ent-reprise/src/rapprochement/` | pour chaque fiche de l'export (élève, responsable, personnel, enseignant, compte, année, site, classe) : un **verdict explicable** — rapprochée sans attendre, archivée, ou « à décider » avec ses candidats, ses **raisons** et sa **confiance** |
| Décisions (pur) | `rapprochement/decision.rs` | règles de validité d'une décision (rattacher / créer / ignorer), éligibilité au lot, correspondance posée |
| Simulation | `simulation.rs` (`simuler_avec_rapprochement`) | la simulation de R2, puis le rapprochement ; nouveau compte « rapprochées » ; **aucune écriture métier** |
| Instantané de ScolIA | `ent-db/src/reprise_rapprochement.rs` | lecture seule, filtre `etablissement_id`, de ce qui sert à comparer ; trace de la file ; purge des résumés |
| File « à décider » | `ent-db/src/reprise_file.rs` | lister, résumer, décider, décider en lot, annuler, équivalences de libellés — droit `reprise.decider`, côté serveur |
| Base | migration **2567** | `rep_proposition*`, `rep_decision*`, `rep_synonyme`, compte `rapprochees` |
| Droit | `ent-auth` : `reprise.decider` | direction et secrétariat (`Administratif`) ; personne d'autre |

## 2. Règles de rapprochement (T0 §6, R-MSE22)

**Principes appliqués** : le cœur fait foi (aucune fiche créée sans décision) ; clé forte d'abord, jamais d'approximation appliquée seule ; tout est explicable ; un résultat par fiche.

| Objet source | Rapprochée **sans attendre** (seulement) | Proposée (« à décider ») | Jamais automatique |
|---|---|---|---|
| `eleve` | **INE** identique, unique dans l'export **et** dans ScolIA, de forme valide, sans contradiction (date de naissance, identité) | nom + prénom + naissance exacts (**forte**) ; nom de naissance, prénom secondaire, nom composé (**moyenne**) ; une lettre d'écart, naissance absente ou différente (**faible**) | tout ce qui n'est pas l'INE |
| `foyer_responsable` | — | élève du foyer déjà rapproché + nom/prénom/e-mail/téléphone (jusqu'à **forte**) | toujours une décision (un responsable n'a pas d'identifiant national) |
| `personnel`, `enseignant` | — | e-mail professionnel + nom (**forte**) ; e-mail personnel = simple indice | toujours une décision |
| `user` (comptes) | — | e-mail normalisé (**forte**) ; compte « administrateur de plateforme » signalé | toujours une décision |
| `annee_scolaire` | mêmes dates de début **et** de fin, uniques des deux côtés | dates à ≤ 7 jours (signalé) ; même libellé | — |
| `classe_site_mapping` | code de division SIECLE identique au code ScolIA, unique des deux côtés | libellé équivalent (« Seconde B » = « 2nde B ») | « 2B » n'est **jamais** déduit de « 2nde B » |
| `etablissement_site` | — | libellé ou code identique | — |
| Équivalence **déjà validée** par l'établissement (classe, site) | rejouée automatiquement (c'est une décision humaine) | — | — |

Cas **toujours présentés à un humain** : INE identique avec naissance différente ou identité discordante ; INE en double (dans l'export ou dans ScolIA) ; jumeaux possibles (même nom et même naissance, prénoms différents : jamais confondus, signalés) ; deux fiches de l'export pour une fiche de ScolIA ; fiche de ScolIA déjà reliée à une autre fiche ; adresse e-mail **partagée** (deux parents : fréquent) ; élève « actif » mais à date de sortie passée (incohérence de l'export).
**Archivés sans décision** (T0 §6.2, §6.4) : élève de l'export **sorti** et absent de ScolIA ; année **passée** absente de ScolIA (jamais créée). Un élève actuel sans candidat n'est jamais archivé en silence : « aucun candidat » → seule une décision peut le créer.

Raisons (codes stables, textes sans aucune valeur de donnée, `rapprochement/types.rs`) : 33 codes, de polarité *pour* (ex. `ine_identique`, `nom_identique`, `naissance_identique`, `meme_eleve`, `email_pro_identique`, `dates_identiques`, `synonyme_valide`), *contre* (ex. `naissance_differente`, `ine_en_double_next`, `email_partage`, `cible_deja_reliee`, `cible_plusieurs_sources`, `ambiguite_dans_le_coeur`) ou *info* (`jumeaux_possibles`, `naissance_inconnue`, `email_personnel_identique`). Confiance : `faible < moyenne < forte < certaine` (« certaine » est réservée aux verdicts automatiques). Au plus 5 candidats par ligne, classés de façon déterministe.

## 3. File « à décider »

- **Droits** : toute méthode de `PgEnt` (`reprise_file_lister`, `reprise_file_resume`, `reprise_decider`, `reprise_decider_lot`, `reprise_decision_annuler`, `reprise_synonymes_lister`, `reprise_synonyme_retirer`) exige `reprise.decider` **avant** toute lecture. Testé : professeur, comptabilité, parent → `Forbidden` sur chaque méthode ; secrétariat et direction → accès ; un autre établissement ne voit rien et ne peut pas décider une ligne qui n'est pas la sienne.
- **Minimisation** : la file ne montre que nom, prénom, date de naissance, classe, **fin** d'INE (« …890A »), adresse **masquée** (« j***@domaine.fr »), un détail court. Jamais l'INE entier, jamais d'adresse postale, de téléphone ni de santé. À la **purge de la zone** (MSE-179) ces résumés sont mis à NULL (source et candidats) ; les propositions, les raisons et les décisions restent comme preuve, sans donnée d'identité. Le **rapport** (JSON et texte) ne contient aucune valeur de donnée : testé par recherche des noms, INE et e-mails de l'export dans les deux formes.
- **Décisions** : `rattacher` (à un candidat **proposé**), `créer`, `ignorer`. Validées côté serveur : un motif est obligatoire pour **ignorer**, pour rattacher malgré une contradiction ou une confiance faible/moyenne, et pour **créer** alors qu'une fiche forte existe ; motif 3 à 500 caractères. Une décision refusée ne laisse aucune trace.
- **Trace** : `rep_decision` — qui (`par_user_id`), quand (`le`), quoi, motif, confiance vue, « en lot » ; `rep_decision_raison` garde les raisons **affichées au moment de décider**. Table en **ajout seul** (déclencheur) : ni suppression ni réécriture, seule l'annulation peut être posée, une fois.
- **Rejouée** : une décision pose une correspondance (`rep_correspondance`, statut `rapprochee` / `creee` / `ignoree`) qui **prime** sur la dérivation à toutes les exécutions suivantes.
- **Réversible** : `reprise_decision_annuler` (motif obligatoire) remet la ligne « à décider » (correspondance réinitialisée, équivalence de libellé liée annulée) **tant qu'aucune exécution d'application n'est terminée** ; ensuite, toutes les méthodes de décision refusent (`ConflitDossier`).
- **En lot** : `reprise_decider_lot` (500 lignes au plus) — *rattacher le meilleur candidat* seulement si **confiance forte et aucune contradiction** ; *créer* seulement les lignes **sans aucun candidat**. Les lignes non éligibles sont écartées avec leur raison et restent « à décider ». Chaque décision du lot est tracée à l'unité, marquée « en lot ». Une adresse partagée, une naissance différente, une fiche déjà reliée ne passent jamais en lot.
- **Périmée** : une proposition d'une exécution plus ancienne que la dernière ne peut pas être décidée (relancer la simulation) — évite de décider sur des candidats qui ont changé.
- **Équivalences** (`rep_synonyme`) : rattacher une classe ou un site mémorise « libellé de l'export → fiche ScolIA », validée par la personne ; rejouée ensuite automatiquement ; listable et retirable.

## 4. Simulation et rapport

`simuler_avec_rapprochement` rejoue R2 puis le rapprochement. **Invariant étendu** (par objet et en total, vérifié par les tests et par une contrainte `CHECK` de la base) :
`lues = créées + mises à jour + inchangées + archivées + ignorées + refusées + à décider + rapprochées`.
« Rapprochée » = lien posé vers une fiche **existante** de ScolIA (règle forte ou décision rejouée) ; elle ne crée ni ne modifie aucune fiche. Une création **décidée** est comptée « créée » (proposée, jamais faite par la simulation). Le rapport ajoute une synthèse sans donnée : rapprochées par règle, à décider par motif et par meilleure confiance, archivées par raison, lignes **hors R3**.
**Lignes hors R3** : objets de rapprochement qui dépendent d'un autre lot (EDT, plannings, groupes, rôles, appartenances, nomenclatures, foyers, régimes…) restent « à décider » avec le motif `rapprochement_hors_r3` ; elles n'entrent pas dans la file humaine de ce lot.
**Aucune écriture dans le métier** : testé contre la base par comptage de toutes les tables hors `rep_*` avant et après (identique). **Rejouable** : deux simulations ont la même empreinte de rapport ; après décision puis enregistrement des correspondances, le rejeu ne repose pas la question (0 rapprochement nouveau, la décision devient « inchangée »). **Aucun test ne lit l'horloge** : le jour de référence (élève sorti, année passée) est un paramètre, pris dans l'instant fourni aux options.

## 5. Conformité

- **Registre RGPD** : le traitement de base `reprise_mise_en_service` (créé par R2, domaine `mise_en_service`) couvre les nouvelles tables (même domaine : **aucun nouveau domaine**) ; sa finalité, ses destinataires (opérateur ScolIA pour la zone ; secrétariat et direction pour la file) et sa lecture (`reprise.decider`) sont mis à jour. Les tables `rep_proposition`, `rep_proposition_candidat` et `rep_decision` sont déclarées **données personnelles** au registre des tables, avec la raison « hors extrait » (transit purgé).
- **Santé** : aucune donnée de santé lue ni affichée (R-MSE5) ; les 7 tables de santé restent refusées par R2 sans être lues.
- **Isolation SQL** : toutes les requêtes `rep_*` et `ent_*` de ce lot portent `etablissement_id` dans le même littéral (garde statique), sauf aucune exception déclarée. `ent_user` (identité globale) n'est lu que joint à `ent_membership` de l'établissement.
- **RLS** : 6 tables, politique d'établissement forcée (compte du test RLS : +6).

## 6. Vérifications (honnêtes : ce qui a tourné)
- `ent-reprise` : 152 tests unitaires et tests d'intégration (dont 6 nouveaux de simulation avec rapprochement et 66 unitaires du rapprochement), R0, R1, R2 inchangés et verts.
- `ent-db` sur PostgreSQL 16 local (`ENT_ENV=test`) : **10 tests `tests_reprise_r3`** verts (simulation sans écriture métier, INE automatique, file avec raisons et confiance, minimisation, rejeu à même empreinte, droits refusés à professeur / comptabilité / parent sur chaque méthode, cloisonnement entre établissements, décision tracée et ajout seul, rejeu, annulation, validations côté serveur, lot, reprise appliquée qui fige, équivalence de classe rejouée puis retirée, purge des résumés) ; 8 tests `tests_reprise_r2` ; registre des tables, extrait personnel, conservation, registre RGPD, test RLS (+6), gardes statiques d'isolation SQL, de jour UTC, d'accès : verts. `ent-auth` : 57 verts (droit `reprise.decider`).
- **clippy 1.99.0** `--workspace --tests --features ent-db/labo` : 2 avertissements trouvés, **tous deux dans les lignes du lot, corrigés** ; relance sur `ent-db` : 0. `cargo fmt -p ent-reprise` ; fichiers nouveaux d'`ent-db` formatés un à un ; aucun `cargo fmt --all` ; aucun `#[ignore]`.
- `CARGO_BUILD_JOBS=1` (et non 2) : à 2 travaux le compilateur est tué (mémoire) sur `ent-db` dans ce conteneur.
- La suite complète d'`ent-db` n'a pas été rejouée en entier (seuls le lot et les gardes concernées) : la CI fait foi. **CI** : une seule, après fusion d'`origin/ENT` (seul `ent-rust/web/Cargo.lock` a changé ; aucun compteur en conflit : actions 244 → 245, RLS +6 recalculés depuis la base commune).
- Corrigé en chemin : marqueurs de conflit dans `MIGRATIONS.md` (hérités de R2) ; catégorie de conservation `mes.reprise` ajoutée (exigée par le registre pour les tables « données personnelles »).

## 7. Ce que le lot ne fait pas
Pas d'écran ni de route HTTP (méthodes `PgEnt` seulement) ; pas d'événement `reprise.decision` ni de journal de consultation de la file ; `rep_echantillon` et `rep_bascule*` (lot MS4 e) ; pas d'application de la reprise (deux personnes, lot suivant) ; indice `siecle__eleve-id-map` non utilisé ; pas de rapprochement des groupes, EDT, plannings, rôles, appartenances (motif `rapprochement_hors_r3`) ; pas de recherche libre d'une fiche ScolIA hors des 5 candidats proposés ; classes comparées à l'année **courante** seulement (l'export n'a pas d'année sur les classes) ; matières : aucune table de matières dans l'export, rien à rapprocher (le domaine `matiere` est prévu dans `rep_synonyme`) ; aucune donnée de santé.

## 8. Questions et choix à valider (pilote / Florian)
1. **Règle 3 de T0 §6.4** (nom + prénom + naissance exacts, unique) : proposée en confiance *forte*, **jamais automatique**, par prudence (consigne : seule l'identité forte rapproche). La rendre automatique éviterait des centaines de décisions en lot, au prix d'un risque d'homonyme : à trancher.
2. Une confirmation **en lot** des cas forts sans contradiction est-elle acceptable pour le secrétariat (chaque décision tracée à l'unité) ?
3. Code de division SIECLE et dates d'année identiques comme clés fortes **automatiques** (sans personne) : confirmé par la conception pour les référentiels, à confirmer.
4. Droit `reprise.decider` : direction et secrétariat, **pas l'Admin** de l'établissement : à confirmer.
5. Durée de conservation de la catégorie `mes.reprise` (12 mois posés, **à valider par le DPO**).
6. Événement `reprise.decision` et journal de consultation de la file : à brancher avec l'écran.
