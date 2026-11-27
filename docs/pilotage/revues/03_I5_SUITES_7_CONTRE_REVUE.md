# Contre-revue — I5 suites passe 7 (PR 151, tête 6360035d) — 2026-11-24

Relecteur `review-sonnet`, lecture seule. **Aucun élevé, aucun trou de verrou.** Verdict : versable après correctifs mineurs.

## Conforme
- Moyen D (318) : formule d'offre identique à 316 ; offre NULL, deux offres (triées, sans interblocage), UPDATE (anciennes ET nouvelles offres, sur-ensemble de 316), exemption de purge ; `search_path` et `public.` ; 318 en fin de tableau ; aucun déclencheur recréé (pas de fenêtre sans sentinelle).
- Moyen C : restauration sous `#[cfg(test)]` seulement ; l'erreur d'origine est rendue.
- Moyen A : requête de catalogue sûre, ne bloque pas le démarrage, couvre les états O, D, R.
- Repli F6 : aucun nom d'élève, cloisonnement par établissement, accès ouvert.
- Tests discriminants (compteurs d'appels, preuve rouge sur 316), instant injecté.

## Moyens
1. `restaurer_sentinelle` sans nouvelles tentatives 40P01/40001, échec seulement en `tracing::error!` (invisible en test). → **Corrigé par le pilote** (boucle de tentatives, `eprintln!`).
2. La vérification de santé ne voyait pas un déclencheur **absent** ; le test recopiait la requête au lieu d'appeler la fonction. → **Corrigé par le pilote** (`SENTINELLES_ATTENDUES`, `LEFT JOIN`, fonction générique sur l'exécuteur, test sur la transaction avec un déclencheur supprimé).
3. T-3 : `s7_rejeu_en_echec` rejouait le vrai 304 puis `rejouer_tout` (base dans un état ancien pour les tests voisins, mécanisme de l'incident CI tranche 3). → **Corrigé par le pilote** (SQL neutre sous le nom de 304, plus de rejeu complet).
4. Repli F6 : attributions retenues sans le garde-fou « rôle de base personnel » de `droits_effectifs`. → **Corrigé par le pilote** (filtre `is_staff`).

## Faibles (en file)
- Offre « d'avant » lue dans l'état courant du dossier (identique à 316 ; seulement en CTE qui modifie aussi le dossier).
- Restauration non couverte en cas de panique ou d'annulation du futur ; autres objets rejoués non remis en état.
- `ALTER TABLE … ENABLE/DISABLE TRIGGER` dans le test de santé : verrous qui peuvent rendre un test voisin victime (`lock_timeout`).
- Test « famille avec attribution : 0 alerte » à ajouter.
