-- 585 — RLS d'établissement, lot « evaluations_pedagogie » (11 tables). Même politique que le pilote
-- (migration 100, docs/RLS_ISOLATION.md) : FORCE ROW LEVEL SECURITY + politique
-- ent_rls_etablissement (variable app.etablissement_id posée par TenantTx ; sans
-- variable : permissif tant que app.rls_strict n'est pas actif). Idempotent (rejoué à chaque
-- démarrage). Pose via ent_rls_poser (définie en 580).

SELECT ent_rls_poser(ARRAY[
        'ent_cahier_texte',
        'ent_cdt_historique',
        'ent_cdt_piece',
        'ent_cdt_visa',
        'ent_matiere',
        'ent_periode',
        'ent_periode_vacances',
        'ent_seance_contenu',
        'ent_travail',
        'ent_travail_eleve',
        'ent_travail_remise'
]);
