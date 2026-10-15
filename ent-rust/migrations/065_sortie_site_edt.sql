-- Sortie changement de site (EDT) + param établissement.

ALTER TABLE ent_edt_creneau
    ADD COLUMN IF NOT EXISTS site TEXT NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS ent_sortie_param (
    etablissement_id UUID PRIMARY KEY,
    autoriser_changement_site BOOLEAN NOT NULL DEFAULT false,
    updated_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE ent_sortie_controle
    DROP CONSTRAINT IF EXISTS ent_sortie_controle_source_chk;

ALTER TABLE ent_sortie_controle
    ADD CONSTRAINT ent_sortie_controle_source_chk
        CHECK (source IN ('permanente', 'exception', 'interdit', 'aucune', 'autre_site'));
