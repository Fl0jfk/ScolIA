-- T1 suites : micro-banc d'écriture (docs/securite/T1_SEARCH_PATH.md, « Suites », §5).
-- À lancer sur une base LOCALE de test (jamais une base réelle), migrée jusqu'au bout :
--   psql -v n=50000 -f scripts/t1_banc_ecriture.sql "$ENT_DATABASE_URL"
-- Insère `n` élèves (table `ent_eleve` : 10 déclencheurs, tous sur des fonctions qui portent
-- `SET search_path = pg_catalog, public, pg_temp`) et rend la durée en ms, « apres » (état migré, avec 690) puis
-- « avant » (clauses retirées dans la transaction : état d'avant 690). Tout est annulé : rien n'est écrit.
\set ON_ERROR_STOP on
\timing off
\if :{?n} \else \set n 50000 \endif

\if :{?essais} \else \set essais 5 \endif
SELECT set_config('banc.n', :'n', false), set_config('banc.essais', :'essais', false);
CREATE TEMP TABLE IF NOT EXISTS banc_resultat (variante text, essai int, ms numeric);

CREATE OR REPLACE FUNCTION pg_temp.banc(p_n int) RETURNS numeric LANGUAGE plpgsql AS $f$
DECLARE t0 timestamptz; v_etab uuid;
BEGIN
  SELECT etablissement_id INTO STRICT v_etab FROM (SELECT etablissement_id FROM public.ent_eleve ORDER BY etablissement_id LIMIT 1) e;
  t0 := clock_timestamp();
  INSERT INTO public.ent_eleve (id, etablissement_id, nom, prenom, status)
  SELECT gen_random_uuid(), v_etab, 'Banc' || g, 'T1', 'actif' FROM generate_series(1, p_n) g;
  RETURN extract(epoch FROM clock_timestamp() - t0) * 1000;
END $f$;

DO $b$
DECLARE i int; r numeric; v text;
BEGIN
  FOREACH v IN ARRAY ARRAY['apres', 'avant'] LOOP
    FOR i IN 1..current_setting('banc.essais')::int LOOP
      r := NULL;
      -- Chaque mesure : sous-transaction annulée par une exception ; le résultat est gardé dans une variable.
      BEGIN
        IF v = 'avant' THEN
          EXECUTE (SELECT string_agg(format('ALTER FUNCTION %s RESET search_path', p.oid::regprocedure), '; ')
                     FROM pg_proc p WHERE p.pronamespace = 'public'::regnamespace AND NOT p.prosecdef
                      AND p.proconfig = ARRAY['search_path=pg_catalog, public, pg_temp']);
        END IF;
        r := pg_temp.banc(current_setting('banc.n')::int);
        RAISE EXCEPTION 'annulation du banc' USING ERRCODE = 'P0001';
      EXCEPTION WHEN SQLSTATE 'P0001' THEN NULL;
      END;
      INSERT INTO pg_temp.banc_resultat VALUES (v, i, r);
    END LOOP;
  END LOOP;
END $b$;

SELECT variante, count(*) AS essais, round(percentile_cont(0.5) WITHIN GROUP (ORDER BY ms)::numeric, 0) AS mediane_ms,
       round(min(ms), 0) AS min_ms, round(max(ms), 0) AS max_ms
  FROM pg_temp.banc_resultat GROUP BY 1 ORDER BY 1;
