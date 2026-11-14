-- Invitation / provisionnement compte face élève (traçabilité staff).

CREATE TABLE IF NOT EXISTS ent_eleve_invitation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    email TEXT NOT NULL,
    statut TEXT NOT NULL,
    invited_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    activated_user_id UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT ent_eleve_invitation_statut_chk
        CHECK (statut IN ('provisionne', 'revoquee'))
);

CREATE INDEX IF NOT EXISTS ent_eleve_invitation_etab_eleve_idx
    ON ent_eleve_invitation (etablissement_id, eleve_id);

CREATE INDEX IF NOT EXISTS ent_eleve_invitation_etab_created_idx
    ON ent_eleve_invitation (etablissement_id, created_at DESC);
