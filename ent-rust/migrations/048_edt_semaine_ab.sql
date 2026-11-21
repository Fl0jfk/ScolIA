-- Alternance semaines A/B : calendrier établissement + filtre sur créneaux.

ALTER TABLE ent_edt_creneau
    ADD COLUMN IF NOT EXISTS semaine_ab TEXT NOT NULL DEFAULT 'toutes';

ALTER TABLE ent_edt_creneau
    DROP CONSTRAINT IF EXISTS ent_edt_creneau_semaine_ab_chk;

ALTER TABLE ent_edt_creneau
    ADD CONSTRAINT ent_edt_creneau_semaine_ab_chk
        CHECK (semaine_ab IN ('toutes', 'a', 'b'));

CREATE TABLE IF NOT EXISTS ent_edt_calendrier_ab (
    etablissement_id UUID PRIMARY KEY,
    -- Lundi de référence = début d'une semaine A.
    reference_lundi DATE NOT NULL,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
