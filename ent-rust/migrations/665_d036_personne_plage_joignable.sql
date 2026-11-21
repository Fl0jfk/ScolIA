-- D-036 (lot 2) — `ent_personne_preference.joignable` et `ent_personne_preference_version.joignable`
-- (JSONB : plages horaires où appeler la personne) deviennent des tables de plages typées : une
-- ligne par plage (jours 1 = lundi … 7 = dimanche, heure de début < heure de fin), pour l'état
-- courant et pour chaque version de l'historique. Le JSON `[{jours, de, a}]` de l'API est
-- reconstruit à la lecture.
--
-- « Liste renseignée mais vide » reste distincte de « jamais renseigné » : colonne
-- `joignable_definie`. Reprise idempotente : seules les lignes dont `joignable_definie` est faux
-- sont reprises (plages mal formées ignorées), puis marquées. Anciennes colonnes en LECTURE SEULE.

ALTER TABLE ent_personne_preference ADD COLUMN IF NOT EXISTS joignable_definie BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_personne_preference_version ADD COLUMN IF NOT EXISTS joignable_definie BOOLEAN NOT NULL DEFAULT false;

SELECT ent_d036_contrainte('ent_personne_preference_version', 'ent_personne_preference_version_etab_id_uq',
    'UNIQUE (etablissement_id, id)');

CREATE TABLE IF NOT EXISTS ent_personne_plage_joignable (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    personne_id UUID NOT NULL,
    ordre SMALLINT NOT NULL CHECK (ordre BETWEEN 0 AND 13),
    jours SMALLINT[] NOT NULL CHECK (cardinality(jours) BETWEEN 1 AND 7 AND jours <@ ARRAY[1,2,3,4,5,6,7]::smallint[]),
    de TIME NOT NULL,
    a TIME NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_personne_plage_joignable_ordre_uq UNIQUE (personne_id, ordre),
    CONSTRAINT ent_personne_plage_joignable_horaire_chk CHECK (de < a),
    CONSTRAINT ent_personne_plage_joignable_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ent_personne_plage_joignable_etab_idx
    ON ent_personne_plage_joignable (etablissement_id, personne_id);

CREATE TABLE IF NOT EXISTS ent_personne_preference_version_plage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    version_id UUID NOT NULL,
    ordre SMALLINT NOT NULL CHECK (ordre BETWEEN 0 AND 13),
    jours SMALLINT[] NOT NULL CHECK (cardinality(jours) BETWEEN 1 AND 7 AND jours <@ ARRAY[1,2,3,4,5,6,7]::smallint[]),
    de TIME NOT NULL,
    a TIME NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_personne_preference_version_plage_ordre_uq UNIQUE (version_id, ordre),
    CONSTRAINT ent_personne_preference_version_plage_horaire_chk CHECK (de < a),
    CONSTRAINT ent_personne_preference_version_plage_version_fk FOREIGN KEY (etablissement_id, version_id)
        REFERENCES ent_personne_preference_version (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ent_personne_preference_version_plage_etab_idx
    ON ent_personne_preference_version_plage (etablissement_id, version_id);

-- Reprise de l'état courant.
INSERT INTO ent_personne_plage_joignable (id, etablissement_id, personne_id, ordre, jours, de, a)
SELECT gen_random_uuid(), p.etablissement_id, p.personne_id, (x.ord - 1)::smallint, x.jours, x.de, x.a
  FROM ent_personne_preference p,
       LATERAL (
           SELECT j.ord,
                  ARRAY(SELECT DISTINCT t::smallint FROM jsonb_array_elements_text(
                            CASE WHEN jsonb_typeof(j.e -> 'jours') = 'array' THEN j.e -> 'jours' ELSE '[]'::jsonb END) t
                         WHERE t ~ '^[1-7]$') AS jours,
                  CASE WHEN j.e ->> 'de' ~ '^([01][0-9]|2[0-3]):[0-5][0-9]$' THEN (j.e ->> 'de')::time END AS de,
                  CASE WHEN j.e ->> 'a' ~ '^([01][0-9]|2[0-3]):[0-5][0-9]$' THEN (j.e ->> 'a')::time END AS a
             FROM jsonb_array_elements(p.joignable) WITH ORDINALITY AS j(e, ord)
            WHERE j.ord <= 14 AND jsonb_typeof(j.e) = 'object'
       ) x
 WHERE NOT p.joignable_definie AND jsonb_typeof(p.joignable) = 'array'
   AND cardinality(x.jours) >= 1 AND x.de IS NOT NULL AND x.a IS NOT NULL AND x.de < x.a
ON CONFLICT DO NOTHING;
UPDATE ent_personne_preference SET joignable_definie = true
 WHERE NOT joignable_definie AND jsonb_typeof(joignable) = 'array';

-- Reprise de l'historique.
INSERT INTO ent_personne_preference_version_plage (id, etablissement_id, version_id, ordre, jours, de, a)
SELECT gen_random_uuid(), v.etablissement_id, v.id, (x.ord - 1)::smallint, x.jours, x.de, x.a
  FROM ent_personne_preference_version v,
       LATERAL (
           SELECT j.ord,
                  ARRAY(SELECT DISTINCT t::smallint FROM jsonb_array_elements_text(
                            CASE WHEN jsonb_typeof(j.e -> 'jours') = 'array' THEN j.e -> 'jours' ELSE '[]'::jsonb END) t
                         WHERE t ~ '^[1-7]$') AS jours,
                  CASE WHEN j.e ->> 'de' ~ '^([01][0-9]|2[0-3]):[0-5][0-9]$' THEN (j.e ->> 'de')::time END AS de,
                  CASE WHEN j.e ->> 'a' ~ '^([01][0-9]|2[0-3]):[0-5][0-9]$' THEN (j.e ->> 'a')::time END AS a
             FROM jsonb_array_elements(v.joignable) WITH ORDINALITY AS j(e, ord)
            WHERE j.ord <= 14 AND jsonb_typeof(j.e) = 'object'
       ) x
 WHERE NOT v.joignable_definie AND jsonb_typeof(v.joignable) = 'array'
   AND cardinality(x.jours) >= 1 AND x.de IS NOT NULL AND x.a IS NOT NULL AND x.de < x.a
ON CONFLICT DO NOTHING;
UPDATE ent_personne_preference_version SET joignable_definie = true
 WHERE NOT joignable_definie AND jsonb_typeof(joignable) = 'array';

-- Plages au format d'échange de l'API.
CREATE OR REPLACE FUNCTION ent_personne_joignable_json(etab uuid, pers uuid) RETURNS jsonb
LANGUAGE sql STABLE AS $$
    SELECT coalesce(jsonb_agg(jsonb_build_object('jours', to_jsonb(p.jours),
                                                 'de', to_char(p.de, 'HH24:MI'),
                                                 'a', to_char(p.a, 'HH24:MI')) ORDER BY p.ordre), '[]'::jsonb)
      FROM ent_personne_plage_joignable p WHERE p.etablissement_id = etab AND p.personne_id = pers
$$;

CREATE OR REPLACE FUNCTION ent_personne_preference_joignable_lecture_seule() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.joignable IS NOT NULL AND (TG_OP = 'INSERT' OR NEW.joignable IS DISTINCT FROM OLD.joignable) THEN
        RAISE EXCEPTION '%.joignable est en lecture seule (D-036) : utiliser les tables de plages', TG_TABLE_NAME
            USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_personne_preference_joignable_lecture_seule ON ent_personne_preference;
CREATE TRIGGER ent_personne_preference_joignable_lecture_seule
    BEFORE INSERT OR UPDATE OF joignable ON ent_personne_preference
    FOR EACH ROW EXECUTE FUNCTION ent_personne_preference_joignable_lecture_seule();
DROP TRIGGER IF EXISTS ent_personne_preference_version_joignable_lecture_seule ON ent_personne_preference_version;
CREATE TRIGGER ent_personne_preference_version_joignable_lecture_seule
    BEFORE INSERT OR UPDATE OF joignable ON ent_personne_preference_version
    FOR EACH ROW EXECUTE FUNCTION ent_personne_preference_joignable_lecture_seule();

SELECT ent_rls_poser(ARRAY['ent_personne_plage_joignable', 'ent_personne_preference_version_plage']);
