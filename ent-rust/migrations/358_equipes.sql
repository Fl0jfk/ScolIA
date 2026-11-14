-- Personnels, lot P5 (06_PERSONNELS.md §8.8, PRS-70 à 75) : ÉQUIPES CONSTITUÉES.
-- Les équipes pédagogiques (classe, groupe, niveau, discipline, élève) sont CALCULÉES (aucune table de
-- membres : voir `personnels_equipes.rs`). Seules les équipes constituées (conseil pédagogique, cellule de
-- veille, équipe éducative autour d'un élève, équipe projet…) ont des tables.
-- UNE ÉQUIPE NE DONNE AUCUN DROIT (Q-P9) : aucun lecteur de droits ne lit ces tables ; un test le garde.
-- Migration additive, rejouable, sans JSON ni jour UTC.

CREATE TABLE IF NOT EXISTS ent_equipe_type (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    etablissement_id UUID
        CHECK (etablissement_id IS NULL OR etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    code TEXT NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{1,59}$'),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    -- Type qui exige un élève pour objet (équipe éducative) : confidentielle d'office.
    objet_eleve BOOLEAN NOT NULL DEFAULT false,
    actif BOOLEAN NOT NULL DEFAULT true,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_equipe_type_code_uq
    ON ent_equipe_type (coalesce(etablissement_id, '00000000-0000-0000-0000-000000000000'::uuid), code);

INSERT INTO ent_equipe_type (etablissement_id, code, libelle, objet_eleve)
SELECT NULL, v.code, v.libelle, v.objet_eleve
  FROM (VALUES
    ('conseil_pedagogique', 'Conseil pédagogique', false),
    ('cellule_veille', 'Cellule de veille', false),
    ('equipe_educative_eleve', 'Équipe éducative autour d''un élève', true),
    ('equipe_projet', 'Équipe projet', false),
    ('equipe_direction', 'Équipe de direction', false),
    ('equipe_disciplinaire_elargie', 'Équipe disciplinaire élargie', false),
    ('portes_ouvertes', 'Portes ouvertes', false),
    ('autre', 'Autre équipe', false)
  ) AS v(code, libelle, objet_eleve)
 WHERE NOT EXISTS (SELECT 1 FROM ent_equipe_type t WHERE t.etablissement_id IS NULL AND t.code = v.code);

CREATE TABLE IF NOT EXISTS ent_equipe (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    type_id UUID NOT NULL REFERENCES ent_equipe_type (id),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 160),
    annee_id UUID,
    du DATE NOT NULL,
    au DATE,
    -- Équipe éducative ou de suivi autour d'un élève (cas P46).
    objet_eleve_id UUID,
    liste_diffusion BOOLEAN NOT NULL DEFAULT false,
    visible_annuaire BOOLEAN NOT NULL DEFAULT false,
    -- Liste visible des seuls membres et de la direction.
    confidentielle BOOLEAN NOT NULL DEFAULT false,
    clos_le TIMESTAMPTZ,
    clos_par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_equipe_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_equipe_etab_id_uq UNIQUE (etablissement_id, id),
    -- Une équipe autour d'un élève est toujours confidentielle et jamais dans l'annuaire.
    CONSTRAINT ent_equipe_eleve_confidentielle_chk CHECK (
        objet_eleve_id IS NULL OR (confidentielle AND NOT visible_annuaire))
);
CREATE INDEX IF NOT EXISTS ent_equipe_etab_idx ON ent_equipe (etablissement_id, du);
CREATE INDEX IF NOT EXISTS ent_equipe_eleve_idx ON ent_equipe (etablissement_id, objet_eleve_id)
    WHERE objet_eleve_id IS NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_equipe_annee_fk') THEN
        ALTER TABLE ent_equipe ADD CONSTRAINT ent_equipe_annee_fk
            FOREIGN KEY (annee_id, etablissement_id) REFERENCES ent_annee_scolaire (id, etablissement_id) NOT VALID;
    END IF;
END
$$;

CREATE TABLE IF NOT EXISTS ent_equipe_membre (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    equipe_id UUID NOT NULL,
    agent_id UUID,
    -- Une famille (ou autre personne) peut être invitée à une équipe éducative (socle 19).
    personne_id UUID,
    role_dans_equipe TEXT NOT NULL DEFAULT 'membre'
        CHECK (role_dans_equipe IN ('coordinateur', 'membre', 'invite')),
    du DATE NOT NULL,
    au DATE,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_equipe_membre_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_equipe_membre_qui_chk CHECK ((agent_id IS NOT NULL) <> (personne_id IS NOT NULL)),
    -- Une personne extérieure (famille) est toujours « invitée » ; un agent ne l'est jamais.
    CONSTRAINT ent_equipe_membre_invite_chk CHECK ((personne_id IS NOT NULL) = (role_dans_equipe = 'invite')),
    CONSTRAINT ent_equipe_membre_equipe_fk FOREIGN KEY (etablissement_id, equipe_id)
        REFERENCES ent_equipe (etablissement_id, id),
    CONSTRAINT ent_equipe_membre_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id),
    CONSTRAINT ent_equipe_membre_sans_chevauchement EXCLUDE USING gist (
        equipe_id WITH =, agent_id WITH =, daterange(du, au, '[]') WITH &&) WHERE (agent_id IS NOT NULL),
    CONSTRAINT ent_equipe_membre_personne_sans_chevauchement EXCLUDE USING gist (
        equipe_id WITH =, personne_id WITH =, daterange(du, au, '[]') WITH &&) WHERE (personne_id IS NOT NULL)
);
CREATE INDEX IF NOT EXISTS ent_equipe_membre_equipe_idx ON ent_equipe_membre (etablissement_id, equipe_id, du);
CREATE INDEX IF NOT EXISTS ent_equipe_membre_agent_idx ON ent_equipe_membre (etablissement_id, agent_id, du)
    WHERE agent_id IS NOT NULL;

-- Gardes entre établissements : le type est fourni ou de l'établissement ; l'élève objet, la personne invitée
-- et les membres sont de l'établissement ; un membre ne déborde pas la période de l'équipe.
CREATE OR REPLACE FUNCTION ent_trg_equipe() RETURNS trigger AS $$
DECLARE
    t RECORD;
BEGIN
    SELECT etablissement_id, objet_eleve INTO t FROM ent_equipe_type WHERE id = NEW.type_id;
    IF NOT FOUND OR (t.etablissement_id IS NOT NULL AND t.etablissement_id <> NEW.etablissement_id) THEN
        RAISE EXCEPTION 'type d''équipe d''un autre établissement' USING ERRCODE = '23503';
    END IF;
    IF t.objet_eleve AND NEW.objet_eleve_id IS NULL THEN
        RAISE EXCEPTION 'cette équipe exige un élève pour objet' USING ERRCODE = '23514';
    END IF;
    IF NEW.objet_eleve_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM ent_eleve e WHERE e.id = NEW.objet_eleve_id AND e.etablissement_id = NEW.etablissement_id) THEN
        RAISE EXCEPTION 'élève d''un autre établissement' USING ERRCODE = '23503';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS ent_equipe_garde ON ent_equipe;
CREATE TRIGGER ent_equipe_garde BEFORE INSERT OR UPDATE OF type_id, objet_eleve_id ON ent_equipe
    FOR EACH ROW EXECUTE FUNCTION ent_trg_equipe();

CREATE OR REPLACE FUNCTION ent_trg_equipe_membre() RETURNS trigger AS $$
BEGIN
    IF NEW.personne_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM ent_personne p WHERE p.id = NEW.personne_id AND p.etablissement_id = NEW.etablissement_id) THEN
        RAISE EXCEPTION 'personne d''un autre établissement' USING ERRCODE = '23503';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM ent_equipe e
                    WHERE e.id = NEW.equipe_id AND e.etablissement_id = NEW.etablissement_id
                      AND e.du <= NEW.du AND (e.au IS NULL OR (NEW.au IS NOT NULL AND NEW.au <= e.au))) THEN
        RAISE EXCEPTION 'membre hors de la période de l''équipe' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS ent_equipe_membre_garde ON ent_equipe_membre;
CREATE TRIGGER ent_equipe_membre_garde BEFORE INSERT OR UPDATE OF equipe_id, personne_id, du, au ON ent_equipe_membre
    FOR EACH ROW EXECUTE FUNCTION ent_trg_equipe_membre();

-- RLS : types fournis (etablissement_id NULL) lisibles de tous, comme `ent_mission_type` (355).
ALTER TABLE ent_equipe_type ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_equipe_type FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_equipe_type;
CREATE POLICY ent_rls_etablissement ON ent_equipe_type FOR ALL
    USING (etablissement_id IS NULL
           OR etablissement_id = (SELECT ent_rls_etablissement())
           OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
                OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));

-- RLS (politique des migrations 100, 480 et 560) sur les deux tables d'établissement.
DO $$
DECLARE
    t text;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_equipe', 'ent_equipe_membre'] LOOP
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
