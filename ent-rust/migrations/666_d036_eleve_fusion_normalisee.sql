-- D-036 (lot 2) — annulation de fusion d'élèves : trois colonnes JSONB qui servent de MÉMOIRE à
-- l'annulation (elles sont relues pour défaire la fusion) deviennent des tables typées.
--   * `ent_eleve_fusion.conflits_resolus`  → `ent_eleve_fusion_conflit` (+ `_conflit_cle`)
--   * `ent_eleve_fusion.doublons_touches`  → `ent_eleve_fusion_doublon`
--   * `ent_eleve_fusion_ligne.cle`         → `ent_eleve_fusion_ligne_cle`
-- Une clé de ligne = couples (colonne, valeur) des colonnes de la clé primaire hors `eleve_id`.
-- (`ent_eleve_fusion.resume` reste : résumé figé affiché, jamais relu pour agir — exception D-036.)
--
-- Ces tables sont en AJOUT SEUL comme le registre de fusion. Reprise idempotente : une fusion (ou
-- une ligne) n'est reprise que si elle n'a encore aucune ligne typée ; les parents ne sont jamais
-- modifiés (ils sont gardés par déclencheur). Anciennes colonnes facultatives, en LECTURE SEULE.

CREATE OR REPLACE FUNCTION ent_d036_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '% est en ajout seul (registre de fusion)', TG_TABLE_NAME USING ERRCODE = '42501';
END
$$;

SELECT ent_d036_contrainte('ent_eleve_fusion_ligne', 'ent_eleve_fusion_ligne_etab_id_uq', 'UNIQUE (etablissement_id, id)');
SELECT ent_d036_retirer_fk_simple('ent_eleve_fusion_ligne', 'fusion_id');
SELECT ent_d036_contrainte('ent_eleve_fusion_ligne', 'ent_eleve_fusion_ligne_fusion_fk',
    'FOREIGN KEY (etablissement_id, fusion_id) REFERENCES ent_eleve_fusion (etablissement_id, id)');
ALTER TABLE ent_eleve_fusion_ligne ALTER COLUMN cle DROP NOT NULL;
ALTER TABLE ent_eleve_fusion ALTER COLUMN conflits_resolus DROP NOT NULL;
ALTER TABLE ent_eleve_fusion ALTER COLUMN doublons_touches DROP NOT NULL;

CREATE TABLE IF NOT EXISTS ent_eleve_fusion_ligne_cle (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    ligne_id BIGINT NOT NULL,
    colonne TEXT NOT NULL CHECK (colonne ~ '^[a-z][a-z0-9_]*$' AND length(colonne) <= 63),
    valeur TEXT NOT NULL CHECK (length(valeur) <= 1000),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (ligne_id, colonne),
    CONSTRAINT ent_eleve_fusion_ligne_cle_ligne_fk FOREIGN KEY (etablissement_id, ligne_id)
        REFERENCES ent_eleve_fusion_ligne (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_eleve_fusion_ligne_cle_etab_idx
    ON ent_eleve_fusion_ligne_cle (etablissement_id, ligne_id);

CREATE TABLE IF NOT EXISTS ent_eleve_fusion_conflit (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    fusion_id UUID NOT NULL,
    ordre INTEGER NOT NULL CHECK (ordre >= 0),
    table_nom TEXT NOT NULL CHECK (table_nom ~ '^[a-z][a-z0-9_]*$' AND length(table_nom) <= 63),
    domaine TEXT NOT NULL CHECK (length(domaine) <= 200),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_fusion_conflit_ordre_uq UNIQUE (fusion_id, ordre),
    CONSTRAINT ent_eleve_fusion_conflit_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_eleve_fusion_conflit_fusion_fk FOREIGN KEY (etablissement_id, fusion_id)
        REFERENCES ent_eleve_fusion (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_eleve_fusion_conflit_etab_idx
    ON ent_eleve_fusion_conflit (etablissement_id, fusion_id);

CREATE TABLE IF NOT EXISTS ent_eleve_fusion_conflit_cle (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    conflit_id UUID NOT NULL,
    colonne TEXT NOT NULL CHECK (colonne ~ '^[a-z][a-z0-9_]*$' AND length(colonne) <= 63),
    valeur TEXT NOT NULL CHECK (length(valeur) <= 1000),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (conflit_id, colonne),
    CONSTRAINT ent_eleve_fusion_conflit_cle_conflit_fk FOREIGN KEY (etablissement_id, conflit_id)
        REFERENCES ent_eleve_fusion_conflit (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_eleve_fusion_conflit_cle_etab_idx
    ON ent_eleve_fusion_conflit_cle (etablissement_id, conflit_id);

CREATE TABLE IF NOT EXISTS ent_eleve_fusion_doublon (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    fusion_id UUID NOT NULL,
    doublon_id UUID NOT NULL,
    statut_avant TEXT NOT NULL CHECK (statut_avant IN ('a_examiner', 'fusionne', 'obsolete')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_fusion_doublon_uq UNIQUE (fusion_id, doublon_id),
    CONSTRAINT ent_eleve_fusion_doublon_fusion_fk FOREIGN KEY (etablissement_id, fusion_id)
        REFERENCES ent_eleve_fusion (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_eleve_fusion_doublon_etab_idx
    ON ent_eleve_fusion_doublon (etablissement_id, fusion_id);

-- Reprise des clés de lignes (lignes sans clé typée seulement).
INSERT INTO ent_eleve_fusion_ligne_cle (etablissement_id, ligne_id, colonne, valeur)
SELECT l.etablissement_id, l.id, k.key, k.value
  FROM ent_eleve_fusion_ligne l, LATERAL jsonb_each_text(l.cle) k
 WHERE jsonb_typeof(l.cle) = 'object'
   AND k.key ~ '^[a-z][a-z0-9_]*$' AND length(k.key) <= 63
   AND k.value IS NOT NULL AND length(k.value) <= 1000
   AND NOT EXISTS (SELECT 1 FROM ent_eleve_fusion_ligne_cle x WHERE x.ligne_id = l.id)
ON CONFLICT DO NOTHING;

-- Reprise des conflits résolus. L'identifiant d'un conflit repris est DÉTERMINISTE
-- (md5 de la fusion et du rang) : les deux instructions retrouvent le même identifiant sans table
-- temporaire (les instructions d'une migration peuvent passer par des connexions différentes).
INSERT INTO ent_eleve_fusion_conflit (id, etablissement_id, fusion_id, ordre, table_nom, domaine)
SELECT md5(f.id::text || '/conflit/' || j.ord)::uuid, f.etablissement_id, f.id, (j.ord - 1)::integer,
       j.e ->> 'table', coalesce(j.e ->> 'domaine', '')
  FROM ent_eleve_fusion f, LATERAL jsonb_array_elements(
           CASE WHEN jsonb_typeof(f.conflits_resolus) = 'array' THEN f.conflits_resolus ELSE '[]'::jsonb END)
           WITH ORDINALITY AS j(e, ord)
 WHERE jsonb_typeof(j.e) = 'object' AND (j.e ->> 'table') ~ '^[a-z][a-z0-9_]*$' AND length(j.e ->> 'table') <= 63
   AND length(coalesce(j.e ->> 'domaine', '')) <= 200
   AND NOT EXISTS (SELECT 1 FROM ent_eleve_fusion_conflit c WHERE c.fusion_id = f.id)
ON CONFLICT DO NOTHING;
INSERT INTO ent_eleve_fusion_conflit_cle (etablissement_id, conflit_id, colonne, valeur)
SELECT f.etablissement_id, md5(f.id::text || '/conflit/' || j.ord)::uuid, k.key, k.value
  FROM ent_eleve_fusion f, LATERAL jsonb_array_elements(
           CASE WHEN jsonb_typeof(f.conflits_resolus) = 'array' THEN f.conflits_resolus ELSE '[]'::jsonb END)
           WITH ORDINALITY AS j(e, ord),
       LATERAL jsonb_each_text(CASE WHEN jsonb_typeof(j.e -> 'cle') = 'object' THEN j.e -> 'cle' ELSE '{}'::jsonb END) k
 WHERE jsonb_typeof(j.e) = 'object'
   AND k.key ~ '^[a-z][a-z0-9_]*$' AND length(k.key) <= 63 AND k.value IS NOT NULL AND length(k.value) <= 1000
   AND EXISTS (SELECT 1 FROM ent_eleve_fusion_conflit c WHERE c.id = md5(f.id::text || '/conflit/' || j.ord)::uuid)
   AND NOT EXISTS (SELECT 1 FROM ent_eleve_fusion_conflit_cle x
                    WHERE x.conflit_id = md5(f.id::text || '/conflit/' || j.ord)::uuid)
ON CONFLICT DO NOTHING;

-- Reprise des doublons touchés.
INSERT INTO ent_eleve_fusion_doublon (id, etablissement_id, fusion_id, doublon_id, statut_avant)
SELECT gen_random_uuid(), f.etablissement_id, f.id, (j.e ->> 'id')::uuid, j.e ->> 'statut'
  FROM ent_eleve_fusion f, LATERAL jsonb_array_elements(
           CASE WHEN jsonb_typeof(f.doublons_touches) = 'array' THEN f.doublons_touches ELSE '[]'::jsonb END) AS j(e)
 WHERE jsonb_typeof(j.e) = 'object'
   AND (j.e ->> 'id') ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
   AND (j.e ->> 'statut') IN ('a_examiner', 'fusionne', 'obsolete')
   AND NOT EXISTS (SELECT 1 FROM ent_eleve_fusion_doublon d WHERE d.fusion_id = f.id)
ON CONFLICT DO NOTHING;

-- Lecture : conflits et clés au format d'échange du code.
CREATE OR REPLACE FUNCTION ent_eleve_fusion_conflits_json(etab uuid, fusion uuid) RETURNS jsonb
LANGUAGE sql STABLE AS $$
    SELECT coalesce(jsonb_agg(jsonb_build_object(
               'table', c.table_nom, 'domaine', c.domaine,
               'cle', coalesce((SELECT jsonb_object_agg(k.colonne, k.valeur) FROM ent_eleve_fusion_conflit_cle k
                                 WHERE k.etablissement_id = c.etablissement_id AND k.conflit_id = c.id), '{}'::jsonb))
           ORDER BY c.ordre), '[]'::jsonb)
      FROM ent_eleve_fusion_conflit c WHERE c.etablissement_id = etab AND c.fusion_id = fusion
$$;

-- Anciennes colonnes : lecture seule (le code n'y écrit plus).
CREATE OR REPLACE FUNCTION ent_eleve_fusion_json_lecture_seule() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_TABLE_NAME = 'ent_eleve_fusion_ligne' THEN
        IF NEW.cle IS NOT NULL THEN
            RAISE EXCEPTION 'ent_eleve_fusion_ligne.cle est en lecture seule (D-036) : utiliser ent_eleve_fusion_ligne_cle'
                USING ERRCODE = '42501';
        END IF;
    ELSIF NEW.conflits_resolus IS DISTINCT FROM '[]'::jsonb OR NEW.doublons_touches IS DISTINCT FROM '[]'::jsonb THEN
        IF NEW.conflits_resolus IS NOT NULL OR NEW.doublons_touches IS NOT NULL THEN
            RAISE EXCEPTION 'conflits_resolus et doublons_touches sont en lecture seule (D-036) : utiliser les tables de fusion'
                USING ERRCODE = '42501';
        END IF;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_eleve_fusion_ligne_cle_lecture_seule ON ent_eleve_fusion_ligne;
CREATE TRIGGER ent_eleve_fusion_ligne_cle_lecture_seule
    BEFORE INSERT ON ent_eleve_fusion_ligne
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_fusion_json_lecture_seule();
DROP TRIGGER IF EXISTS ent_eleve_fusion_json_lecture_seule ON ent_eleve_fusion;
CREATE TRIGGER ent_eleve_fusion_json_lecture_seule
    BEFORE INSERT ON ent_eleve_fusion
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_fusion_json_lecture_seule();

-- Ajout seul.
DROP TRIGGER IF EXISTS ent_eleve_fusion_ligne_cle_ajout_seul ON ent_eleve_fusion_ligne_cle;
CREATE TRIGGER ent_eleve_fusion_ligne_cle_ajout_seul BEFORE UPDATE OR DELETE ON ent_eleve_fusion_ligne_cle
    FOR EACH ROW EXECUTE FUNCTION ent_d036_ajout_seul();
DROP TRIGGER IF EXISTS ent_eleve_fusion_conflit_ajout_seul ON ent_eleve_fusion_conflit;
CREATE TRIGGER ent_eleve_fusion_conflit_ajout_seul BEFORE UPDATE OR DELETE ON ent_eleve_fusion_conflit
    FOR EACH ROW EXECUTE FUNCTION ent_d036_ajout_seul();
DROP TRIGGER IF EXISTS ent_eleve_fusion_conflit_cle_ajout_seul ON ent_eleve_fusion_conflit_cle;
CREATE TRIGGER ent_eleve_fusion_conflit_cle_ajout_seul BEFORE UPDATE OR DELETE ON ent_eleve_fusion_conflit_cle
    FOR EACH ROW EXECUTE FUNCTION ent_d036_ajout_seul();
DROP TRIGGER IF EXISTS ent_eleve_fusion_doublon_ajout_seul ON ent_eleve_fusion_doublon;
CREATE TRIGGER ent_eleve_fusion_doublon_ajout_seul BEFORE UPDATE OR DELETE ON ent_eleve_fusion_doublon
    FOR EACH ROW EXECUTE FUNCTION ent_d036_ajout_seul();

SELECT ent_rls_poser(ARRAY['ent_eleve_fusion_ligne_cle', 'ent_eleve_fusion_conflit',
                           'ent_eleve_fusion_conflit_cle', 'ent_eleve_fusion_doublon']);
