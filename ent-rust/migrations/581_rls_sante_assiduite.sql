-- 581 — RLS d'établissement, lot « sante_assiduite » (21 tables). Même politique que le pilote
-- (migration 100, docs/RLS_ISOLATION.md) : FORCE ROW LEVEL SECURITY + politique
-- ent_rls_etablissement (variable app.etablissement_id posée par TenantTx ; sans
-- variable : permissif tant que app.rls_strict n'est pas actif). Idempotent (rejoué à chaque
-- démarrage). Pose via ent_rls_poser (définie en 580).

SELECT ent_rls_poser(ARRAY[
        'ent_absence',
        'ent_absence_declaration',
        'ent_absence_motif_etab',
        'ent_absence_rapprochement',
        'ent_absence_rh',
        'ent_absence_seance',
        'ent_absence_signal',
        'ent_alerte_cpe',
        'ent_appel',
        'ent_appel_ligne',
        'ent_appel_relance',
        'ent_dispense_vs',
        'ent_retard',
        'ent_signalement_assiduite',
        'ent_sortie_autorisation',
        'ent_sortie_controle',
        'ent_sortie_demande',
        'ent_sortie_param',
        'ent_sortie_regle_niveau',
        'ent_sortie_regle_regime',
        'ent_vs_regle_assiduite'
]);
