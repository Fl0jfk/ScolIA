# Socle 11 — lot MB0 « Correctifs de sécurité » : livraison

Branche `ent-socle-11-code-mb0` (base `origin/ENT`). Conception : `11_APPLICATIONS_MOBILES.md` §9 (MB0), `11_MOBILE.md` §27.1 (MO0). Statut : **implémenté et testé en local** (serveur) ; **pas encore validé** (revue `review-sonnet` et revue sécurité MASVS à faire) ; pas prêt pilote.

## Fait (serveur, Linux)
| Réf. | Correctif | Où |
|---|---|---|
| **DS2 / MEN-11** | La durée longue (mobile) ne dépend plus de la déclaration du client : `client_effectif(déclaré, assurance)` ne retient « mobile » que si l'assurance est **passkey** ; sinon durée **web** (12 h, 2 h d'inactivité). Appliqué à l'émission (`issue_session`) et à la prolongation glissante. En attendant la clé d'appareil de MB4, la passkey est la seule preuve (R-MO3). Les sessions « mobile » déjà ouvertes sans passkey sont ramenées à la durée web par la migration. | `auth_repo.rs`, migration 2220 |
| **DS1** | `ent_push_device.session_id` ; un déclencheur SQL révoque, **dans la même transaction**, les appareils d'une session révoquée (déconnexion, appareil perdu, accès fermé, révocation globale : tous les chemins). | 2220 |
| **DS3** | `POST /devices/push` ne renvoie plus le jeton ; empreinte SHA-256 (`token_empreinte`) ; un jeton déjà actif pour une autre personne/session est **révoqué avec trace** (`revoque_motif = reaffecte`), jamais réaffecté en silence ; l'événement métier porte `reaffecte_depuis` (sans le jeton). | `push_repo.rs`, `service.rs`, `routes.rs`, 2220 |
| **DS4** | `DELETE /devices/push/{id}` (propriétaire seul, 404 sinon) ; la déconnexion coupe l'inscription par DS1. | `routes.rs`, `service.rs` |
| **DS5** | `ent_push_device` : métadonnées exportées sans jeton, **révoqué** à la restauration (neutralisation `Maj`) (registre). | `registre_donnees.rs` |
| — | Les inscriptions sans session ne sont plus utilisées pour l'envoi après 30 jours sans réinscription (requêtes d'envoi et de disponibilité du canal). | `notifications.rs`, `ntf/destinataires.rs`, `campagnes.rs`, `push_repo.rs` |

Migration **2220** (plage 2220–2239 ; 2221–2239 libres). Aucune table nouvelle ; aucune migration versée modifiée. Tests existants adaptés : deux `INSERT … ON CONFLICT (etablissement_id, token)` deviennent `… WHERE revoked_at IS NULL` (index d'unicité désormais partiel).

## Non fait (et pourquoi)
- **Chiffrement du jeton** (`token_chiffre`, `enc:v2`) : aucune brique de chiffrement (E11) n'existe dans le dépôt ; le jeton reste en clair en base (jamais renvoyé, exclu de l'export, révoqués à la restauration (neutralisation)). À reprendre quand la brique existera (socle 8, N7).
- **Avis de sécurité à l'ancien titulaire** (R-MO17) : seule la trace (`reaffecte_depuis`) est écrite ; l'avis dépend du socle 8 (N7).
- **DA1** (expiration des caches), **DA2** (`targetSdk 36`), **retrait d'OkHttp** (D-062) : correctifs d'appli Kotlin, dans la branche `ent-mobile-app` (absente de `ENT`), non vérifiables dans ce lot. À faire en MB1 avec la CI de l'appli.
- **Enveloppe d'erreur codée** : MB1, pas MB0.
- **Retrait des 4 prototypes** de `ent-rust/apps/` : MB1 (la conception le met en MB1). Aucune régression à craindre : ils ne dépendent que de `/devices/push` (réponse sans `token`, ils ne le lisaient pas).
- **DX1, DX2** (Next.js) : signalés à Florian, non touchés.

## Tests
- `ent-db` `tests_mobile_mb0` (9) : `client_effectif` ; « mobile » + mot de passe = durée web (client stocké = web, inactivité posée) ; web et mobile+passkey inchangés ; prolongation glissante refusée sans passkey ; révocation de session et révocation globale coupent le push ; réaffectation d'un jeton ; désinscription réservée au propriétaire ; registre (non exporté, purgé). **9/9 verts** (`ENT_ENV=test`, `ENT_RLS_MODE=strict`, base neuve « test »).
- `ent-api` `recette_mobile_mb0` (2, séparation HTTP) : `client:"mobile"` + mot de passe → `expires_at` ≤ 13 h et `/auth/me.client = web` ; `POST /devices/push` sans jeton dans la réponse, `DELETE` refusé à un autre acteur / sans jeton, 204 puis 404. **2/2 verts**. `recette_securite_3` (clients web/mobile) non rejouée ici.
- Régression `ent-db --lib` complet (2476 tests) : **64 échecs**, tous dans des familles sans lien avec le push ; rejoués sur base neuve : les groupes communication, internat, RGPD, reprise passent seuls ou à un seul fil (échecs dus à la charge : `statement timeout`, parallélisme), `tests_evaluations_v0`, `tests_administratif_a0bis`, `tests_inscriptions_i3_i4` **échouent aussi sur `origin/ENT` nu** dans cet environnement. `tests_inscriptions::iban_masque_et_nouveau_mandat` échoue seul (`propositions.len() == 0`), sans rapport avec le push mais **non confronté à `origin/ENT`** : à vérifier par la CI.

## Compteurs (règle 28)
0 table ; `tests_rls.rs` inchangé ; registre : `ent_push_device` export → purge (même nombre de lignes) ; `Action::TOUTES` inchangé ; routes : **+1** (`DELETE /devices/push/{id}`) ; `migrate.rs` **+1** (2220) ; `MIGRATION_TAG` inchangé (3808).

## Tête et CI
Voir le message au pilote (tête finale et état de la CI renseignés après le push).

## Correctifs de revue (2026-11-22)
Base : tête distante `159ca4ed`, puis fusion de `origin/ENT` (`ebac1517`, semis de labo relatif ; aucun conflit, compteurs recalculés depuis la base commune `02659006` : `lib.rs` +2 modules de tests côté ENT, +0 de mon côté hors `tests_mobile_mb0` déjà présent ; `MIGRATION_TAG` inchangé). **La migration 2220 n'étant pas versée, elle est modifiée** ; aucune autre migration versée ne change. `url` 2.5.9 appliqué comme la PR 121 (`cargo update -p url --precise 2.5.9` dans `ent-rust` et `ent-rust/web` : `url` et sa dépendance `form_urlencoded` 1.2.3, rien d'autre ; `cargo metadata --locked` de `web` vérifié).

### Règles générales
1. **Règle 1** : une seule fonction SQL, `ent_push_actif(revoked_at, session_id, last_seen_at)` (appuyée sur `ent_session_vivante` : non révoquée, non expirée, inactivité respectée ; sans session : réinscription de moins de 30 jours), appelée par `notifications.rs` (deux chemins), `ntf/destinataires.rs`, `campagnes.rs`, `push_repo.rs`.
2. **Règle 2** : `ent_push_reaffecter` (`SECURITY DEFINER`, verrou consultatif par empreinte, parcours des établissements car la RLS est forcée) révoque (`reaffecte`, trace) tout jeton actif de même empreinte détenu par une autre personne ; la même personne peut l'avoir dans plusieurs établissements.
3. **Règle 3** : le jeton en clair n'existe que sur la ligne active de `ent_push_device`.

### Fait
| Réf. | Correctif | Test |
|---|---|---|
| Compilation | `ent-lab` : `register_push_device_sans_session` (variante explicite labo/tests) ; en production `register_push_device_for_actor` exige `session_id: Uuid` | compilation workspace + labo |
| **E1** | `ent_push_actif` partout ; suppression de session : `ON DELETE SET NULL` + déclencheur `BEFORE DELETE` qui révoque l'appareil (`expire`, trace gardée) | `appareil_d_une_session_expiree…`, `…inactive…` (web, 3 h), `purge_des_sessions_expirees_garde_la_trace…` |
| **E2** | `socle1_sessions_web_inactivite_mobile_longue_et_appareils` : le login mot de passe « mobile » est vérifié **web** ; la partie mobile passe par une vraie session passkey (SoftPasskey, enrôlement puis `/auth/passkey/login/*`), mêmes vérifications qu'avant (inactivité, prolongation > 80 j, liste, déconnexion ciblée) | `ent-api --lib` (158), `recette_securite_3`, `recette_mobile_mb0`, `separation` |
| **M1** | `SELECT … FROM ent_session … FOR SHARE` dans la transaction d'inscription (session vivante **et** de l'acteur) ; absente → 401 | `inscription_avec_une_session_morte_est_refusee` |
| **M2** | règle 2 | `un_jeton_n_a_qu_un_titulaire_entre_etablissements` (deux établissements ; même personne dans deux établissements : rien révoqué) |
| **M3** | même personne, nouvelle session : `ON CONFLICT … DO UPDATE SET session_id` sur la même ligne, sans révocation ; `reaffecte_depuis` = autres personnes seulement | `meme_personne_nouvelle_session_meme_jeton…` |
| **M4** | `ent_push_outbox` : `token_empreinte` + `device_id`, plus de jeton (`CHECK (token IS NULL)`, lignes existantes neutralisées dans 2220) ; l'envoi relit le jeton sur la ligne active (`relire_jeton`) et n'envoie rien si elle est révoquée entre-temps ; registre : export sans `token`/`token_empreinte` | `l_historique_d_envoi_ne_garde_pas_le_jeton` |
| **M5** | `finish_passkey_login` mobile/web (seul chemin à durée mobile) ; SSO forcé web ; `completer_session_apres_enrolement` (durée web sans passkey, y compris ligne forgée « mobile ») ; migration (sessions ramenées, empreintes remplies, rejeu sans effet) | `passkey_mobile_donne_la_duree_mobile…`, `sso_rattachement_par_email…`, `fin_d_enrolement…`, `migration_2220_ramene…` |
| **F2** | `resolve_bearer` applique `client_effectif` à la lecture | même test passkey (ligne forgée lue « web ») |
| **F3** | `LEAST(expires_at, created_at + 12 h)` ; au-delà, session close (appareils révoqués par le déclencheur) | test de migration |
| **F4** | l'empreinte (tenue à jour par déclencheur) est le critère de recherche et d'unicité ; index sur `token` supprimé | tests ci-dessus |
| **F5** | `ON CONFLICT` puis relecture : trois inscriptions simultanées → jamais de 500 ; portabilité : métadonnées de l'appareil exportées sans jeton ni empreinte, révocation de tous les appareils actifs à la restauration | `inscriptions_simultanees…`, `registre_jetons_non_exportes_et_purges` |

### Non fait / risques connus
- **F1** (passkey + client déclaré « mobile » depuis un navigateur) : reste pour **MB4** (clé d'appareil) ; risque connu, réf. MEN-11.
- **Chiffrement du jeton** : toujours hors lot (aucune brique E11) ; **reste à faire du socle 8 (N7)** ; entre-temps le jeton n'est en clair qu'à un seul endroit (règle 3).
- DA1, DA2, retrait d'OkHttp, enveloppe d'erreur, retrait des prototypes, avis de sécurité à l'ancien titulaire : inchangés (voir plus haut).

### Vérifications locales (base fraîche dont le nom contient « test », `CARGO_TARGET_DIR` unique)
- `tests_mobile_mb0` 20/20 ; `ent-api --lib --features labo` 158/158 ; `recette_mobile_mb0` 2/2 ; séparation HTTP 57/57 ; `recette_securite_3` 10/10 (`rs3_02_double_encaissement_sepa…` échoue sous charge, passe seul : instable, sans lien avec le lot). clippy 1.99 : les 3 commandes de la CI (`ent-lab` inclus) sans avertissement ; `cargo metadata --locked` de `web` ok.
- **Régressions trouvées et corrigées pendant la revue** : (1) le registre des données refusait « éphémère + exportable » → la neutralisation de `ent_push_device` devient une mise à jour (`revoked_at = now()`, motif `expire`) au lieu d'une purge ; (2) quatre helpers de test (`tests_communication`, `tests_messagerie_m0a`, …) utilisaient `ON CONFLICT (etablissement_id, token)` → alignés sur `token_empreinte`.
- **« 64 échecs hors périmètre »** : `ent-db --lib --features labo -- --test-threads=4` sur base fraîche = 66 échecs au premier passage (lot compris). Confrontés à `origin/ENT` nu (autre base fraîche) : 41 échouent aussi à nu (sous charge : délais d'instruction, verrous). Les 25 autres : 2 régressions du lot (ci-dessus, corrigées) ; les 23 restants ne passent pas sous charge mais **passent seuls** sur la branche (vérifié sur les groupes `tests_communication`, `tests_edt_s2`, `tests_dossier_personnes`, `tests_eleve_scolarite`, `tests_messagerie_m0a`, `tests_rgpd_g2`, etc.). Aucun échec restant ne vient du lot.
- Compteurs (règle 28) : `MIGRATION_TAG` inchangé, 0 table nouvelle, registre : même nombre de lignes.
