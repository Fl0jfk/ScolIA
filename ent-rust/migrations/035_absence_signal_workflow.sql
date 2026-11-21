-- Workflow signal VS : prise en charge CPE + clôture avec note.
-- Statuts : ouvert → en_cours → clos (mail / commission restent hors scope).

ALTER TABLE ent_absence_signal
    DROP CONSTRAINT IF EXISTS ent_absence_signal_statut_chk;

ALTER TABLE ent_absence_signal
    ADD CONSTRAINT ent_absence_signal_statut_chk
        CHECK (statut IN ('ouvert', 'en_cours', 'clos'));

ALTER TABLE ent_absence_signal
    ADD COLUMN IF NOT EXISTS note_vs TEXT;

ALTER TABLE ent_absence_signal
    ADD COLUMN IF NOT EXISTS traite_par_user_id UUID;

ALTER TABLE ent_absence_signal
    ADD COLUMN IF NOT EXISTS traite_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS ent_absence_signal_actifs_idx
    ON ent_absence_signal (etablissement_id, statut)
    WHERE statut IN ('ouvert', 'en_cours');
