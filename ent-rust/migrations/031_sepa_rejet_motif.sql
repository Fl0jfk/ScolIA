-- Motif de rejet banque sur prélèvement SEPA.

ALTER TABLE ent_fac_sepa_prelevement
    ADD COLUMN IF NOT EXISTS motif_rejet TEXT;

ALTER TABLE ent_fac_sepa_prelevement
    ADD COLUMN IF NOT EXISTS rejete_at TIMESTAMPTZ;
