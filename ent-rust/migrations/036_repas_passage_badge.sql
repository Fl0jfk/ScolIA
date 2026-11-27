-- Passage badge cantine / activité repas (pointage jour × activité).
-- Un passage actif par élève / activité / jour (soft-annulation via annule).

CREATE TABLE IF NOT EXISTS ent_repas_passage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    activite_id UUID NOT NULL REFERENCES ent_repas_activite (id) ON DELETE RESTRICT,
    jour DATE NOT NULL,
    badge_lu TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'badge',
    annule BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    actor_user_id TEXT,
    CONSTRAINT ent_repas_passage_source_chk
        CHECK (source IN ('badge', 'manuel')),
    CONSTRAINT ent_repas_passage_badge_chk
        CHECK (char_length(trim(badge_lu)) > 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_repas_passage_actif_uq
    ON ent_repas_passage (etablissement_id, eleve_id, activite_id, jour)
    WHERE annule = false;

CREATE INDEX IF NOT EXISTS ent_repas_passage_jour_idx
    ON ent_repas_passage (etablissement_id, jour)
    WHERE annule = false;

CREATE INDEX IF NOT EXISTS ent_repas_passage_eleve_idx
    ON ent_repas_passage (etablissement_id, eleve_id, jour);
