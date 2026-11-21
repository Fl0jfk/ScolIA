-- Dispenses Vie scolaire (EPS / piscine / médical…).

CREATE TABLE IF NOT EXISTS ent_dispense_vs (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    motif TEXT NOT NULL,
    debut DATE NOT NULL,
    fin DATE NOT NULL,
    commentaire TEXT NOT NULL DEFAULT '',
    statut TEXT NOT NULL DEFAULT 'active',
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    closed_by_user_id UUID REFERENCES ent_user (id),
    closed_at TIMESTAMPTZ,
    CONSTRAINT ent_dispense_vs_motif_chk
        CHECK (motif IN ('eps', 'piscine', 'sport', 'medical', 'autre')),
    CONSTRAINT ent_dispense_vs_statut_chk
        CHECK (statut IN ('active', 'close')),
    CONSTRAINT ent_dispense_vs_periode_chk
        CHECK (fin >= debut)
);

CREATE INDEX IF NOT EXISTS ent_dispense_vs_eleve_idx
    ON ent_dispense_vs (etablissement_id, eleve_id, debut DESC);

CREATE INDEX IF NOT EXISTS ent_dispense_vs_actives_idx
    ON ent_dispense_vs (etablissement_id, statut, debut)
    WHERE statut = 'active';
