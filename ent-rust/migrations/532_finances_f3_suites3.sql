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
--   * M2 (contre-revue des suites, 3) : `ent_fin_tache.evenement_id` lie la tâche de régularisation de régime à
--     l'**événement** de changement de régime ; l'index d'unicité des tâches ouvertes (522) est recréé avec cette
--     colonne (NULL pour toutes les autres natures : même comportement qu'avant). Deux événements en anomalie pour un
--     même élève donnent deux tâches ; chacune n'est close que par la régularisation de son événement.
-- Aucune fonction n'est créée ni recréée (règle 38 : rien à protéger par `search_path`).

ALTER TABLE public.ent_fin_tache DROP CONSTRAINT IF EXISTS ent_fin_tache_nature_check;
ALTER TABLE public.ent_fin_tache ADD CONSTRAINT ent_fin_tache_nature_check CHECK (nature IN (
    'payeur_manquant', 'repartition_incomplete', 'demande_repartition', 'designation_a_accepter',
    'prise_en_charge_a_renouveler', 'part_non_couverte', 'compte_a_verifier', 'ligne_sans_periode',
    'guichet_quota', 'regularisation_regime_non_facturee'));

ALTER TABLE public.ent_fin_article ADD COLUMN IF NOT EXISTS retire_a_partir_du DATE;

ALTER TABLE public.ent_fin_tache ADD COLUMN IF NOT EXISTS evenement_id UUID;
DROP INDEX IF EXISTS public.ent_fin_tache_ouverte_uq;
CREATE UNIQUE INDEX IF NOT EXISTS ent_fin_tache_ouverte_uq ON public.ent_fin_tache (
    etablissement_id, nature,
    COALESCE(eleve_id, '00000000-0000-0000-0000-000000000000'::uuid),
    COALESCE(compte_id, '00000000-0000-0000-0000-000000000000'::uuid),
    COALESCE(designation_id, '00000000-0000-0000-0000-000000000000'::uuid),
    COALESCE(prise_en_charge_id, '00000000-0000-0000-0000-000000000000'::uuid),
    COALESCE(categorie, ''),
    COALESCE(evenement_id, '00000000-0000-0000-0000-000000000000'::uuid)
) WHERE statut = 'ouverte';
