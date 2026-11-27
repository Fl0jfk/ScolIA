-- D-022 : un créneau d'emploi du temps peut être rattaché à un GROUPE (LV2 allemand,
-- option, atelier…) du référentiel. Sans groupe : le créneau concerne la classe entière.
-- Le groupe est du paramétrage libre de l'établissement (aucune liste dans le code).
-- Nullable : tous les créneaux existants restent « classe entière ».
ALTER TABLE ent_edt_creneau
    ADD COLUMN IF NOT EXISTS groupe_id UUID REFERENCES ent_groupe (id);

CREATE INDEX IF NOT EXISTS ent_edt_creneau_groupe_idx
    ON ent_edt_creneau (etablissement_id, groupe_id)
    WHERE groupe_id IS NOT NULL;
