-- Quittance émise à chaque encaissement (reçu foyer).

CREATE TABLE IF NOT EXISTS ent_fac_quittance (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    foyer_id UUID NOT NULL REFERENCES ent_foyer (id),
    facture_id UUID NOT NULL REFERENCES ent_fac_facture (id) ON DELETE CASCADE,
    encaissement_id UUID NOT NULL REFERENCES ent_fac_encaissement (id) ON DELETE CASCADE,
    numero TEXT NOT NULL,
    montant_centimes BIGINT NOT NULL,
    moyen TEXT NOT NULL,
    emise_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by_user_id UUID REFERENCES ent_user (id),
    CONSTRAINT ent_fac_quittance_montant_chk CHECK (montant_centimes > 0),
    CONSTRAINT ent_fac_quittance_encaissement_uq UNIQUE (encaissement_id),
    CONSTRAINT ent_fac_quittance_numero_uq UNIQUE (etablissement_id, numero)
);

CREATE INDEX IF NOT EXISTS ent_fac_quittance_foyer_idx
    ON ent_fac_quittance (etablissement_id, foyer_id, emise_at DESC);

CREATE INDEX IF NOT EXISTS ent_fac_quittance_facture_idx
    ON ent_fac_quittance (etablissement_id, facture_id);
