-- Personnels, lot P5 (06_PERSONNELS.md §8.7, PRS-66) : responsabilités de groupes et d'ateliers DATÉES.
-- `ent_groupe_responsable` (098) devient une table datée : une ligne = un agent responsable, animateur ou
-- co-animateur d'un groupe sur une période. La clé (groupe, compte) cède la place à une contrainte d'exclusion
-- datée (pas deux périodes qui se chevauchent pour un même agent et un même groupe). `user_id` est conservé
-- en PROJECTION (compte de l'agent) : les lecteurs existants (portée `mes_groupes`, cahier, périmètre) ne
-- changent que par la condition de dates. Migration additive, rejouable. Aucun JSON, aucun jour UTC : le jour
-- est celui de l'établissement (`ent_aujourdhui_etab`).

CREATE EXTENSION IF NOT EXISTS btree_gist;

ALTER TABLE ent_groupe_responsable ADD COLUMN IF NOT EXISTS id UUID DEFAULT gen_random_uuid();
ALTER TABLE ent_groupe_responsable ADD COLUMN IF NOT EXISTS agent_id UUID;
ALTER TABLE ent_groupe_responsable ADD COLUMN IF NOT EXISTS nature TEXT NOT NULL DEFAULT 'responsable';
ALTER TABLE ent_groupe_responsable ADD COLUMN IF NOT EXISTS du DATE;
ALTER TABLE ent_groupe_responsable ADD COLUMN IF NOT EXISTS au DATE;
ALTER TABLE ent_groupe_responsable ADD COLUMN IF NOT EXISTS termine_par_user_id UUID;

-- Reprise : agent du compte (personne liée au compte) et date de début = jour de création, à Paris.
UPDATE ent_groupe_responsable r
   SET id = gen_random_uuid()
 WHERE r.id IS NULL;

UPDATE ent_groupe_responsable r
   SET agent_id = (SELECT a.id
                     FROM ent_agent a
                     JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
                    WHERE a.etablissement_id = r.etablissement_id AND p.user_id = r.user_id
                      AND a.fusionne_dans_agent_id IS NULL
                    LIMIT 1)
 WHERE r.agent_id IS NULL AND r.user_id IS NOT NULL;

UPDATE ent_groupe_responsable r
   SET du = ent_aujourdhui_etab(r.etablissement_id, r.created_at)
 WHERE r.du IS NULL;

ALTER TABLE ent_groupe_responsable ALTER COLUMN id SET NOT NULL;
ALTER TABLE ent_groupe_responsable ALTER COLUMN du SET NOT NULL;

DO $$
DECLARE
    pk_cols text;
BEGIN
    SELECT string_agg(a.attname, ',' ORDER BY a.attname) INTO pk_cols
      FROM pg_constraint c
      JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = ANY (c.conkey)
     WHERE c.conrelid = 'ent_groupe_responsable'::regclass AND c.contype = 'p';
    IF pk_cols = 'groupe_id,user_id' THEN
        ALTER TABLE ent_groupe_responsable DROP CONSTRAINT ent_groupe_responsable_pkey;
        ALTER TABLE ent_groupe_responsable ADD PRIMARY KEY (id);
    ELSIF pk_cols IS NULL THEN
        ALTER TABLE ent_groupe_responsable ADD PRIMARY KEY (id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_groupe_resp_nature_chk') THEN
        ALTER TABLE ent_groupe_responsable ADD CONSTRAINT ent_groupe_resp_nature_chk
            CHECK (nature IN ('responsable', 'animateur', 'co_animateur'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_groupe_resp_dates_chk') THEN
        ALTER TABLE ent_groupe_responsable ADD CONSTRAINT ent_groupe_resp_dates_chk
            CHECK (au IS NULL OR au >= du);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_groupe_resp_qui_chk') THEN
        ALTER TABLE ent_groupe_responsable ADD CONSTRAINT ent_groupe_resp_qui_chk
            CHECK (agent_id IS NOT NULL OR user_id IS NOT NULL);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_groupe_resp_agent_fk') THEN
        ALTER TABLE ent_groupe_responsable ADD CONSTRAINT ent_groupe_resp_agent_fk
            FOREIGN KEY (etablissement_id, agent_id) REFERENCES ent_agent (etablissement_id, id) NOT VALID;
    END IF;
    -- Un même agent n'a pas deux responsabilités qui se chevauchent sur un même groupe.
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_groupe_resp_sans_chevauchement') THEN
        ALTER TABLE ent_groupe_responsable ADD CONSTRAINT ent_groupe_resp_sans_chevauchement
            EXCLUDE USING gist (groupe_id WITH =, agent_id WITH =,
                                daterange(du, au, '[]') WITH &&) WHERE (agent_id IS NOT NULL);
    END IF;
END
$$;

-- La clé primaire (groupe, compte) est partie : le compte peut être absent (agent sans compte).
ALTER TABLE ent_groupe_responsable ALTER COLUMN user_id DROP NOT NULL;

CREATE INDEX IF NOT EXISTS ent_groupe_resp_agent_idx
    ON ent_groupe_responsable (etablissement_id, agent_id, du);

-- Garde et projection : l'agent est de l'établissement, le compte projeté est celui de l'agent,
-- le jour de début par défaut est celui de l'établissement.
CREATE OR REPLACE FUNCTION ent_trg_groupe_responsable() RETURNS trigger AS $$
BEGIN
    IF NEW.id IS NULL THEN
        NEW.id := gen_random_uuid();
    END IF;
    IF NEW.du IS NULL THEN
        NEW.du := ent_aujourdhui_etab(NEW.etablissement_id);
    END IF;
    IF NEW.agent_id IS NULL AND NEW.user_id IS NOT NULL THEN
        NEW.agent_id := (SELECT a.id
                           FROM ent_agent a
                           JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
                          WHERE a.etablissement_id = NEW.etablissement_id AND p.user_id = NEW.user_id
                            AND a.fusionne_dans_agent_id IS NULL
                          LIMIT 1);
    END IF;
    IF NEW.agent_id IS NOT NULL THEN
        NEW.user_id := (SELECT p.user_id
                          FROM ent_agent a
                          JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
                         WHERE a.etablissement_id = NEW.etablissement_id AND a.id = NEW.agent_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM ent_groupe g WHERE g.id = NEW.groupe_id AND g.etablissement_id = NEW.etablissement_id) THEN
        RAISE EXCEPTION 'groupe d''un autre établissement' USING ERRCODE = '23503';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_groupe_responsable_garde ON ent_groupe_responsable;
CREATE TRIGGER ent_groupe_responsable_garde
    BEFORE INSERT OR UPDATE OF agent_id, user_id, groupe_id ON ent_groupe_responsable
    FOR EACH ROW EXECUTE FUNCTION ent_trg_groupe_responsable();
