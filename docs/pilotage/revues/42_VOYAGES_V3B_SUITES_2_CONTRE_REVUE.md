# Contre-revue — voyages V3b suites 2 (PR 153, tête 6d159ab6) — 2026-11-25

Relecteur `review-sonnet`, lecture seule, rien exécuté. **Aucun élevé.** Verdict : après correctifs.

## Conforme
- 657 ne régresse aucune garde de 656 (habilitation par accès ouvert, acompte ni payé ni annulé à la création, portée, expiration 30 jours, usage unique) ; `search_path` et `public.` ; 657 en fin de tableau.
- `ent_sortie_paye_net` reproduit `verses_cash` ; garde du participant, du payeur de la ligne et de `decide_par_user_id` dans la branche collective.
- Révocation : autrice seule, ajout seul, refusée après usage ; une autorisation révoquée ne sert plus ; route sous `verrou_sortie`, cloisonnée.
- Tests M3 probants (expiration par déclencheurs dans une transaction, horodatages 42501, `portee_chk`, autre établissement réel, suspension de l'autrice, révocation).

## Moyens
- **M-1** : la borne collective ne comptait pas les avoirs déjà proposés (cumul possible au-delà du versé ; le test l'attendait comme un succès). → **Corrigé par le pilote** : refus d'un second avoir actif (proposé, retenu, à revoir) pour le même (participant, compte), même règle que l'application ; test corrigé (cumul refusé, gardes de personne sur un second payeur).
- **M-2** : course révocation / usage en SQL direct (pas de verrou dans les déclencheurs). → **En partie** : le déclencheur d'avoir collectif prend le verrou de la sortie (clé de `verrou_sortie`) ; les déclencheurs de faits : en file.
- **M-3** : un contournement « annulation collective » utilisé couvre toutes les propositions collectives de la sortie pendant 30 jours et ne se révoque plus. → **Décision D-069 du pilote** (déjà actée aux correctifs V3b) : la portée collective est la sortie entière ; chaque couple n'a qu'un avoir actif (M-1). Révocation d'un collectif utilisé pour les écritures à venir : en file.
- **M-4** : le test de rattrapage écartait la contrainte en autocommit et rejouait 656 puis 657 sur la base commune (fonctions d'avant 657 un instant sous les tests voisins). → **Corrigé par le pilote** : seule la contrainte est rétablie, plus aucun rejeu. Le précédent (test de 656, ligne ~3818) : en file.

## Faibles (en file)
F-1 test « contournement ne débloque pas la décision par un parent » ; F-2 portée affichée (compte de la demandeuse) ; F-3 et F-5 messages SQL non mappés en refus métier ; F-4 révocation d'une demande expirée jamais autorisée (403 au lieu de 409) ; F-6 refus de révocation non tracé ; F-7 couverture HTTP (200, 403 non autrice, autre établissement) ; F-8 idempotence du rattrapage sur `annulee_le` et les autres lignes ; frontière exacte des 30 jours.
