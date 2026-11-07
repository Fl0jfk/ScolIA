-- Notes P0 : périodes (définition + verrou) + devoirs + rattachement des notes.

CREATE TABLE IF NOT EXISTS ent_note_periode (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    date_debut DATE NOT NULL,
    date_fin DATE NOT NULL,
    -- ouverte = saisie notes OK ; fermee = verrou total ; partielle = notes verrouillées
    statut TEXT NOT NULL DEFAULT 'ouverte',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_note_periode_code_uq UNIQUE (etablissement_id, code),
    CONSTRAINT ent_note_periode_dates_chk CHECK (date_fin >= date_debut),
    CONSTRAINT ent_note_periode_statut_chk CHECK (
        statut IN ('ouverte', 'fermee', 'partielle_appreciations')
    )
);

CREATE INDEX IF NOT EXISTS ent_note_periode_etab_idx
    ON ent_note_periode (etablissement_id);

CREATE TABLE IF NOT EXISTS ent_devoir (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    periode_id UUID NOT NULL REFERENCES ent_note_periode (id),
    matiere_id UUID NOT NULL REFERENCES ent_matiere (id),
    classe TEXT NOT NULL,
    titre TEXT NOT NULL,
    date_devoir DATE NOT NULL,
    coef DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    sur DOUBLE PRECISION NOT NULL DEFAULT 20.0,
    created_by_user_id UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_devoir_coef_chk CHECK (coef > 0),
    CONSTRAINT ent_devoir_sur_chk CHECK (sur > 0)
);

CREATE INDEX IF NOT EXISTS ent_devoir_periode_idx
    ON ent_devoir (etablissement_id, periode_id);

CREATE INDEX IF NOT EXISTS ent_devoir_classe_idx
    ON ent_devoir (etablissement_id, classe);

ALTER TABLE ent_note
    ADD COLUMN IF NOT EXISTS devoir_id UUID REFERENCES ent_devoir (id);

ALTER TABLE ent_note
    ADD COLUMN IF NOT EXISTS periode_id UUID REFERENCES ent_note_periode (id);

CREATE INDEX IF NOT EXISTS ent_note_devoir_idx
    ON ent_note (etablissement_id, devoir_id);

CREATE INDEX IF NOT EXISTS ent_note_periode_idx
    ON ent_note (etablissement_id, periode_id, matiere_id);
