-- 215 — RLS pilote (socle 1, C4) étendue aux tables du lot S2-11 : même politique que
-- la migration 100 (établissement posé par TenantTx). Idempotent.
-- Prérequis : `ent_rls_etablissement()` / `ent_rls_strict()` créées par la 100.

DO $$
DECLARE
    t text;
    tables text[] := ARRAY[
        'ent_pai_signature', 'ent_contrainte_publiee', 'ent_amenagement',
        'ent_aesh_affectation', 'ent_equipe_reunion', 'ent_sante_protocole'
    ];
BEGIN
    FOREACH t IN ARRAY tables LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('DROP POLICY IF EXISTS ent_rls_etablissement ON %I', t);
        EXECUTE format(
            'CREATE POLICY ent_rls_etablissement ON %I FOR ALL '
            'USING (etablissement_id = (SELECT ent_rls_etablissement()) '
            '  OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict()))) '
            'WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement()) '
            '  OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))',
            t);
    END LOOP;
END
$$;
