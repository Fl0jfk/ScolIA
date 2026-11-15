-- 453 — Socle 10 (emploi du temps), lot T3 : RLS d'établissement des tables créées en 445 (E13).
-- Même politique que 452 (`ent_rls_poser`, définie en 580) : FORCE ROW LEVEL SECURITY + politique
-- `ent_rls_etablissement`. Rejouée APRÈS 580 : placée juste après 452 dans MIGRATIONS. Idempotent.

SELECT ent_rls_poser(ARRAY[
        'ent_edt_lot',
        'ent_edt_lot_cible',
        'ent_edt_modification',
        'ent_edt_modification_interne'
]);
