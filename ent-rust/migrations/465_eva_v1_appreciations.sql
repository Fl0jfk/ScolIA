-- Socle 21, lot V1 (suite) — 465 : appréciations (types, versions, brouillons), historique en ajout
-- seul, banque de phrases du professeur (clé personne × établissement, R-V35). Additif, rejouable.
-- Aucune colonne JSON. `eva_appreciation` succède à `ent_note_appreciation` (056).

CREATE TABLE IF NOT EXISTS eva_appreciation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    periode_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('cours', 'generale', 'vie_scolaire', 'intermediaire', 'dispositif')),
    -- Obligatoire pour une appréciation de cours ou intermédiaire ; interdit sinon.
    cours_id UUID,
    texte TEXT NOT NULL CHECK (char_length(btrim(texte)) BETWEEN 1 AND 2000),
    -- Un brouillon n'est jamais imprimé ni montré à un autre que son auteur (R-V19).
    etat TEXT NOT NULL DEFAULT 'brouillon' CHECK (etat IN ('brouillon', 'validee')),
    aide_ia BOOLEAN NOT NULL DEFAULT false,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    auteur UUID NOT NULL,
    origine_appreciation_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_appreciation_cours_chk CHECK ((type IN ('cours', 'intermediaire')) = (cours_id IS NOT NULL)),
    CONSTRAINT eva_appreciation_periode_fk FOREIGN KEY (periode_id, etablissement_id)
        REFERENCES ent_periode (id, etablissement_id),
    CONSTRAINT eva_appreciation_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id),
    CONSTRAINT eva_appreciation_cours_fk FOREIGN KEY (cours_id, etablissement_id)
        REFERENCES ent_cours (id, etablissement_id),
    CONSTRAINT eva_appreciation_id_etab_uq UNIQUE (id, etablissement_id)
);
-- Une appréciation de cours par (période, élève, cours) ; une seule de chaque autre type.
CREATE UNIQUE INDEX IF NOT EXISTS eva_appreciation_cours_uq
    ON eva_appreciation (etablissement_id, periode_id, eleve_id, type, cours_id) WHERE cours_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS eva_appreciation_type_uq
    ON eva_appreciation (etablissement_id, periode_id, eleve_id, type) WHERE cours_id IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS eva_appreciation_origine_uq
    ON eva_appreciation (etablissement_id, origine_appreciation_id) WHERE origine_appreciation_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS eva_appreciation_historique (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    appreciation_id UUID NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    texte TEXT NOT NULL,
    etat TEXT NOT NULL,
    aide_ia BOOLEAN NOT NULL,
    auteur UUID NOT NULL,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_appreciation_histo_fk FOREIGN KEY (appreciation_id, etablissement_id)
        REFERENCES eva_appreciation (id, etablissement_id),
    CONSTRAINT eva_appreciation_histo_uq UNIQUE (appreciation_id, version)
);

CREATE OR REPLACE FUNCTION ent_trg_eva_appreciation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        -- Un brouillon se jette ; une appréciation validée ne disparaît jamais (R-V6).
        IF OLD.etat = 'validee' THEN
            RAISE EXCEPTION 'eva_appreciation : une appréciation validée ne se supprime pas' USING ERRCODE = '23514';
        END IF;
        RETURN OLD;
    END IF;
    IF NEW.periode_id IS DISTINCT FROM OLD.periode_id OR NEW.eleve_id IS DISTINCT FROM OLD.eleve_id
       OR NEW.type IS DISTINCT FROM OLD.type OR NEW.cours_id IS DISTINCT FROM OLD.cours_id THEN
        RAISE EXCEPTION 'eva_appreciation : la clé d''une appréciation ne change pas' USING ERRCODE = '23514';
    END IF;
    IF NEW.texte IS NOT DISTINCT FROM OLD.texte AND NEW.etat IS NOT DISTINCT FROM OLD.etat
       AND NEW.aide_ia IS NOT DISTINCT FROM OLD.aide_ia THEN
        RETURN OLD;
    END IF;
    NEW.version := OLD.version + 1;
    NEW.maj_le := now();
    RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION ent_trg_eva_appreciation_historique() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.version = OLD.version THEN
        RETURN NULL;
    END IF;
    INSERT INTO eva_appreciation_historique (id, etablissement_id, appreciation_id, version, texte, etat, aide_ia, auteur)
    VALUES (gen_random_uuid(), NEW.etablissement_id, NEW.id, NEW.version, NEW.texte, NEW.etat, NEW.aide_ia, NEW.auteur);
    RETURN NULL;
END
$$;

DROP TRIGGER IF EXISTS eva_appreciation_garde ON eva_appreciation;
CREATE TRIGGER eva_appreciation_garde BEFORE UPDATE OR DELETE ON eva_appreciation
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_appreciation();
DROP TRIGGER IF EXISTS eva_appreciation_histo ON eva_appreciation;
CREATE TRIGGER eva_appreciation_histo AFTER INSERT OR UPDATE ON eva_appreciation
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_appreciation_historique();
DROP TRIGGER IF EXISTS eva_appreciation_histo_ajout_seul ON eva_appreciation_historique;
CREATE TRIGGER eva_appreciation_histo_ajout_seul BEFORE UPDATE OR DELETE ON eva_appreciation_historique
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();

-- Banque de phrases du professeur : rangée par (personne, établissement) ; aucun texte d'un
-- établissement n'est lu dans un autre sans geste de la personne (R-V35, D-013).
CREATE TABLE IF NOT EXISTS eva_banque_phrase (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    user_id UUID NOT NULL,
    texte TEXT NOT NULL CHECK (char_length(btrim(texte)) BETWEEN 1 AND 500),
    etiquettes TEXT CHECK (etiquettes IS NULL OR char_length(etiquettes) <= 200),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_banque_phrase_uq UNIQUE (etablissement_id, user_id, texte)
);

SELECT ent_rls_poser(ARRAY['eva_appreciation', 'eva_appreciation_historique', 'eva_banque_phrase']);
