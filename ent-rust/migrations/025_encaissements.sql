-- Encaissements sur facture émise (centimes). Soldée quand somme >= total.

CREATE TABLE IF NOT EXISTS ent_fac_encaissement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    facture_id UUID NOT NULL REFERENCES ent_fac_facture (id) ON DELETE CASCADE,
    montant_centimes BIGINT NOT NULL,
    moyen TEXT NOT NULL,
    reference TEXT,
    encaisse_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by_user_id UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fac_encaissement_montant_chk CHECK (montant_centimes > 0),
    CONSTRAINT ent_fac_encaissement_moyen_chk CHECK (
        moyen IN ('especes', 'cheque', 'virement', 'cb', 'autre')
    )
);

CREATE INDEX IF NOT EXISTS ent_fac_encaissement_facture_idx
    ON ent_fac_encaissement (etablissement_id, facture_id);

ALTER TABLE ent_fac_facture
    ADD COLUMN IF NOT EXISTS soldee_at TIMESTAMPTZ;
