-- Ventilation de la déduction bourse sur chaque échéance (centimes ≥ 0).

ALTER TABLE ent_fac_echeance
    ADD COLUMN IF NOT EXISTS bourse_centimes BIGINT NOT NULL DEFAULT 0;

ALTER TABLE ent_fac_echeance
    DROP CONSTRAINT IF EXISTS ent_fac_echeance_bourse_chk;

ALTER TABLE ent_fac_echeance
    ADD CONSTRAINT ent_fac_echeance_bourse_chk CHECK (bourse_centimes >= 0);
