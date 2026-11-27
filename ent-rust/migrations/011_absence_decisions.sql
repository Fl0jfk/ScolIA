-- Issues VS d'une absence : accepter / refuser le justif, ou annuler (erreur d'appel → présent).

ALTER TABLE ent_absence DROP CONSTRAINT IF EXISTS ent_absence_statut_chk;

ALTER TABLE ent_absence
    ADD CONSTRAINT ent_absence_statut_chk
    CHECK (statut IN ('a_traiter', 'justifiee', 'refusee', 'annulee_present'));

ALTER TABLE ent_absence
    ADD COLUMN IF NOT EXISTS decision_note TEXT;

ALTER TABLE ent_absence
    ADD COLUMN IF NOT EXISTS decision_par_user_id UUID;

ALTER TABLE ent_absence
    ADD COLUMN IF NOT EXISTS decision_at TIMESTAMPTZ;
