-- 203 — Règlements manuels sur le compte d'un foyer et lettrage (S2-9, FIN-08).
--
-- Un règlement (chèque, virement, espèces…) est reçu pour un foyer puis imputé
-- sur ses échéances les plus anciennes : une ligne `ent_fac_encaissement` par
-- facture touchée. Le **lettrage** rapproche une facture de ce qui la solde
-- (encaissements, avoirs) sous un code commun.

CREATE TABLE IF NOT EXISTS ent_fin_reglement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    foyer_id UUID NOT NULL REFERENCES ent_foyer (id),
    montant_centimes BIGINT NOT NULL,
    moyen TEXT NOT NULL,
    reference TEXT,
    recu_le DATE NOT NULL,
    -- Part non imputée (trop-perçu) : reste au crédit du foyer.
    non_impute_centimes BIGINT NOT NULL DEFAULT 0,
    created_by UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_reglement_montant_chk CHECK (montant_centimes > 0 AND montant_centimes <= 100000000),
    CONSTRAINT ent_fin_reglement_non_impute_chk CHECK (non_impute_centimes >= 0 AND non_impute_centimes <= montant_centimes),
    CONSTRAINT ent_fin_reglement_moyen_chk CHECK (moyen IN ('especes', 'cheque', 'virement', 'cb', 'autre'))
);

CREATE INDEX IF NOT EXISTS ent_fin_reglement_foyer_idx
    ON ent_fin_reglement (etablissement_id, foyer_id, recu_le DESC);

ALTER TABLE ent_fac_encaissement ADD COLUMN IF NOT EXISTS reglement_id UUID REFERENCES ent_fin_reglement (id);

CREATE TABLE IF NOT EXISTS ent_fin_lettrage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    -- Code de lettrage : posé quand la facture est soldée (NULL = imputation partielle).
    code TEXT,
    facture_id UUID NOT NULL REFERENCES ent_fac_facture (id),
    piece TEXT NOT NULL,
    piece_id UUID NOT NULL,
    montant_centimes BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fin_lettrage_piece_chk CHECK (piece IN ('encaissement', 'avoir')),
    CONSTRAINT ent_fin_lettrage_montant_chk CHECK (montant_centimes > 0),
    CONSTRAINT ent_fin_lettrage_piece_uq UNIQUE (etablissement_id, facture_id, piece, piece_id)
);

CREATE INDEX IF NOT EXISTS ent_fin_lettrage_facture_idx
    ON ent_fin_lettrage (etablissement_id, facture_id);
CREATE INDEX IF NOT EXISTS ent_fin_lettrage_code_idx
    ON ent_fin_lettrage (etablissement_id, code);
