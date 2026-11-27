-- 206 — RLS des tables du lot S2-9, même politique que le pilote (migration 100,
-- docs/RLS_ISOLATION.md). Les dépôts S2-9 écrivent via TenantTx ou posent
-- app.etablissement_id dans la transaction du consommateur. Idempotent.

DO $$
DECLARE
    t text;
    tables text[] := ARRAY[
        'ent_fin_parametres', 'ent_fin_article', 'ent_fin_tarif', 'ent_fin_reduction',
        'ent_fin_reduction_eleve', 'ent_fin_bourse_bareme', 'ent_fin_echeancier_type',
        'ent_fin_compteur', 'ent_fin_compte_payeur', 'ent_fin_prenotification', 'ent_fin_rejet',
        'ent_fin_reglement', 'ent_fin_lettrage', 'ent_fin_palier_relance', 'ent_fin_relance',
        'ent_fin_export', 'ent_fin_export_piece'
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
