-- 582 — RLS d'établissement, lot « finances » (8 tables). Même politique que le pilote
-- (migration 100, docs/RLS_ISOLATION.md) : FORCE ROW LEVEL SECURITY + politique
-- ent_rls_etablissement (variable app.etablissement_id posée par TenantTx ; sans
-- variable : permissif tant que app.rls_strict n'est pas actif). Idempotent (rejoué à chaque
-- démarrage). Pose via ent_rls_poser (définie en 580).

SELECT ent_rls_poser(ARRAY[
        'ent_bank_rapprochement',
        'ent_bourse_type',
        'ent_eleve_bourse',
        'ent_foyer_iban_histo',
        'ent_foyer_quotient',
        'ent_mandat_sepa_demande',
        'ent_tarif_conso',
        'ent_taux_tva'
]);
