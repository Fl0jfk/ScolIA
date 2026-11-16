# Lot R2 (MS4 c) — zone de reprise, décodage strict, correspondance, simulation

Statut : **implémenté et testé en local (PostgreSQL 16)** sur l'export **fictif** au format Next.js (R0) ; CI : une seule, lancée en fin de lot (SHA et numéro de run transmis au pilote). **Jamais exécuté sur un export réel** (celui de Florian, anonymisé par R1, viendra plus tard) : rien ici ne prouve « prêt pilote » ni « prêt production ».
Branche `ent-socle-28-code-r2`, base `origin/ENT`. Aucune base de production, aucun secret, rien du Next.js touché.

## 1. Ce que livre le lot

| Brique | Où | Rôle |
|---|---|---|
| Décodage **strict** | `ent-reprise/src/decodage/` | lit le dump et les contenus S3 ; tout champ inconnu, type inattendu ou valeur hors domaine est une erreur **rapportée** |
| Périmètre | `donnees/perimetre_reprise.tsv`, `perimetre.rs` | que fait-on de chacune des 190 tables et des familles de collections (reprise, rapprochement, archive, ignorée, santé refusée) |
| Correspondance | `correspondance.rs`, migration 2565, `ent-db/src/reprise.rs` | identifiant Next.js → identifiant ScolIA, **dérivé** (SHA-256), stable, rejouable |
| Simulation | `simulation.rs` | rejoue la décision de reprise sans rien écrire dans le métier ; rapport créé / mis à jour / inchangé / archivé / ignoré / refusé / à décider |
| Zone de reprise | migration 2566 (générée), `zone.rs`, `ent-db/src/reprise_zone.rs` | tables de transit `reprise.*`, chargement, relecture de contrôle, purge |
| Trace | migration 2564, `reprise.rs` | exécution numérotée, comptes (invariant en `CHECK`), refus, lignes à décider |
| Commandes | `ent-reprise decoder`, `simuler`, `zone-ddl`, `jeu-fictif --variante decodable` | hors base (la simulation contre la base passe par `ent-db`) |

## 2. Décodage strict — ce qui est compris, rejeté, et pourquoi

Chaque ligne d'une table du dump donne : **comprise** (chargée en zone), **refusée** (code + colonne + message sans valeur), ou **ignorée** (raison). Invariant par objet : `lues = comprises + refusées + ignorées`. Les comptes sont donnés **par table et par collection** ; le détail des refus est borné à 50 lignes par objet (les comptes par code restent exacts), sans aucune valeur de donnée.

Codes de refus (stables) :
- cellule : `null_interdit`, `uuid_invalide`, `uuid_nul`, `instant_invalide`, `instant_hors_domaine` (années 1900–2100), `jour_invalide`, `jour_hors_domaine`, `entier_invalide`, `booleen_invalide`, `decimal_invalide`, `reel_invalide`, `json_invalide`, `tableau_invalide`, `element_nul`, `texte_trop_long`, `type_sql_inconnu` ;
- JSON des colonnes `jsonb` (formes déclarées d'après `db/*.ts`) : `json_type`, `json_cle_inconnue`, `json_cle_manquante`, `json_enum`, `json_profondeur`, `json_taille`, `json_non_catalogue` ;
- documents aplatis (`*_attr`, `ent_collection_attr`) : `attr_prefixe` (préfixes `s:` `n:` `b:` `j:` `null:null` `undef:`), `attr_nombre`, `attr_booleen`, `attr_json`, `attr_chemin`, `attr_len`, `attr_feuille_et_enfants`, `attr_racine_et_enfants` (le document entier est alors refusé) ;
- clés : `cle_absente`, `cle_dupliquee` ; schéma : `schema_ecart` (table en écart : colonne inconnue ou manquante, table inconnue ; les lignes ne sont pas décodées) ; `ligne_mal_formee` ;
- collections : `collection_inconnue` (R-01 : bloque la reprise) ;
- **santé** : `sante_hds` — les 7 tables de santé et `settings__bien-etre` sont refusées **sans que leurs cellules soient lues** (R-MSE5) ;
- S3 : `s3_secret` (préfixe `tenants/`, `secrets/`, clés privées : refusé et jamais lu), `s3_version`, `s3_pieces_incoherentes`, `s3_cle_invalide` (`..`), `s3_contenu_absent`, `s3_taille`.

**Ignorées, avec raison** (non chargées, comptées) : `technique`, `secret`, `jeton`, `etat_interface`, `perdable`, `plateforme`, `file_de_traitement`, `journal_import_next`, `jeton_expire` (dépôts S3 en attente, au-delà de leur durée de 72 h : l'instant de référence est **fourni** par `--maintenant`, jamais lu de l'horloge), `inventorie_non_repris`, `contenu_non_fourni`.

**Colonnes secrètes** (les 10 colonnes « effacer » de la liste blanche R1 : mots de passe, jetons OAuth, secret 2FA, clés push…) : jamais chargées en zone, comptées dans `secrets_ecartes`.

### Colonnes JSON : ce qui est typé, ce qui reste à cataloguer
- **Typées** (formes fermées d'après les types TypeScript) : `fd_campagne.*`, `fd_fiche.*`, `fd_reponse.payload`, `partenariat_offre.*`, `famille_messaging_settings.roles_can_initiate`, `eleve.options`, `sst_fiche.workflow`, `portes_ouvertes_slot_staff.meta`.
- **Instantanés d'audit** (JSON borné, archivé tel quel, exception D-036) : les trois `*_audit.metadata`, `metier_event.payload`.
- **À cataloguer** (le Next.js ne type pas leur contenu : `Record<string, unknown>`) : `personnel.payload`, `preinscription.payload`, `eleve_scolarite.grille_repas`, `ref_nomenclature.metadata_json`. Leur forme est **fermée et vide** : tout contenu non vide est **refusé** et les **noms de clés** sont listés (`colonnes_a_cataloguer`). C'est volontaire (R-17 de T0) : la première exécution sur l'export réel anonymisé produira la liste des clés à ajouter au catalogue ; elles ne sont jamais « rangées en vrac ». **Conséquence à connaître** : sur l'export réel, beaucoup de lignes `personnel` seront refusées tant que ce catalogue n'est pas complété.

### JSON S3
Décodés strictement : `rentree/submissions/pending/<jeton>/meta.json` et `requests/pending/<jeton>/meta.json` (les deux seuls JSON S3 que le code du Next.js lit directement, T0 §2.4). Les autres objets sont classés : secret (refusé), base de connaissances (ignorée), autre JSON (inventorié, non repris), fichier (copié par le lot fichiers R7, ignoré ici). La **référence** d'un document est l'empreinte de sa clé : la clé contient un jeton d'accès, jamais conservé ni cité.

## 3. Zone de reprise (migration 2566, générée)

- Schéma `reprise`, **190 − non chargées** tables `reprise.next_<table>` (une par table source chargée : disposition reprise, rapprochement ou archive, soit 157 tables) + 6 tables S3 typées. Colonnes de données en **texte brut**, une par champ attendu, plus les colonnes de transit `_execution_id`, `_etablissement_cible`, `_source_ligne`, `_cle_source`, `_empreinte`, `_objet`, `_disposition` (et `_type_feuille` pour les tables clé-valeur).
- **Séparée du métier** : schéma distinct, aucune clé étrangère, aucune RLS métier, aucune table dans `public`.
- **Invisible** de tout utilisateur de l'ENT : `REVOKE ALL … FROM PUBLIC`, aucun droit pour le rôle de l'application ; seul `ent_repris` (`ent_reprise`, `NOLOGIN`) y accède. Testé : un rôle quelconque reçoit « permission denied » (42501), `PUBLIC` n'a aucun droit.
- **Purgée** : `vider_zone(execution)` supprime toutes les lignes de l'exécution ; la date est notée (`rep_execution.zone_videe_le`).
- Chargée **après** le décodage, relue et comparée (nombre de lignes par table) avant la simulation ; une zone qui ne relit pas ce qu'on y a mis fait échouer l'exécution.
- Exception « instantané d'audit » de D-036 déclarée à `docs/socles/MIGRATIONS.md` (colonnes JSON du Next.js en texte brut le temps de la vérification).

## 4. Correspondance

`id_derive(établissement cible, source, objet source, identifiant source)` = SHA-256 → UUID version 8. **Jamais aléatoire** : identique à chaque exécution et sur toute machine ; deux établissements cibles ne partagent aucun identifiant ; les frontières de champs comptent (`("ab","c")` ≠ `("a","bc")`). Une correspondance déjà enregistrée (décision humaine de R3) **prime** sur la dérivation. `rep_correspondance` : unique par `(établissement, source, objet, identifiant)` ; l'enregistrement est **idempotent** et ne change jamais un `cible_id` déjà posé ; il est **interdit pour une simulation**.

## 5. Simulation

Fonction pure (`simuler`) : entrée = décodage + correspondances enregistrées ; sortie = rapport déterministe. Règles :
- `reprise` : sans correspondance → **créée** ; avec correspondance et même empreinte → **inchangée** ; empreinte différente → **mise à jour** ; objet **« modifié depuis la reprise » → à décider** (jamais écrasé) ;
- `rapprochement` : **jamais de fiche du cœur sans décision** : sans correspondance → **à décider** (file `rep_execution_a_decider`, préparée pour R3) ;
- `archive` → archivée ; santé → refusée ; ignorées → ignorées avec raison.

Invariant par objet **et en total**, vérifié par les tests et par une contrainte `CHECK` de la base : `lues = créées + mises à jour + inchangées + archivées + ignorées + refusées + à décider`.
**Aucune écriture dans le métier** : testé contre la base par comptage de toutes les tables hors `rep_*` avant et après (identique), et `rep_correspondance` reste vide après une simulation. La simulation n'écrit que la **trace** de l'exécution (`rep_execution*`, `rep_refus`) et la zone de transit.
**Rejouable / idempotent** : deux simulations identiques ont la même empreinte de rapport ; après enregistrement des correspondances, une nouvelle simulation donne 0 création, 0 mise à jour, `inchangées` = anciennes créations, et le « à décider » reste à décider.

## 4bis. Ce que le lot ne fait pas (et pourquoi)
- Il **ne crée aucun objet métier** ni aucune fiche du cœur : R3 (rapprochement, file de décisions) et les lots de module (R5 à R9).
- Il ne contient **aucune commande qui se connecte** : `ent-reprise` reste sans dépendance réseau ni base (garde R0 intacte) ; le chargement en zone et la simulation contre la base sont des fonctions d'`ent-db` (`reprise_zone::executer_simulation`), à brancher sur l'outil d'exploitation de MS4 (opérateur, deux personnes pour l'application, R-MSE11).
- Aucune **donnée de santé** : refusée, jamais lue (R-MSE5).
- Les décisions humaines (`rep_decision`), les synonymes, les échantillons, le tableau de bascule : lots suivants.
