# Socle 16 — lot « suite » : flux M12 (interrogation longue) et pièce sur son propre message différé

Branche `ent-socle-16-code-suite`, créée depuis `origin/ENT` (02659006c, déjà à jour à la fin du lot). **Aucune migration** (la plage 491–497 reste libre ; 3300–3309 non utilisée).
Niveau : **implémenté et testé en local, non validé par Florian, non prêt pilote.** Base PostgreSQL 16 locale neuve `ent_msg_test`, `ENT_EXIGER_BASE=1`.

## Choix du lot
M7 (carnet et cartes) et M8 (élèves) dépendent de plusieurs socles propriétaires et ne sont pas faits ; la consigne attendait M9, M10, M12. Ce lot prend **M12 (partie serveur)**, la plus autonome, et la suite connue (pièce sur message différé). **M7, M8, M9, M10, M11 restent à faire.**

## Fait
| Sujet | Où | Test |
|---|---|---|
| **Pièce sur son propre message différé** : l'**auteur** peut joindre une pièce avant la remise (elle naît en quarantaine, suit le message, n'est visible de personne d'autre : lecture des pièces déjà filtrée par `auteur = lecteur` ou remise passée) ; un autre participant reste refusé (409) ; retiré : toujours refusé ; limite de 5 toujours sous verrou | `msg_repo.rs::insert_piece` (relit l'auteur sous `FOR UPDATE`) | `corr_f1_…` mis à jour (autre auteur refusé, aucune ligne ; auteur accepté) |
| **M12 flux « quelque chose a changé »** : `GET /messages/flux?apres=<curseur>&attente=<s ≤ 25>` ; **aucun contenu** (identifiant du fil et instant) ; vue `ent_msg_fil_lisible` relue à **chaque** interrogation : aucun événement pour un non-participant, plus rien dès qu'un participant est retiré ; message différé muet avant sa remise ; curseur = heure de la base − 5 s (un message validé en retard n'est pas perdu, un doublon est inoffensif) ; même route pour le web et l'appli unique (parité des droits, D-003) | `msg_repo.rs::evenements_depuis` (classée « filtre intégré » dans la garde), `msg_m12.rs`, route `flux_messagerie` | `m12_flux_participants_seulement_sans_contenu_et_retrait`, `m12_flux_message_differe_invisible_avant_remise` |

## Non fait / écarts
1. **Pas de SSE** : l'interrogation longue ne demande aucune dépendance. Un flux SSE exige `tokio-stream` ou `futures-util` (déjà dans `Cargo.lock`, mais nouvel arc pour `ent-api`) : **à valider avant**. Le contrat (route, curseur, payload) reste valable.
2. Pas d'attente active côté base (pas de `LISTEN/NOTIFY`) : relecture toutes les 2 s par appel en attente ; à revoir si la charge l'exige.
3. Pas de test HTTP de la route (les tests couvrent le service) ; brouillons hors ligne et dictée (M12) non faits ; aucun écran Leptos.
4. M7, M8, M9, M10, M11, M13 à M17 : non faits.

## Vérifications
- ent-db (labo, base démarrée) : 199 tests de messagerie / gardes / isolation / corrections réussis, 0 échec ; `isolation_sql_statique` et `jour_utc_interdit` verts. Un premier lancement de `corr_f1` a échoué une fois (première migration de la base neuve, rejoué vert) ; à surveiller.
- clippy 1.99 `-D warnings`, `ent-db` et `ent-api` (labo, `--all-targets`) : 0 avertissement. Tous les `--locked`. Pas de `cargo fmt --all` (rustfmt non installé pour 1.99 ici, fichiers `ent-db` hors périmètre de formatage). Web non touché.
- Aucun test à l'heure réelle ni au jour UTC (instants lus dans la base).

## Compteurs (règle 28)
Tables avec RLS : **+0** ; `Action::TOUTES` inchangé ; routes **+1** (`/messages/flux`) ; `MIGRATION_TAG` inchangé.

## Correctif CI n° 479 (suite)
- `corr_f1_…` paniquait sur `Domain(MsgPiecePasAuteur)` : **pas une régression du code**. Le domaine (`plan_attacher_piece`) refuse tout non-auteur *avant* le dépôt ; le test attendait un conflit (repli du dépôt, seconde barrière). Le test accepte désormais `MsgPiecePasAuteur` ou le conflit pour « un autre que l'auteur » (aucune ligne créée), et garde : cinq pièces au plus sous verrou, refus sur message **retiré**, auteur accepté sur son message différé. Aucune règle de sécurité affaiblie.
- Le correctif a d'abord manqué l'import de `DomainError` (vu à la compilation locale, jamais poussé vert).
- Local (base neuve `ent_msg_test2`, `ENT_EXIGER_BASE=1`) : 124 tests (messagerie, `corr_`, M12, `isolation_sql_statique`, `jour_utc_interdit`) verts, 0 échec. Fusion d'`origin/ENT` faite (ebac151) ; compteurs inchangés (RLS +0, routes +1, migrations 0).
- Toujours non fait : SSE, LISTEN/NOTIFY, test HTTP du flux, brouillons hors ligne, dictée, M7–M11, M13–M17. Compilation de `ent-db` : tuée par manque de mémoire avec les réglages par défaut ici ; passée avec `CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_CODEGEN_UNITS=64`.
