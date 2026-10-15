-- Vie scolaire — incidents & sanctions (tables CONFIGURABLE établissement).
-- Dictée : saisie VS + consultation aussi sur fiche élève. Commission / permis points = UNKNOWN.

CREATE TABLE IF NOT EXISTS ent_incident_type (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    actif BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_incident_type_code_uq UNIQUE (etablissement_id, code),
    CONSTRAINT ent_incident_type_code_chk CHECK (code ~ '^[a-z0-9_]{2,40}$')
);

CREATE INDEX IF NOT EXISTS ent_incident_type_etab_idx
    ON ent_incident_type (etablissement_id);

CREATE TABLE IF NOT EXISTS ent_sanction_type (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    actif BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sanction_type_code_uq UNIQUE (etablissement_id, code),
    CONSTRAINT ent_sanction_type_code_chk CHECK (code ~ '^[a-z0-9_]{2,40}$')
);

CREATE INDEX IF NOT EXISTS ent_sanction_type_etab_idx
    ON ent_sanction_type (etablissement_id);

CREATE TABLE IF NOT EXISTS ent_sanction_motif (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    actif BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sanction_motif_code_uq UNIQUE (etablissement_id, code),
    CONSTRAINT ent_sanction_motif_code_chk CHECK (code ~ '^[a-z0-9_]{2,40}$')
);

CREATE INDEX IF NOT EXISTS ent_sanction_motif_etab_idx
    ON ent_sanction_motif (etablissement_id);

CREATE TABLE IF NOT EXISTS ent_incident (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    type_id UUID NOT NULL REFERENCES ent_incident_type (id) ON DELETE RESTRICT,
    jour DATE NOT NULL,
    description TEXT,
    created_by_user_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS ent_incident_eleve_idx
    ON ent_incident (etablissement_id, eleve_id, jour DESC);

CREATE TABLE IF NOT EXISTS ent_sanction (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    type_id UUID NOT NULL REFERENCES ent_sanction_type (id) ON DELETE RESTRICT,
    motif_id UUID REFERENCES ent_sanction_motif (id) ON DELETE SET NULL,
    incident_id UUID REFERENCES ent_incident (id) ON DELETE SET NULL,
    jour DATE NOT NULL,
    -- Durée retenue en minutes (NULL si non applicable).
    duree_minutes INTEGER,
    note TEXT,
    statut TEXT NOT NULL DEFAULT 'active',
    created_by_user_id UUID,
    annulee_par_user_id UUID,
    annulee_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sanction_statut_chk
        CHECK (statut IN ('active', 'executee', 'annulee')),
    CONSTRAINT ent_sanction_duree_chk
        CHECK (duree_minutes IS NULL OR duree_minutes > 0)
);

CREATE INDEX IF NOT EXISTS ent_sanction_eleve_idx
    ON ent_sanction (etablissement_id, eleve_id, jour DESC);

CREATE INDEX IF NOT EXISTS ent_sanction_statut_idx
    ON ent_sanction (etablissement_id, statut);
