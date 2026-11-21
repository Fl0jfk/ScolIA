-- Socle 6 (personnels), lot P3, migration 362 : l'agent comme sujet des autres tables
-- (`docs/socles/06_PERSONNELS.md` §8.14, §17 n° 362 ; R-P3).
--
-- Avant : le créneau d'emploi du temps, la séance et l'absence du personnel désignaient un COMPTE
-- (`prof_user_id`, `remplacant_user_id`, `user_id`). Un intervenant sans compte, un contractuel pas
-- encore invité, un remplaçant désigné avant son arrivée ne pouvaient pas y figurer.
--
-- Après : ces trois tables portent aussi `agent_id` (et `remplacant_agent_id` pour la séance). Les
-- colonnes `*_user_id` restent (projection tant que des lecteurs existent, retrait au lot P14) ;
-- `agent_id` est recalculé depuis le compte à chaque écriture qui change le compte, et rempli par
-- reprise pour l'existant. Aucune ligne n'est rejetée : un compte sans agent laisse `agent_id` NULL.
-- Additive, rejouable.

-- L'agent (non fusionné) d'un compte dans un établissement.
CREATE OR REPLACE FUNCTION ent_agent_du_compte(p_etab UUID, p_user UUID)
RETURNS UUID LANGUAGE sql STABLE AS $$
    SELECT a.id
      FROM ent_agent a
      JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
     WHERE a.etablissement_id = p_etab AND p.user_id = p_user AND a.fusionne_dans_agent_id IS NULL
     LIMIT 1
$$;

ALTER TABLE ent_edt_creneau ADD COLUMN IF NOT EXISTS agent_id UUID;
ALTER TABLE ent_edt_seance ADD COLUMN IF NOT EXISTS agent_id UUID;
ALTER TABLE ent_edt_seance ADD COLUMN IF NOT EXISTS remplacant_agent_id UUID;
ALTER TABLE ent_absence_rh ADD COLUMN IF NOT EXISTS agent_id UUID;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_creneau_agent_fk') THEN
        ALTER TABLE ent_edt_creneau ADD CONSTRAINT ent_edt_creneau_agent_fk
            FOREIGN KEY (etablissement_id, agent_id) REFERENCES ent_agent (etablissement_id, id) NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_seance_agent_fk') THEN
        ALTER TABLE ent_edt_seance ADD CONSTRAINT ent_edt_seance_agent_fk
            FOREIGN KEY (etablissement_id, agent_id) REFERENCES ent_agent (etablissement_id, id) NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_seance_remplacant_agent_fk') THEN
        ALTER TABLE ent_edt_seance ADD CONSTRAINT ent_edt_seance_remplacant_agent_fk
            FOREIGN KEY (etablissement_id, remplacant_agent_id) REFERENCES ent_agent (etablissement_id, id) NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_absence_rh_agent_fk') THEN
        ALTER TABLE ent_absence_rh ADD CONSTRAINT ent_absence_rh_agent_fk
            FOREIGN KEY (etablissement_id, agent_id) REFERENCES ent_agent (etablissement_id, id) NOT VALID;
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS ent_edt_creneau_agent_idx ON ent_edt_creneau (etablissement_id, agent_id) WHERE agent_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_edt_seance_agent_idx ON ent_edt_seance (etablissement_id, agent_id, jour) WHERE agent_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_absence_rh_agent_idx ON ent_absence_rh (etablissement_id, agent_id) WHERE agent_id IS NOT NULL;

-- Déclencheurs : `agent_id` suit le compte (un agent posé explicitement à l'insertion est respecté).
CREATE OR REPLACE FUNCTION ent_trg_creneau_agent() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.agent_id IS NULL THEN
            NEW.agent_id := ent_agent_du_compte(NEW.etablissement_id, NEW.prof_user_id);
        END IF;
    ELSIF NEW.prof_user_id IS DISTINCT FROM OLD.prof_user_id AND NEW.agent_id IS NOT DISTINCT FROM OLD.agent_id THEN
        NEW.agent_id := ent_agent_du_compte(NEW.etablissement_id, NEW.prof_user_id);
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_edt_creneau_agent ON ent_edt_creneau;
CREATE TRIGGER ent_edt_creneau_agent BEFORE INSERT OR UPDATE OF prof_user_id ON ent_edt_creneau
    FOR EACH ROW EXECUTE FUNCTION ent_trg_creneau_agent();

CREATE OR REPLACE FUNCTION ent_trg_seance_agent() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.agent_id IS NULL THEN
            NEW.agent_id := ent_agent_du_compte(NEW.etablissement_id, NEW.prof_user_id);
        END IF;
        IF NEW.remplacant_agent_id IS NULL AND NEW.remplacant_user_id IS NOT NULL THEN
            NEW.remplacant_agent_id := ent_agent_du_compte(NEW.etablissement_id, NEW.remplacant_user_id);
        END IF;
    ELSE
        IF NEW.prof_user_id IS DISTINCT FROM OLD.prof_user_id AND NEW.agent_id IS NOT DISTINCT FROM OLD.agent_id THEN
            NEW.agent_id := ent_agent_du_compte(NEW.etablissement_id, NEW.prof_user_id);
        END IF;
        IF NEW.remplacant_user_id IS DISTINCT FROM OLD.remplacant_user_id
           AND NEW.remplacant_agent_id IS NOT DISTINCT FROM OLD.remplacant_agent_id THEN
            NEW.remplacant_agent_id := CASE WHEN NEW.remplacant_user_id IS NULL THEN NULL
                ELSE ent_agent_du_compte(NEW.etablissement_id, NEW.remplacant_user_id) END;
        END IF;
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_edt_seance_agent ON ent_edt_seance;
CREATE TRIGGER ent_edt_seance_agent BEFORE INSERT OR UPDATE OF prof_user_id, remplacant_user_id ON ent_edt_seance
    FOR EACH ROW EXECUTE FUNCTION ent_trg_seance_agent();

CREATE OR REPLACE FUNCTION ent_trg_absence_rh_agent() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.agent_id IS NULL THEN
            NEW.agent_id := ent_agent_du_compte(NEW.etablissement_id, NEW.user_id);
        END IF;
    ELSIF NEW.user_id IS DISTINCT FROM OLD.user_id AND NEW.agent_id IS NOT DISTINCT FROM OLD.agent_id THEN
        NEW.agent_id := ent_agent_du_compte(NEW.etablissement_id, NEW.user_id);
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absence_rh_agent ON ent_absence_rh;
CREATE TRIGGER ent_absence_rh_agent BEFORE INSERT OR UPDATE OF user_id ON ent_absence_rh
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absence_rh_agent();

-- Reprise : l'agent du compte, pour les lignes qui n'en ont pas encore (rejouable).
UPDATE ent_edt_creneau c SET agent_id = ent_agent_du_compte(c.etablissement_id, c.prof_user_id)
 WHERE c.agent_id IS NULL AND ent_agent_du_compte(c.etablissement_id, c.prof_user_id) IS NOT NULL;

UPDATE ent_edt_seance s SET agent_id = ent_agent_du_compte(s.etablissement_id, s.prof_user_id)
 WHERE s.agent_id IS NULL AND ent_agent_du_compte(s.etablissement_id, s.prof_user_id) IS NOT NULL;

UPDATE ent_edt_seance s SET remplacant_agent_id = ent_agent_du_compte(s.etablissement_id, s.remplacant_user_id)
 WHERE s.remplacant_agent_id IS NULL AND s.remplacant_user_id IS NOT NULL
   AND ent_agent_du_compte(s.etablissement_id, s.remplacant_user_id) IS NOT NULL;

UPDATE ent_absence_rh r SET agent_id = ent_agent_du_compte(r.etablissement_id, r.user_id)
 WHERE r.agent_id IS NULL AND ent_agent_du_compte(r.etablissement_id, r.user_id) IS NOT NULL;
