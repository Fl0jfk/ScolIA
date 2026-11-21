-- 100 — Row-Level Security PILOTE (socle 1, C4) : défense en profondeur de
-- l'isolation des établissements. Conception : docs/RLS_ISOLATION.md.
--
-- Additif et sans effet tant que l'application ne pose pas app.etablissement_id
-- (variable de session posée par TenantTx, ent-db/src/tenant_tx.rs) :
--   * variable absente et mode non strict  -> politique permissive (comportement actuel) ;
--   * variable posée                       -> seules les lignes de cet établissement ;
--   * variable absente et mode strict      -> 0 ligne, écriture refusée.
-- Mode strict = app.rls_strict a 'on' : recommande au niveau du role applicatif
-- (ALTER ROLE ... SET app.rls_strict = 'on'), ou par TenantTx si ENT_RLS_MODE=strict.
-- FORCE ROW LEVEL SECURITY : la politique vaut aussi pour le proprietaire des tables.
-- Seuls un superutilisateur ou un role BYPASSRLS y echappent (jamais le role applicatif).
-- Idempotent (rejoue a chaque demarrage).

CREATE OR REPLACE FUNCTION ent_rls_etablissement() RETURNS uuid
LANGUAGE sql STABLE AS $$
    SELECT nullif(current_setting('app.etablissement_id', true), '')::uuid
$$;

CREATE OR REPLACE FUNCTION ent_rls_strict() RETURNS boolean
LANGUAGE sql STABLE AS $$
    SELECT coalesce(current_setting('app.rls_strict', true), '') IN ('on', '1', 'true')
$$;

DO $$
DECLARE
    t text;
    tables text[] := ARRAY[
        'ent_fiche_sante', 'ent_pai', 'ent_pai_piece', 'ent_infirmerie_passage',
        'ent_psy_entretien', 'ent_psy_rdv', 'ent_psy_dossier',
        'ent_incident_type', 'ent_sanction_type', 'ent_sanction_motif',
        'ent_incident', 'ent_sanction', 'ent_vs_commission', 'ent_vs_commission_dossier',
        'ent_fac_facture', 'ent_fac_ligne', 'ent_fac_encaissement', 'ent_fac_echeance',
        'ent_fac_quittance', 'ent_fac_sepa_mandat', 'ent_fac_sepa_prelevement',
        'ent_fac_rappel'
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
