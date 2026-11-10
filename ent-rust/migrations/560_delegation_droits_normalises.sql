-- D-036 (lot « normalisation JSON ») — délégations : `ent_delegation.droits` (JSONB) devient deux
-- tables typées. Un droit délégué = une ligne (action, portée) ; ses cibles désignées (classes ou
-- groupes) = une ligne par cible. Le moteur de droits (`droits_effectifs`) lit ces tables.
--
-- Reprise idempotente (les migrations sont rejouées à chaque démarrage) : seules les délégations
-- SANS ligne de droit sont reprises depuis l'ancienne colonne ; le code n'y écrit plus que `[]`.
-- L'ancienne colonne `droits` est conservée EN LECTURE SEULE (déclencheur ci-dessous) et sera
-- retirée dans un lot ultérieur (lot D-036 « retrait des colonnes reprises »).

CREATE TABLE IF NOT EXISTS ent_delegation_droit (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    delegation_id UUID NOT NULL REFERENCES ent_delegation (id) ON DELETE CASCADE,
    ordre SMALLINT NOT NULL CHECK (ordre BETWEEN 0 AND 19),
    action TEXT NOT NULL CHECK (length(action) BETWEEN 1 AND 80),
    portee TEXT NOT NULL CHECK (length(portee) BETWEEN 1 AND 40),
    PRIMARY KEY (delegation_id, ordre),
    CONSTRAINT ent_delegation_droit_unique_uq UNIQUE (delegation_id, action, portee)
);
CREATE INDEX IF NOT EXISTS ent_delegation_droit_etab_idx
    ON ent_delegation_droit (etablissement_id, delegation_id);

-- Cible désignée d'un droit (classe ou groupe de l'établissement : référence polymorphe, vérifiée
-- par le service à l'écriture, comme avant).
CREATE TABLE IF NOT EXISTS ent_delegation_droit_cible (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    delegation_id UUID NOT NULL,
    ordre SMALLINT NOT NULL,
    cible_id UUID NOT NULL,
    PRIMARY KEY (delegation_id, ordre, cible_id),
    FOREIGN KEY (delegation_id, ordre)
        REFERENCES ent_delegation_droit (delegation_id, ordre) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ent_delegation_droit_cible_etab_idx
    ON ent_delegation_droit_cible (etablissement_id, delegation_id);

-- Reprise des droits (délégations sans ligne de droit seulement).
INSERT INTO ent_delegation_droit (etablissement_id, delegation_id, ordre, action, portee)
SELECT d.etablissement_id, d.id, (j.ord - 1)::smallint, btrim(j.e ->> 'action'), btrim(j.e ->> 'portee')
  FROM ent_delegation d,
       LATERAL jsonb_array_elements(
           CASE WHEN jsonb_typeof(d.droits) = 'array' THEN d.droits ELSE '[]'::jsonb END
       ) WITH ORDINALITY AS j(e, ord)
 WHERE NOT EXISTS (SELECT 1 FROM ent_delegation_droit x WHERE x.delegation_id = d.id)
   AND j.ord <= 20
   AND jsonb_typeof(j.e) = 'object'
   AND length(btrim(coalesce(j.e ->> 'action', ''))) BETWEEN 1 AND 80
   AND length(btrim(coalesce(j.e ->> 'portee', ''))) BETWEEN 1 AND 40
ON CONFLICT DO NOTHING;

-- Reprise des cibles (une fois par droit repris : le droit n'a encore aucune cible).
INSERT INTO ent_delegation_droit_cible (etablissement_id, delegation_id, ordre, cible_id)
SELECT dr.etablissement_id, dr.delegation_id, dr.ordre, c.cible
  FROM ent_delegation_droit dr
  JOIN ent_delegation d ON d.id = dr.delegation_id,
       LATERAL (
           SELECT CASE WHEN t ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
                       THEN t::uuid END AS cible
             FROM jsonb_array_elements_text(
                      CASE WHEN jsonb_typeof(d.droits) = 'array'
                                AND jsonb_typeof(d.droits -> (dr.ordre::int) -> 'cibles') = 'array'
                           THEN d.droits -> (dr.ordre::int) -> 'cibles' ELSE '[]'::jsonb END
                  ) AS t
       ) c
 WHERE c.cible IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM ent_delegation_droit_cible y
                    WHERE y.delegation_id = dr.delegation_id AND y.ordre = dr.ordre)
ON CONFLICT DO NOTHING;

-- Ancienne colonne : lecture seule. Le code n'y écrit plus que `[]` (valeur par défaut).
ALTER TABLE ent_delegation ALTER COLUMN droits SET DEFAULT '[]'::jsonb;

CREATE OR REPLACE FUNCTION ent_delegation_droits_lecture_seule() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.droits IS DISTINCT FROM '[]'::jsonb
       AND (TG_OP = 'INSERT' OR NEW.droits IS DISTINCT FROM OLD.droits) THEN
        RAISE EXCEPTION 'ent_delegation.droits est en lecture seule (D-036) : utiliser ent_delegation_droit'
            USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_delegation_droits_lecture_seule ON ent_delegation;
CREATE TRIGGER ent_delegation_droits_lecture_seule
    BEFORE INSERT OR UPDATE OF droits ON ent_delegation
    FOR EACH ROW EXECUTE FUNCTION ent_delegation_droits_lecture_seule();

-- RLS permissive (politique des migrations 100 et 480).
DO $$
DECLARE
    t text;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_delegation_droit', 'ent_delegation_droit_cible'] LOOP
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
