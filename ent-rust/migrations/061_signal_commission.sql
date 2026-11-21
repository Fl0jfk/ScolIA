-- Lien signal absences répétées → dossier de commission VS.

ALTER TABLE ent_absence_signal
    ADD COLUMN IF NOT EXISTS commission_dossier_id UUID
        REFERENCES ent_vs_commission_dossier (id);

CREATE UNIQUE INDEX IF NOT EXISTS ent_absence_signal_commission_dossier_uq
    ON ent_absence_signal (commission_dossier_id)
    WHERE commission_dossier_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS ent_absence_signal_commission_idx
    ON ent_absence_signal (etablissement_id, commission_dossier_id)
    WHERE commission_dossier_id IS NOT NULL;
