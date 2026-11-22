# Socle 11 — lot MB0 « Correctifs de sécurité » : livraison

Branche `ent-socle-11-code-mb0` (base `origin/ENT`). Conception : `11_APPLICATIONS_MOBILES.md` §9 (MB0), `11_MOBILE.md` §27.1 (MO0). Statut : **implémenté et testé en local** (serveur) ; **pas encore validé** (revue `review-sonnet` et revue sécurité MASVS à faire) ; pas prêt pilote.

## Fait (serveur, Linux)
| Réf. | Correctif | Où |
|---|---|---|
| **DS2 / MEN-11** | La durée longue (mobile) ne dépend plus de la déclaration du client : `client_effectif(déclaré, assurance)` ne retient « mobile » que si l'assurance est **passkey** ; sinon durée **web** (12 h, 2 h d'inactivité). Appliqué à l'émission (`issue_session`) et à la prolongation glissante. En attendant la clé d'appareil de MB4, la passkey est la seule preuve (R-MO3). Les sessions « mobile » déjà ouvertes sans passkey sont ramenées à la durée web par la migration. | `auth_repo.rs`, migration 2220 |
| **DS1** | `ent_push_device.session_id` ; un déclencheur SQL révoque, **dans la même transaction**, les appareils d'une session révoquée (déconnexion, appareil perdu, accès fermé, révocation globale : tous les chemins). | 2220 |
| **DS3** | `POST /devices/push` ne renvoie plus le jeton ; empreinte SHA-256 (`token_empreinte`) ; un jeton déjà actif pour une autre personne/session est **révoqué avec trace** (`revoque_motif = reaffecte`), jamais réaffecté en silence ; l'événement métier porte `reaffecte_depuis` (sans le jeton). | `push_repo.rs`, `service.rs`, `routes.rs`, 2220 |
| **DS4** | `DELETE /devices/push/{id}` (propriétaire seul, 404 sinon) ; la déconnexion coupe l'inscription par DS1. | `routes.rs`, `service.rs` |
| **DS5** | `ent_push_device` : hors portabilité, **purgé** à la restauration (registre). | `registre_donnees.rs` |
| — | Les inscriptions sans session ne sont plus utilisées pour l'envoi après 30 jours sans réinscription (requêtes d'envoi et de disponibilité du canal). | `notifications.rs`, `ntf/destinataires.rs`, `campagnes.rs`, `push_repo.rs` |

Migration **2220** (plage 2220–2239 ; 2221–2239 libres). Aucune table nouvelle ; aucune migration versée modifiée. Tests existants adaptés : deux `INSERT … ON CONFLICT (etablissement_id, token)` deviennent `… WHERE revoked_at IS NULL` (index d'unicité désormais partiel).

## Non fait (et pourquoi)
- **Chiffrement du jeton** (`token_chiffre`, `enc:v2`) : aucune brique de chiffrement (E11) n'existe dans le dépôt ; le jeton reste en clair en base (jamais renvoyé, exclu de l'export, purgé à la restauration). À reprendre quand la brique existera (socle 8, N7).
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
