# Contre-revue des suites F3, 2 (PR 146, tête 7a4fd476) — 2026-11-24

Relecteur `review-sonnet` sur worktree, lecture seule. **Aucun Élevé.** Garde-fou anti double facturation **intact** (clés
stables, ancienne ligne retrouvée au rejeu) ; cache par jour de vigueur correct ; alerte du second regard conforme
(`parametrer`/`voter`, ni e-mail ni date en dur).

## Moyens
1. **Régularisation de régime silencieuse** : `facturer_changement_regime` ignorait `Ok(Err(motif))` → un trou de tarif
   d'article laissait la famille non facturée sans alerte. → **Corrigé en partie par le pilote** : journal `warn!` (établissement,
   événement, motif). **En file** : tâche d'anomalie dédiée (nature nouvelle dans `ent_fin_tache`, migration 532).
2. **Introduction d'un article en cours d'année bloquée** (trou en tête) et contournement « désactiver l'article » incorrect
   (sous-facturation des mois dus). → **Décision D-069 et correctif du pilote** : seul un trou qui **suit** un mois tarifé
   bloque ; un trou en tête est facturé à partir du premier mois tarifé ; un mois sans aucun tarif reste bloquant. Tests
   `f3s2_trou_en_tete_d_un_article_facture_a_partir_du_premier_mois_tarife` et `f3s2_trou_apres_un_mois_tarife_reste_bloquant`.
   Doc corrigée : « désactiver l'article » n'est pas la voie de retrait (en file : fin de tarif datée).

## Faibles (en file)
1. Troncature par tronçon (±1 centime en faveur de la famille) ; réductions par tronçon ; aucun test de montant impair.
2. Signatures de tronçon par vecteur de foyers : un foyer inchangé est coupé si l'autre change → segmenter par (article, foyer).
3. **Prise en charge d'organisme vérifiée au début du tronçon seulement** : une prise qui expire en décembre n'ouvre plus
   `part_non_couverte` pour janvier-juin si l'article est inchangé → contrôler la couverture sur toute la période (prioritaire).
4. Publication d'une version close supplantée refusée en 409 même si déjà publiée : n'appliquer qu'à la première publication.
5. Tests : clé du second tronçon et rejeu non vérifiés ; pas de test changement de régime sur un trou, ni deux foyers.
6. `.is_ok()` de l'alerte avale les erreurs techniques.
