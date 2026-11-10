# PRA — stockage des objets : suivi du lot `ent-stockage-objets-s3`

Branche `ent-stockage-objets-s3` (base `origin/ENT`). Suite de l'audit PRA (`docs/PRA_SAUVEGARDES.md` de la branche `origin/ent-mc0hoe`, §16.4 et §18.4) et de l'erreur **F-013**.

> **Note de classement.** `docs/PRA_SAUVEGARDES.md` n'existe pas encore sur `origin/ENT` (il vit sur `origin/ent-mc0hoe`). Créer ici un fichier du même nom donnerait un conflit d'ajout au moment de la fusion. Ce suivi est donc dans un fichier séparé ; **à reporter tel quel dans une section « Suivi » de `PRA_SAUVEGARDES.md`** quand ce document atteindra `ENT`.

Niveau : **implémenté et testé en local (PostgreSQL 16) ; non validé par Florian ; non éprouvé contre un vrai bucket Scaleway ou OVHcloud.**

## 1. Faits — ce que faisait `ent-object` avant (F-013)
- Les fichiers étaient écrits **sur le disque local** ; l'envoi S3 (`ENT_OBJECT_S3_HTTP`) était facultatif, **non signé** (PUT HTTP brut), et son échec était ignoré (`let _ = …`).
- Les clés de certains objets étaient **réécrites** (photo d'élève = `{etab}/trombi/{eleve}.jpg`, bulletin = `{periode}.pdf-{eleve}` à chaque génération).
- Aucune empreinte ni taille en base pour les fichiers ; aucune vérification à la lecture ; un échec de vérification d'existence valait « absent ».

## 2. Ce qui change (implémenté)
| Sujet | Après |
|---|---|
| Fournisseur | trait `StorageProvider` (`ent-object/src/provider.rs`) ; implémentations `S3Provider` (production), `DiskProvider` (labo), `MemoryProvider` (tests), `FaultyProvider` (pannes simulées). La logique métier ne voit que `ObjectStore`. |
| Production | `PgEnt::connect` appelle `ObjectStore::from_env_strict(false)` hors labo : **S3 obligatoire**, configuration complète, `https://` ; `disk` et `memory` refusés ; sinon l'API ne démarre pas. |
| Signature | AWS **SigV4 par en-têtes** (module `s3.rs`, vecteur officiel AWS testé) ; région, endpoint, style chemin / hôte virtuel configurables (mêmes variables que les liens pré-signés). |
| Immuabilité | Réécrire une clé avec un contenu différent → `Immutable`. Même contenu → idempotent. Photo et bulletin/livret utilisent une clé **neuve** à chaque version. |
| Empreinte | SHA-256 calculé **avant** l'envoi, envoyé en `x-amz-content-sha256` (le serveur refuse un corps différent) et en métadonnée `x-amz-meta-sha256` ; **relecture des métadonnées après écriture** (taille + empreinte) ; vérification à **chaque lecture** ; comparaison aux valeurs de la base (`ent_objet`). |
| Base | Table `ent_objet` (migration **500**) : `etablissement_id`, `cle_objet`, `taille`, `sha256`, `mime`, `cree_le`, `supprime_le` ; RLS ; clé = préfixe de l'établissement (contrainte) ; **immuable** par déclencheur (pas de `DELETE`, seule `supprime_le` peut passer de NULL à une date). Écrite dans la même transaction que la ligne métier quand il y en a une. |
| Échecs | Jamais ignorés : erreur remontée (HTTP 503 stockage indisponible, 500 intégrité, 409 immuable), `tracing::error!` sans contenu ni secret, compteurs exposés par `/metrics` (`ent_objet_operations{operation,resultat}`). Une panne de vérification d'existence est une **erreur**, pas « absent ». |
| Liens | Inchangé dans le principe (déjà court, ≤ 900 s, émis après contrôle des droits, SigV4 en mode `s3`). En mode local, `GET /fichiers/{jeton}` lit désormais via `lire_objet` (empreinte contrôlée). Aucun lien public. |
| Clés (E14) | `{etab}/{espace}/{uuid}.{ext}` : UUID et mots fixes seulement. Photo : `{etab}/trombi/{eleve}/{uuid}.{ext}` (le contrôle d'appartenance à l'élève est conservé, l'ancienne forme reste lisible). |

## 3. Exigences E1–E14 (PRA §18.4) pour ce lot
| E | État |
|---|---|
| E1 | `ent_objet.etablissement_id NOT NULL`. |
| E2 | Pas de FK vers une ligne d'établissement : la référence est la clé objet, contrainte `cle_objet LIKE etablissement_id/%`. **Écart assumé** : les tables métier gardent `storage_key` (texte) sans FK vers `ent_objet` (reprise des objets historiques sans ligne, voir §5). |
| E3 | `id UUID` généré par l'application. |
| E4 | Table d'établissement, pas globale. |
| E5 | Objet immuable + ligne `cle_objet`, `taille`, `sha256`, `mime`, `cree_le`, `supprime_le`. **Reste** : les deux `BYTEA` (§5) et les objets historiques sans ligne. |
| E6 | `cree_le` ; pas de `modifie_le` (table non modifiable) ; suppression = `supprime_le`. |
| E7 | Journal des accès sensibles inchangé ; pas de nouvelle action sensible. |
| E8 | Migration additive et rejouable (`IF NOT EXISTS`, `CREATE OR REPLACE`, `DROP TRIGGER IF EXISTS`). |
| E9 | Pas de nouvelle file d'effets de bord. |
| E10 | Aucun JSON. |
| E11 | Aucune nouvelle clé de chiffrement ; les identifiants S3 sont des secrets d'environnement (coffre). Le chiffrement serveur (SSE) se règle sur le bucket, pas dans l'application. |
| E12 | **Non fait ici** : le registre de classement des tables (`registre_donnees.rs`) n'existe pas sur `ENT` ; `ent_objet` est à y classer : *portée établissement, catégorie d'export « simple » (métadonnées), pas de neutralisation à la restauration* ; **les octets** sont dans la copie de buckets (D-040). |
| E13 | RLS standard ; le compte de tables RLS de `tests_rls.rs` est passé de 152 à 153. |
| E14 | Aucune donnée d'établissement dans les journaux (jamais le nom de fichier ni la clé dans un message d'erreur client) ni dans les noms d'objets. |

## 4. Choix de la bibliothèque (justifié)
| Option | Licence | Constat | Verdict |
|---|---|---|---|
| `aws-sdk-s3` | Apache-2.0 | Très complet, mais **asynchrone**, des dizaines de crates (`aws-smithy-*`, `aws-config`, `aws-sigv4`…), pas de choix de `ring` simple (aws-lc-rs par défaut) ; surdimensionné pour 3 opérations. | Rejeté (poids, surface d'audit). |
| `rust-s3` | MIT | Maintenue, SigV4 intégré, mais chaîne de dépendances large (features `tokio`/`sync` exclusives), comportements spécifiques par fournisseur. | Rejeté (poids, risques de features). |
| `object_store` | Apache-2.0 | Bonne abstraction multi-nuages (celle d'Arrow), asynchrone, dépendances moyennes ; ajoute `reqwest`/`hyper` et une abstraction de plus au-dessus de la nôtre. | Rejeté (double abstraction ; nous avons notre trait). |
| **`ureq` 3 + SigV4 interne** | MIT/Apache-2.0 | `ureq` : client HTTP **synchrone** (l'API du magasin l'est), rustls + `ring` (déjà dans l'arbre), +36 lignes au `Cargo.lock`. La signature SigV4 existait déjà (liens pré-signés, vecteur officiel AWS) : on l'a étendue aux en-têtes, avec le **vecteur officiel « GET Object »** de la documentation AWS en test. | **Retenu.** |

Réserve honnête : on maintient donc ~100 lignes de signature (testées contre le vecteur AWS). Si les besoins grandissent (envoi par parties au-delà de 5 Go, suppression, versionnage, listage), passer à `aws-sdk-s3` derrière le même trait.

Conséquence : les appels sont **synchrones** et bloquent un thread de l'exécuteur le temps de la requête (délai de connexion 5 s, global 60 s, 3 tentatives). Acceptable aux tailles actuelles (≤ quelques Mo) ; à surveiller avec les métriques ; si besoin, `spawn_blocking` côté appelants.

## 5. Plan de reprise des deux tables en `BYTEA` (non fait : ni simple ni sûr dans ce lot)
Tables : `ent_msg_piece.contenu` (pièces de messagerie) et `ent_pai_piece.contenu` (pièces de PAI **historiques** : depuis le lot D2, les nouveaux dépôts PAI vont déjà dans le magasin, `contenu` vide et `storage_key` renseignée).

Ce qui est déjà couvert par ce lot : à l'émission d'un lien, le contenu historique est recopié dans le magasin sous une clé conforme, **vérifié** et **enregistré dans `ent_objet`** ; une erreur de stockage remonte (avant : ignorée).

Plan, en quatre étapes, chacune additive et réversible (E8) :
1. **Migration additive** : `ent_msg_piece.storage_key TEXT`, `objet_id UUID` ; `ent_msg_piece.contenu` devient nullable avec défaut vide (comme `ent_pai_piece` en 059). Les **nouveaux** envois de pièces écrivent dans le magasin (comme le cahier de textes) et ne remplissent plus `contenu`.
2. **Reprise en lots** (outil de maintenance, idempotent, reprenable) : pour chaque pièce avec `contenu` non vide et sans objet : écrire l'objet (clé `{etab}/messages|pai/{piece.id}.{ext}`, immuable), **relire et comparer SHA-256**, enregistrer `ent_objet`, poser `storage_key`. Ne **vide** `contenu` qu'après vérification et après un point de sauvegarde vérifié ; rapport de lots (comptes, volume, échecs) ; reprise sans risque si interrompue.
3. **Observation** : métriques `lectures_sans_empreinte` et nombre de lignes avec `contenu` non vide ; une période d'observation sur le pilote avant de supprimer.
4. **Suppression de la colonne** (migration ultérieure, destructive, **avec décision** et sauvegarde vérifiée, `MIGRATIONS.md`) ; `VACUUM FULL` programmé hors pointe pour rendre l'espace à la base et aux WAL.

Questions pour Florian : (a) volume réel des pièces de messagerie à reprendre (à mesurer sur la base pilote, pas sur la production Next) ; (b) rétention des pièces de messagerie supprimées (D-043, DPO).

## 6. Reste à faire / hors périmètre
- **Aucun essai contre un vrai bucket** (Scaleway ou OVHcloud) : le client est testé contre un faux serveur HTTP local et le vecteur AWS ; l'essai réel est un préalable au pilote. Points à vérifier : `HEAD` d'une clé absente sans droit de listage (403 traité comme « absent » avant écriture, l'écriture tranche), `x-amz-meta-sha256` restitué en `HEAD`/`GET`, comportement de `Content-Length` du `HEAD`.
- Objets historiques écrits avant la migration 500 : pas de ligne `ent_objet` ; lus avec la seule empreinte du fournisseur (disque : fichier `.sha256` absent pour les très anciens, donc non vérifiables ; compteur `lectures_sans_empreinte`). Reprise : tâche de maintenance à écrire avec §5.
- Réplication vers les destinations de sauvegarde A et B (D-040), Object Lock, SSE : hors de ce lot (réglages de bucket et lot `ent-sauvegarde`).
- Les graines du labo (`seed.rs`) écrivent encore avec `let _ = put(…)` : labo seulement, mémoire ou disque, non concerné par la production.
- Registre de classement des tables (E12) : à créer avec le lot PRA ; y inscrire `ent_objet`.
