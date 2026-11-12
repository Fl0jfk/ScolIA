-- 452 — Socle 10 (emploi du temps), lots T1 et T2 : RLS d'établissement des tables créées en 441, 442,
-- 443 et 444 (E13). Même politique que le pilote (migration 100, docs/RLS_ISOLATION.md) via
-- `ent_rls_poser` (définie en 580) : FORCE ROW LEVEL SECURITY + politique `ent_rls_etablissement`
-- (variable `app.etablissement_id` posée par TenantTx ; sans variable : permissif tant que
-- `app.rls_strict` n'est pas actif). Idempotent. Rejouée APRÈS 580 : placée après 587 dans MIGRATIONS.

SELECT ent_rls_poser(ARRAY[
        'ent_edt_besoin',
        'ent_edt_piece',
        'ent_edt_placement',
        'ent_edt_placement_lieu',
        'ent_edt_seance_intervenant',
        'ent_edt_seance_lieu',
        'ent_edt_seance_public_fusion',
        'ent_edt_version',
        'ent_edt_version_perimetre'
]);
