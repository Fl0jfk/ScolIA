-- Quotient familial foyer + taux TVA établissement.

CREATE TABLE IF NOT EXISTS ent_foyer_quotient (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    foyer_id UUID NOT NULL REFERENCES ent_foyer (id) ON DELETE CASCADE,
    annee_scolaire TEXT NOT NULL,
    quotient_centimes BIGINT NOT NULL,
    parts_x100 INTEGER NOT NULL,
    updated_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_foyer_quotient_parts_chk CHECK (parts_x100 > 0 AND parts_x100 <= 2000),
    CONSTRAINT ent_foyer_quotient_q_chk CHECK (quotient_centimes >= 0),
    CONSTRAINT ent_foyer_quotient_uq UNIQUE (etablissement_id, foyer_id, annee_scolaire)
);

CREATE INDEX IF NOT EXISTS ent_foyer_quotient_foyer_idx
    ON ent_foyer_quotient (etablissement_id, foyer_id);

CREATE TABLE IF NOT EXISTS ent_taux_tva (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    taux_bps INTEGER NOT NULL,
    actif BOOLEAN NOT NULL DEFAULT true,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_taux_tva_bps_chk CHECK (taux_bps >= 0 AND taux_bps <= 10000),
    CONSTRAINT ent_taux_tva_code_uq UNIQUE (etablissement_id, code)
);

CREATE INDEX IF NOT EXISTS ent_taux_tva_etab_idx
    ON ent_taux_tva (etablissement_id)
    WHERE actif = true;
