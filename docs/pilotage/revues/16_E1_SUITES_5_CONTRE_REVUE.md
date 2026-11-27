# Contre-revue — E1 suites passe 5 (PR 152, tête 13074f49) — 2026-11-24

Relecteur `review-sonnet`, lecture seule. **Aucun élevé.** Verdict : versable après correctifs mineurs.

## Conforme
- Faible 1 : le report ne fait jamais partir un avis annulé (UPDATE limité à `statut='envoi'`), ne l'avance jamais (`GREATEST`), reste dans l'établissement ; pas de double envoi entre instances (avis en `envoi` non réclamable).
- Faible 2 : preuve par la RLS stricte sous un rôle non propriétaire, sans BYPASSRLS, avec témoin positif ; aucun SQLSTATE vers HTTP ; 42501 et 55P03 distingués.
- Limiteur : clés `&'static str` bornées, verrou empoisonné géré, instant passé par l'appelant.
- Garde statique : `message_existe_sous_verrou_brut` justement classé `PAR_IDENTIFIANT`.

## Moyen
- **M1** : le compteur d'observations était un `AtomicU64` global, incrémenté aussi par toute lecture du flux : `p5_f3` pouvait passer vert sans l'appel (tests parallèles). → **Corrigé par le pilote** : compteur hors flux **par établissement** (tests), le test exige 0 avant et ≥ 1 après sur son établissement neuf ; commentaire corrigé.

## Faibles
1. `warn!` « contrôle d'existence impossible » une fois par avis (500 avis d'un message verrouillé = 500 lignes). → **Corrigé par le pilote** (limité à une ligne par minute). Même constat pour `signaler_echec_etablissement` : en file.
2. 55P03 non mémorisé : chaque avis d'un même message verrouillé attend 5 s (jusqu'à ~7 min de passage, borné) : en file (cache par message).
3. Report calculé depuis le début du passage ; erreur permanente rebouclant sans passer en `echec` : en file (compteur de reports, D-069).
4. Équivalence enveloppe / `_brut` non testée : en file.
5. Limiteur : une même cause masque des erreurs de natures différentes 60 s ; pas de compteur de lignes supprimées : en file.
- Test à ajouter : annulation validée après report → `message_annule`.
