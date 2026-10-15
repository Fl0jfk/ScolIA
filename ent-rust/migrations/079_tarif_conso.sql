-- Tarifs consommation cantine / formules.

CREATE TABLE IF NOT EXISTS ent_tarif_conso (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    montant_centimes BIGINT NOT NULL,
    debut DATE NOT NULL,
    fin DATE,
    actif BOOLEAN NOT NULL DEFAULT true,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_tarif_conso_montant_chk CHECK (montant_centimes >= 0),
    CONSTRAINT ent_tarif_conso_dates_chk CHECK (fin IS NULL OR fin >= debut),
    CONSTRAINT ent_tarif_conso_code_uq UNIQUE (etablissement_id, code, debut)
);

CREATE INDEX IF NOT EXISTS ent_tarif_conso_etab_idx
    ON ent_tarif_conso (etablissement_id, debut)
    WHERE actif = true;
