# Contre-revue de la messagerie E1 + suites M12 (PR 126, tête ca8ebafd) — revue indépendante du pilote

Contre-revue du lot E1/M1–M4 (copie e1, patch e1.patch). Lecture statique du code, de la migration 491 et des tests : je n'ai rien exécuté. Je n'ai pas testé non plus le comportement de PostgreSQL ; ce qui en dépend est marqué « supposé ».

**Verdict : aucun constat Élevé.** Le cœur du curseur xid8 est correct. Les défauts sont des Moyens : une annulation non atomique, un contournement du limiteur, une exception d'annulation sans search_path.

## Moyen

**M-1. L'annulation n'est pas « tout ou rien » pour l'avis planifié (vérifié par lecture).**
- Fichier : `ent-rust/crates/ent-db/src/service.rs` lignes 18832–18862. La suppression du message est commitée dans `annuler_message_differe`. Ensuite, dans une seconde transaction, `UPDATE ent_notification_envoi SET statut='annule' ... statut='en_attente'`.
- Scénario 1 : un échec ou un crash entre les deux commits laisse l'avis push `en_attente` pour un message supprimé. L'API répond 500, la famille reçoit « Nouveau message » pour un message annulé, et l'auteur croit à une erreur alors que l'annulation a eu lieu. Cela contredit la règle 1 (« jamais d'erreur opaque »).
- Scénario 2 : course à l'échéance. L'annulation commit à T−ε, la seconde transaction passe après T. Le distributeur a déjà pris l'avis (`statut` différent de `en_attente`), donc il part quand même.
- Les tests ne le voient pas : `e1_corriger_puis_annuler…` vérifie « avis annulé » en séquentiel.
- Correctif : passer l'annulation de l'avis dans la transaction de `annuler_message_differe`, avant le DELETE de `ent_msg`. Ne passer à `Refusee` qu'après avoir annulé aussi les avis `en_cours`, ou rendre le distributeur sensible à l'existence du message.
- Le reste (versions, cartes, accès, lectures, accusés, pièces, message) est bien dans une seule transaction. Le `FOR UPDATE` sur `ent_msg` sérialise correctement correction et annulation (verrou FK key-share). Aucune autre FK vers `ent_msg` n'existe, hors `ent_msg_telephone` (migration 486), qui ne concerne pas un différé.

**M-2. Contournement du limiteur et requêtes coûteuses par curseur fabriqué (vérifié par lecture).**
- Le curseur v2 n'est ni signé ni borné : `msg_repo.rs` (decoder) accepte n'importe quel `xid` (u64), `differe` et `emis`.
- `flux_limite.rs` lignes 162–172 : `rappel_libre` dépend seulement du fait que la dernière réponse de la session était `tronque`, pas du curseur envoyé. Un utilisateur qui a plus de 200 événements lisibles (direction, CPE) envoie `v2.0.-.-.0.-.<emis=now>`. La réponse est tronquée, le rappel suivant est donc exempt de l'intervalle et du plafond du compte. Il renvoie le même curseur forgé, en boucle (un seul appel à la fois par session subsiste).
- Chaque appel relit tout l'historique lisible depuis xid 0 et depuis l'époque 1970 pour les différés, avec agrégation, sans qu'aucun index d'`ent_msg` n'aide. C'est une amplification de charge authentifiée, que la revue précédente avait évitée par « c'est le serveur qui s'en souvient ».
- Il n'y a pas de fuite de contenu : `ent_msg_fil_lisible` et le jugement de lecture restent appliqués indépendamment du curseur, et la `TenantTx` cloisonne. Un curseur forgé ne donne donc que des identifiants de fils déjà lisibles.
- `emis` forgé (futur ou récent) neutralise la resynchronisation à 600 s. Sans gravité en soi.
- Correctif :
  - signer le curseur (HMAC avec une clé serveur, incluant utilisateur et établissement) ; c'est le plus simple et ça couvre `emis`, ou
  - à défaut, refuser `xid` hors [xmin − marge, xmin], `differe` dans le futur ou hors fenêtre, et `emis` > maintenant ;
  - et n'accorder `rappel_libre` que si le curseur reçu est celui rendu à la session (en mémoire serveur).

**M-3. Fonction de garde sans `search_path` fixé (vérifié par lecture ; l'exploitation est supposée).**
- `491_msg_annulation_versions_flux_xid.sql` lignes 15–38 : `LANGUAGE plpgsql` sans `SET search_path`. Les références `ent_msg` et `ent_msg_piece` ne sont pas qualifiées. `pg_temp` est cherché en premier pour les relations.
- L'exception repose sur `current_setting('ent.msg_annulation')`, un GUC libre que n'importe quelle session du rôle applicatif peut poser. Elle est bornée par `v_msg::text = v_cible` et par `remise_prevue_le > now()`.
- Un appelant capable d'exécuter du SQL (une injection, un bug futur) peut créer `CREATE TEMP TABLE ent_msg(etablissement_id, id, remise_prevue_le)` avec une ligne future. Le `EXISTS` est alors satisfait pour un message déjà remis, ce qui permet de supprimer cartes, accès au journal ou versions de ce message. Cela suppose le privilège TEMP de PUBLIC (par défaut) et l'exécution de SQL arbitraire : Faible à Moyen en probabilité, mais le correctif est de deux lignes.
- La fonction n'est pas SECURITY DEFINER : la RLS de l'appelant s'applique à son `EXISTS`, donc pas de franchissement d'établissement. C'est correct.
- Correctif :
  - `SET search_path = pg_catalog, public` sur la fonction (ou qualifier toutes les tables et écrire `pg_temp` en dernier) ;
  - `REVOKE TEMP ON DATABASE … FROM PUBLIC` ;
  - idéalement, un jeton lié à la transaction plutôt qu'un GUC, par exemple `pg_current_xact_id()` posé dans une table de garde par la fonction d'annulation.
- Le périmètre est « un message jamais remis » (même établissement via RLS), pas « l'auteur dans sa transaction ». Si on veut cela, il faut une fonction SECURITY DEFINER d'annulation avec son propre search_path.

## Faible

**F-1. Reprise après dump/restore, clone ou PITR logique (supposé).**
- `msg_xid` est stocké en valeur absolue de cluster. Si une base est restaurée dans un cluster dont le compteur xid est plus bas, les lignes existantes ont `msg_xid` supérieur à `xmin`, et `position.xid = xmin.max(apres.xid)` garde le plus grand.
- Les nouveaux messages obtiennent un xid inférieur au curseur et ne sont jamais servis (saut silencieux) pendant au plus la fenêtre de 600 s, avant la resynchronisation. Les anciennes lignes sortent comme événements « nouveau » parasites quand `xmin` les dépasse.
- Un `pg_upgrade` ou une restauration physique conservent les xid : pas de problème. À documenter, ou à lier le curseur à `pg_control_system()` / identifiant du cluster.

**F-2. Test de la transaction longue (vérifié).**
- `tests_messagerie_revue.rs` (patch ≈ 2010–2105) : `tronquees` n'est qu'affiché dans le message d'assertion, jamais asserté > 0. Le titre « avec troncature » n'est donc pas prouvé, car la troncature dépend de `reglages(1,1)` et des autres fils.
- La borne `borne_xid` est lue après coup sur une autre connexion ; l'assertion `p1.xid <= borne_xid` est donc plus faible que « le curseur reste sous la transaction ouverte ».
- Correctif : asserter `tronquees >= 1` et comparer à `pg_current_xact_id` de la transaction ouverte, lu à l'intérieur.

**F-3. Test de garde bornée (vérifié).**
- `e1_corriger_deux_fois_puis_annuler`, ligne 2641 du patch : `assert!(r.is_err())` n'importe quelle erreur convient (connexion, droit). Il faut vérifier le code `23514` et le texte « ajout seul ».
- Aucun test ne prouve que le réglage sur le message A ne permet pas de supprimer les versions du message B encore planifié, ni la protection inter-établissement.
- Le test `e1_si_la_base_refuse…` crée une table dans la base partagée : nettoyage manquant en cas de panique avant le `DROP`.

**F-4. M3 : cas « candidats écartés par le jugement de lecture » non testé (vérifié).**
- C'était le défaut décrit (rappel immédiat répété). Le code est correct (`depuis = r.position`), mais `revue_m3_…` ne l'exerce pas : l'événement servi est toujours accepté. Ajouter un fil lisible en SQL mais refusé par `actor_can_access_fil`.
- Le test mélange une mesure d'horloge réelle (`t0.elapsed() >= 5 × 20 ms`) et un rapport de requêtes ≤ 6×. C'est tolérable ; un test de charge CI chargée pourrait être instable côté minuterie, mais la borne inférieure est robuste.
- `flux_http_appel_patient_aboutit_avec_le_temps_injecte` : seuil `< 20 s` en `Instant` réel. Il discrimine bien 25 s contre 25 ms, acceptable.

**F-5. Migration (vérifié).**
- `ADD COLUMN … NOT NULL DEFAULT pg_current_xact_id()` est volatile : réécriture complète de `ent_msg` sous verrou exclusif. Acceptable en labo, à planifier en pilote.
- Les index `(etablissement_id, msg_xid)` ne sont probablement pas utilisés par le plan (jointure depuis `ent_msg_fil_lisible`) : à confirmer avec EXPLAIN.
- Une transaction préparée (2PC) orpheline, ou un slot de réplication / `idle in transaction` avec xid, gèle `xmin` pour tous les établissements. C'est documenté dans la livraison (limite 1) ; une alerte est nécessaire avant le pilote.

**F-6. À vérifier, non confirmé (supposé).**
- Après annulation, le fil reste, avec ses participants et son sujet, même si le message annulé était son premier message (`insert_message_remise` ajoute le participant). Si la liste des fils d'une famille ne filtre pas les fils sans message remis, le sujet d'un message annulé reste visible. Je n'ai pas lu le filtre de liste côté famille.
- `DELETE FROM ent_msg_lecture WHERE lu_jusqua_message_id = $2` supprime aussi le marqueur de lecture de l'auteur : effet probablement bénin, déjà présent en 490.

## Points vérifiés corrects (bref)

1. **Curseur xid8.**
   - La borne `pg_snapshot_xmin(pg_current_snapshot())` est exacte : toute transaction de xid inférieur est terminée, donc un événement validé après le passage du curseur porte un xid ≥ xmin et est servi plus tard.
   - La comparaison est correcte : `xid >= apres` et `< xmin`, plus le tuple strict `(xid, fil, genre) >`. Rattrapé : `(xmin, nil, "")`. Pas de répétition : la position part d'`xmin`.
   - Pas de wraparound (xid8 sur 64 bits, parsé en u64).
   - Les retraits passent par `retire_xid`, posé par un déclencheur `BEFORE UPDATE OF retire_le`.
   - Un retrait d'un message déjà vu est servi ; un retrait avant échéance n'est pas annoncé.
   - Les lignes existantes (xid de la migration, antérieur à tout `xmin` futur) ne sont pas relues, ce qui est voulu.
   - Les différés ne dépendent pas d'xid (échéance) : limite 3 de la livraison (transaction ouverte au-delà de l'échéance) jugée acceptable. Les échéances identiques en plage calme sont servies ensemble dans une même requête.
   - Les corrections (`ent_msg_version`) ne génèrent pas d'événement : comportement inchangé, hors périmètre.
2. **Cloisonnement.** Il passe par `TenantTx` (RLS) et `ent_msg_fil_lisible`, puis par le jugement de lecture, indépendamment du curseur. `pg_stat_activity` et `pg_read_all_stats` sont bien supprimés.
3. **Exception d'annulation (limites).**
   - Seul DELETE passe : UPDATE reste refusé.
   - `v_msg::text = v_cible` et `remise_prevue_le > now()` bornent à un message, et `remise_prevue_le NULL` donne faux.
   - `Refusee` retourne avant `commit`, donc rollback complet.
   - Les codes 23503/23514/42501 sont mappés en erreur métier 409 ; le reste reste une erreur technique (acceptable).
4. **F6** (404 avant 403) correct. **F2** correct au niveau du limiteur pour le plafond du compte, mais voir M-2 pour le contournement par curseur forgé. **F7** : horloge de la base utilisée pour lot et programmation, le reste est listé et justifié.

Fichiers principaux (copie e1, `/tmp/claude-0/-home-user-ScolIA/11355a13-49ee-5b3e-ab9a-53b200277b90/scratchpad/e1/`) :
- `ent-rust/migrations/491_msg_annulation_versions_flux_xid.sql`
- `ent-rust/crates/ent-db/src/msg_repo.rs` (PositionFlux, `evenements_immediats`, `evenements_differes`, `annuler_message_differe`)
- `ent-rust/crates/ent-db/src/service.rs` lignes 18795–18863
- `ent-rust/crates/ent-api/src/flux_limite.rs`
- `ent-rust/crates/ent-db/src/tests_messagerie_revue.rs`

Patch complet : `/tmp/claude-0/-home-user-ScolIA/11355a13-49ee-5b3e-ab9a-53b200277b90/scratchpad/e1.patch`.

