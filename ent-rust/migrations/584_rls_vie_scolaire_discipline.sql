-- 584 — RLS d'établissement, lot « vie_scolaire_discipline » (5 tables). Même politique que le pilote
-- (migration 100, docs/RLS_ISOLATION.md) : FORCE ROW LEVEL SECURITY + politique
-- ent_rls_etablissement (variable app.etablissement_id posée par TenantTx ; sans
-- variable : permissif tant que app.rls_strict n'est pas actif). Idempotent (rejoué à chaque
-- démarrage). Pose via ent_rls_poser (définie en 580).

SELECT ent_rls_poser(ARRAY[
        'ent_edition_vs',
        'ent_eleve_interdiction',
        'ent_procedure_disciplinaire',
        'ent_punition',
        'ent_punition_type'
]);
