-- 580 — RLS d'établissement, lot « eleves_responsables » (33 tables). Même politique que le pilote
-- (migration 100, docs/RLS_ISOLATION.md) : FORCE ROW LEVEL SECURITY + politique
-- ent_rls_etablissement (variable app.etablissement_id posée par TenantTx ; sans
-- variable : permissif tant que app.rls_strict n'est pas actif). Idempotent (rejoué à chaque
-- démarrage). Pose via ent_rls_poser (définie en 580).

CREATE OR REPLACE FUNCTION ent_rls_poser(tables text[]) RETURNS void
LANGUAGE plpgsql AS $$
DECLARE
    t text;
BEGIN
    FOREACH t IN ARRAY tables LOOP
        -- Déjà conforme (RLS + FORCE + politique d'établissement) : aucun verrou exclusif repris
        -- à chaque démarrage (évite les deadlocks avec le trafic en cours).
        CONTINUE WHEN EXISTS (
            SELECT 1 FROM pg_class c JOIN pg_policies p ON p.tablename = c.relname AND p.schemaname = 'public'
             WHERE c.relname = t AND c.relnamespace = 'public'::regnamespace
               AND c.relrowsecurity AND c.relforcerowsecurity
               AND p.policyname = 'ent_rls_etablissement'
               AND p.qual LIKE '%ent_rls_strict()%' AND p.with_check LIKE '%ent_rls_strict()%');
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

SELECT ent_rls_poser(ARRAY[
        'ent_adresse_protegee',
        'ent_convention_stage',
        'ent_coordonnee_verification',
        'ent_doc_a_signer',
        'ent_document_grant',
        'ent_eleve',
        'ent_eleve_compte',
        'ent_eleve_contact_urgence',
        'ent_eleve_dispositif',
        'ent_eleve_document',
        'ent_eleve_invitation',
        'ent_eleve_lien',
        'ent_eleve_orphelin',
        'ent_eleve_periode',
        'ent_eleve_scolarite_annee',
        'ent_entreprise',
        'ent_foyer',
        'ent_identite_journal',
        'ent_import_ligne',
        'ent_import_lot',
        'ent_info_comp_champ',
        'ent_info_comp_valeur',
        'ent_journal_modif',
        'ent_personne',
        'ent_personne_consentement',
        'ent_personne_coordonnee',
        'ent_personne_coordonnee_histo',
        'ent_personne_divergence_decision',
        'ent_personne_fusion',
        'ent_personne_preference',
        'ent_personne_preference_version',
        'ent_personne_reserve',
        'ent_personne_version'
]);
