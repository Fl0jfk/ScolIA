-- Modèles de messages staff + archive des fils (boîte établissement).

CREATE TABLE IF NOT EXISTS ent_msg_modele (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    titre TEXT NOT NULL,
    sujet TEXT NOT NULL,
    corps TEXT NOT NULL,
    actif BOOLEAN NOT NULL DEFAULT true,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS ent_msg_modele_etab_idx
    ON ent_msg_modele (etablissement_id, actif, titre);

ALTER TABLE ent_msg_fil
    ADD COLUMN IF NOT EXISTS archive BOOLEAN NOT NULL DEFAULT false;

ALTER TABLE ent_msg_fil
    ADD COLUMN IF NOT EXISTS archived_at TIMESTAMPTZ;

ALTER TABLE ent_msg_fil
    ADD COLUMN IF NOT EXISTS modele_id UUID REFERENCES ent_msg_modele (id);

CREATE INDEX IF NOT EXISTS ent_msg_fil_actifs_idx
    ON ent_msg_fil (etablissement_id, last_message_at DESC)
    WHERE archive = false;
