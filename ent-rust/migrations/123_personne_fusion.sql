-- Socle 2, lot S2-1 (DOS-15 appliqué aux personnes) : fusion de doublons
-- RÉVERSIBLE. L'instantané garde l'état exact de la personne absorbée (liens,
-- contacts, compte) ; la défusion le restaure. Aucune fusion de comptes (D-013) :
-- deux personnes ayant chacune un compte différent ne sont jamais fusionnées.

CREATE TABLE IF NOT EXISTS ent_personne_fusion (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    source_id UUID NOT NULL REFERENCES ent_personne (id),
    cible_id UUID NOT NULL REFERENCES ent_personne (id),
    instantane JSONB NOT NULL,
    motif TEXT NOT NULL CHECK (length(btrim(motif)) BETWEEN 3 AND 500),
    fusionne_par UUID,
    fusionne_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    annule_par UUID,
    annule_le TIMESTAMPTZ,
    CONSTRAINT ent_personne_fusion_distinctes CHECK (source_id <> cible_id)
);

-- Une personne n'est absorbée qu'une fois à la fois.
CREATE UNIQUE INDEX IF NOT EXISTS ent_personne_fusion_active_uq
    ON ent_personne_fusion (source_id) WHERE annule_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_personne_fusion_etab_idx
    ON ent_personne_fusion (etablissement_id, fusionne_le DESC);
