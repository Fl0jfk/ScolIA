-- 583 — RLS d'établissement, lot « communication » (4 tables). Même politique que le pilote
-- (migration 100, docs/RLS_ISOLATION.md) : FORCE ROW LEVEL SECURITY + politique
-- ent_rls_etablissement (variable app.etablissement_id posée par TenantTx ; sans
-- variable : permissif tant que app.rls_strict n'est pas actif). Idempotent (rejoué à chaque
-- démarrage). Pose via ent_rls_poser (définie en 580).

SELECT ent_rls_poser(ARRAY[
        'ent_mail_outbox',
        'ent_push_device',
        'ent_push_outbox',
        'ent_sms_outbox'
]);
