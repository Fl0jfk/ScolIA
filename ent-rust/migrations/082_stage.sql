-- Stages / entreprises / conventions.

CREATE TABLE IF NOT EXISTS ent_entreprise (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    raison_sociale TEXT NOT NULL,
    siret TEXT,
    adresse TEXT NOT NULL DEFAULT '',
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_entreprise_raison_chk CHECK (char_length(raison_sociale) BETWEEN 1 AND 200),
    CONSTRAINT ent_entreprise_siret_chk CHECK (siret IS NULL OR char_length(siret) = 14)
);

CREATE INDEX IF NOT EXISTS ent_entreprise_etab_idx
    ON ent_entreprise (etablissement_id);

CREATE TABLE IF NOT EXISTS ent_convention_stage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    entreprise_id UUID NOT NULL REFERENCES ent_entreprise (id),
    debut DATE NOT NULL,
    fin DATE NOT NULL,
    tuteur_nom TEXT NOT NULL,
    statut TEXT NOT NULL DEFAULT 'active',
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_convention_dates_chk CHECK (fin >= debut),
    CONSTRAINT ent_convention_tuteur_chk CHECK (char_length(tuteur_nom) BETWEEN 1 AND 120),
    CONSTRAINT ent_convention_statut_chk CHECK (statut IN ('active', 'cloturee', 'annulee'))
);

CREATE INDEX IF NOT EXISTS ent_convention_etab_eleve_idx
    ON ent_convention_stage (etablissement_id, eleve_id);

CREATE INDEX IF NOT EXISTS ent_convention_entreprise_idx
    ON ent_convention_stage (etablissement_id, entreprise_id);
