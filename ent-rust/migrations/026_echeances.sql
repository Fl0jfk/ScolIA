-- Échéancier facture (montants en centimes, dates civiles).

CREATE TABLE IF NOT EXISTS ent_fac_echeance (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    facture_id UUID NOT NULL REFERENCES ent_fac_facture (id) ON DELETE CASCADE,
    rang INT NOT NULL,
    libelle TEXT NOT NULL,
    montant_centimes BIGINT NOT NULL,
    date_echeance DATE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fac_echeance_rang_chk CHECK (rang > 0),
    CONSTRAINT ent_fac_echeance_montant_chk CHECK (montant_centimes > 0),
    CONSTRAINT ent_fac_echeance_rang_uq UNIQUE (etablissement_id, facture_id, rang)
);

CREATE INDEX IF NOT EXISTS ent_fac_echeance_facture_idx
    ON ent_fac_echeance (etablissement_id, facture_id, date_echeance);
