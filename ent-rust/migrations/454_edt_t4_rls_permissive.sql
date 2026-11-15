-- 454 — Socle 10 (emploi du temps), lot T4 : RLS d'établissement du journal de localisation (447).
-- Même politique que 452 et 453 (`ent_rls_poser`, définie en 580) ; placée juste après 453. Idempotent.

SELECT ent_rls_poser(ARRAY['ent_edt_localisation_journal']);
