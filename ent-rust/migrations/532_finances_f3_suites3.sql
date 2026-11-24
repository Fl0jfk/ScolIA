-- 532 — Finances, suites 3 du lot F3 (contre-revue des suites, 2 ; D-069).
--
-- Migrations 520 à 523 et 531 versées : elles ne changent pas (règle 25). Cette migration (fin du tableau,
-- règle 38) :
--   * Moyen 1 : nature de tâche nouvelle `regularisation_regime_non_facturee` dans `ent_fin_tache` (changement de
--     régime que la génération ne peut pas facturer : trou de tarif, payeur manquant…). Même table, même RLS et mêmes
--     droits que les autres tâches (gestionnaires de l'établissement : engagements ou lecture financière) ;
--   * fin de tarif datée : `ent_fin_article.retire_a_partir_du` (premier jour où l'article n'est plus dû ; les mois
--     commencés avant cette date restent dus). Distinct de `actif` : désactiver un article le sort de la génération
--     pour toute l'année (sous-facturation des mois dus).
-- Aucune fonction n'est créée ni recréée (règle 38 : rien à protéger par `search_path`).

ALTER TABLE public.ent_fin_tache DROP CONSTRAINT IF EXISTS ent_fin_tache_nature_check;
ALTER TABLE public.ent_fin_tache ADD CONSTRAINT ent_fin_tache_nature_check CHECK (nature IN (
    'payeur_manquant', 'repartition_incomplete', 'demande_repartition', 'designation_a_accepter',
    'prise_en_charge_a_renouveler', 'part_non_couverte', 'compte_a_verifier', 'ligne_sans_periode',
    'guichet_quota', 'regularisation_regime_non_facturee'));

ALTER TABLE public.ent_fin_article ADD COLUMN IF NOT EXISTS retire_a_partir_du DATE;
