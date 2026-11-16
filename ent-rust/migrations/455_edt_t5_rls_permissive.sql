-- 455 — Socle 10 (emploi du temps), lot T5 : RLS d'établissement du journal des versions et des versions individuelles
-- (449). Même politique que 452 à 454 (`ent_rls_poser`, définie en 580) ; placée juste après 454. Idempotent.

SELECT ent_rls_poser(ARRAY['ent_edt_version_journal', 'ent_edt_individuelle', 'ent_edt_individuelle_ligne']);
