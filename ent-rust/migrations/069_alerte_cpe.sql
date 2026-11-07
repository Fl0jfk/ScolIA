-- Alertes CPE / Vie scolaire (file d'attention manuelle).

CREATE TABLE IF NOT EXISTS ent_alerte_cpe (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    titre TEXT NOT NULL,
    detail TEXT NOT NULL DEFAULT '',
    severite TEXT NOT NULL DEFAULT 'attention',
    statut TEXT NOT NULL DEFAULT 'ouverte',
    visible_famille BOOLEAN NOT NULL DEFAULT false,
    jour DATE NOT NULL,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    closed_by_user_id UUID REFERENCES ent_user (id),
    closed_at TIMESTAMPTZ,
    cloture_note TEXT NOT NULL DEFAULT '',
    CONSTRAINT ent_alerte_cpe_severite_chk
        CHECK (severite IN ('info', 'attention', 'urgent')),
    CONSTRAINT ent_alerte_cpe_statut_chk
        CHECK (statut IN ('ouverte', 'close')),
    CONSTRAINT ent_alerte_cpe_titre_chk
        CHECK (char_length(trim(titre)) >= 1)
);

CREATE INDEX IF NOT EXISTS ent_alerte_cpe_ouvertes_idx
    ON ent_alerte_cpe (etablissement_id, statut, severite, jour DESC)
    WHERE statut = 'ouverte';

CREATE INDEX IF NOT EXISTS ent_alerte_cpe_eleve_idx
    ON ent_alerte_cpe (etablissement_id, eleve_id, created_at DESC);
