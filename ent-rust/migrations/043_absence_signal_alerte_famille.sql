-- Alerte famille au seuil d'absences : un fil messagerie unique par signal.
-- Pas de SMTP ici — canal = messagerie ENT (idempotent via fil_alerte_id).

ALTER TABLE ent_absence_signal
    ADD COLUMN IF NOT EXISTS fil_alerte_id UUID REFERENCES ent_msg_fil (id);

CREATE UNIQUE INDEX IF NOT EXISTS ent_absence_signal_fil_alerte_uq
    ON ent_absence_signal (fil_alerte_id)
    WHERE fil_alerte_id IS NOT NULL;
