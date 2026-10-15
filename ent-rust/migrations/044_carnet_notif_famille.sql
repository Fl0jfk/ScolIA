-- Notification famille à la saisie carnet (visible) : fil messagerie unique.

ALTER TABLE ent_carnet_entree
    ADD COLUMN IF NOT EXISTS fil_notif_id UUID REFERENCES ent_msg_fil (id);

CREATE UNIQUE INDEX IF NOT EXISTS ent_carnet_fil_notif_uq
    ON ent_carnet_entree (fil_notif_id)
    WHERE fil_notif_id IS NOT NULL;
