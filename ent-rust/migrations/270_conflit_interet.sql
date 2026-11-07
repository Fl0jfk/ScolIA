-- Socle 4, lot A1 (R-A18) : conflit d'intérêts d'un agent vis-à-vis d'un élève.
-- La détection automatique (l'agent est lui-même responsable, contact ou payeur de l'élève) se
-- fait en lecture ; cette table porte la DÉCLARATION (l'agent ou la direction) pour les cas que
-- le schéma ne voit pas : ex-conjoint, ancien membre du foyer, proche. Jamais de suppression :
-- une déclaration se lève (date et auteur). Migration additive et rejouable.

CREATE TABLE IF NOT EXISTS ent_conflit_interet (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    user_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id) ON DELETE CASCADE,
    nature TEXT NOT NULL CHECK (nature IN ('ex_conjoint', 'ancien_foyer', 'proche', 'autre')),
    origine TEXT NOT NULL CHECK (origine IN ('agent', 'direction')),
    declare_par UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    leve_le TIMESTAMPTZ,
    leve_par UUID,
    CONSTRAINT ent_conflit_interet_leve_chk CHECK ((leve_le IS NULL) = (leve_par IS NULL))
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_conflit_interet_actif_uq
    ON ent_conflit_interet (etablissement_id, user_id, eleve_id) WHERE leve_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_conflit_interet_eleve_idx
    ON ent_conflit_interet (etablissement_id, eleve_id) WHERE leve_le IS NULL;
