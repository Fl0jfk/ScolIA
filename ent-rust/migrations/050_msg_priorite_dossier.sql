-- Priorité + dossier (libellé staff) sur les fils de messagerie.

ALTER TABLE ent_msg_fil
    ADD COLUMN IF NOT EXISTS priorite TEXT NOT NULL DEFAULT 'normale';

ALTER TABLE ent_msg_fil
    DROP CONSTRAINT IF EXISTS ent_msg_fil_priorite_chk;

ALTER TABLE ent_msg_fil
    ADD CONSTRAINT ent_msg_fil_priorite_chk
        CHECK (priorite IN ('normale', 'haute'));

ALTER TABLE ent_msg_fil
    ADD COLUMN IF NOT EXISTS dossier TEXT;

CREATE INDEX IF NOT EXISTS ent_msg_fil_priorite_idx
    ON ent_msg_fil (etablissement_id, priorite, last_message_at DESC)
    WHERE archive = false;

CREATE INDEX IF NOT EXISTS ent_msg_fil_dossier_idx
    ON ent_msg_fil (etablissement_id, dossier)
    WHERE dossier IS NOT NULL AND archive = false;
