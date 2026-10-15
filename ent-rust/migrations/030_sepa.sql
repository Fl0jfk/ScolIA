-- Mandats SEPA foyer + prélèvements planifiés (P1 : CSV banque, pas pain.008).

ALTER TABLE ent_fac_encaissement
    DROP CONSTRAINT IF EXISTS ent_fac_encaissement_moyen_chk;

ALTER TABLE ent_fac_encaissement
    ADD CONSTRAINT ent_fac_encaissement_moyen_chk CHECK (
        moyen IN ('especes', 'cheque', 'virement', 'cb', 'prelevement', 'autre')
    );

CREATE TABLE IF NOT EXISTS ent_fac_sepa_mandat (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    foyer_id UUID NOT NULL REFERENCES ent_foyer (id),
    rum TEXT NOT NULL,
    iban TEXT NOT NULL,
    bic TEXT,
    titulaire TEXT NOT NULL,
    statut TEXT NOT NULL DEFAULT 'actif',
    signe_at TIMESTAMPTZ NOT NULL,
    created_by_user_id UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fac_sepa_mandat_statut_chk CHECK (statut IN ('actif', 'revoke')),
    CONSTRAINT ent_fac_sepa_mandat_rum_uq UNIQUE (etablissement_id, rum)
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_fac_sepa_mandat_actif_foyer_uq
    ON ent_fac_sepa_mandat (etablissement_id, foyer_id)
    WHERE statut = 'actif';

CREATE INDEX IF NOT EXISTS ent_fac_sepa_mandat_foyer_idx
    ON ent_fac_sepa_mandat (etablissement_id, foyer_id);

CREATE TABLE IF NOT EXISTS ent_fac_sepa_prelevement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    mandat_id UUID NOT NULL REFERENCES ent_fac_sepa_mandat (id),
    facture_id UUID NOT NULL REFERENCES ent_fac_facture (id) ON DELETE CASCADE,
    foyer_id UUID NOT NULL REFERENCES ent_foyer (id),
    montant_centimes BIGINT NOT NULL,
    date_execution DATE NOT NULL,
    end_to_end_id TEXT NOT NULL,
    statut TEXT NOT NULL DEFAULT 'prevu',
    encaissement_id UUID REFERENCES ent_fac_encaissement (id),
    created_by_user_id UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fac_sepa_prelevement_montant_chk CHECK (montant_centimes > 0),
    CONSTRAINT ent_fac_sepa_prelevement_statut_chk CHECK (
        statut IN ('prevu', 'exporte', 'execute', 'annule', 'rejete')
    ),
    CONSTRAINT ent_fac_sepa_prelevement_e2e_uq UNIQUE (etablissement_id, end_to_end_id)
);

CREATE INDEX IF NOT EXISTS ent_fac_sepa_prelevement_facture_idx
    ON ent_fac_sepa_prelevement (etablissement_id, facture_id);

CREATE INDEX IF NOT EXISTS ent_fac_sepa_prelevement_lot_idx
    ON ent_fac_sepa_prelevement (etablissement_id, statut, date_execution);
