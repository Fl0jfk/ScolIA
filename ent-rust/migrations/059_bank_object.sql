-- Rapprochement banque (relevé → SEPA) + clé objet pour pièces PAI.

CREATE TABLE IF NOT EXISTS ent_bank_rapprochement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    end_to_end_id TEXT NOT NULL,
    montant_centimes BIGINT NOT NULL,
    result TEXT NOT NULL,
    motif TEXT,
    prelevement_id UUID,
    action TEXT NOT NULL,
    status TEXT NOT NULL,
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_bank_rapprochement_result_chk CHECK (
        result IN ('settled', 'rejected')
    ),
    CONSTRAINT ent_bank_rapprochement_action_chk CHECK (
        action IN ('execute', 'reject', 'skip')
    ),
    CONSTRAINT ent_bank_rapprochement_status_chk CHECK (
        status IN ('applied', 'unmatched', 'error', 'dry_run')
    )
);

CREATE INDEX IF NOT EXISTS ent_bank_rapprochement_etab_idx
    ON ent_bank_rapprochement (etablissement_id, created_at DESC);

CREATE INDEX IF NOT EXISTS ent_bank_rapprochement_e2e_idx
    ON ent_bank_rapprochement (etablissement_id, end_to_end_id);

ALTER TABLE ent_pai_piece
    ADD COLUMN IF NOT EXISTS storage_key TEXT;

-- Contenu peut rester en base (legacy) OU être vidé si storage_key renseigné.
ALTER TABLE ent_pai_piece
    ALTER COLUMN contenu DROP NOT NULL;

ALTER TABLE ent_pai_piece
    ALTER COLUMN contenu SET DEFAULT '\x'::bytea;
