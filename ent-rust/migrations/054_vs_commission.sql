-- Commission VS + dossiers élèves (décision + points retirés pour permis à points).

CREATE TABLE IF NOT EXISTS ent_vs_commission (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    jour DATE NOT NULL,
    heure TIME NOT NULL,
    objet TEXT NOT NULL,
    lieu TEXT NOT NULL DEFAULT '',
    statut TEXT NOT NULL DEFAULT 'planifiee',
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_vs_commission_statut_chk
        CHECK (statut IN ('planifiee', 'tenue', 'annulee'))
);

CREATE INDEX IF NOT EXISTS ent_vs_commission_jour_idx
    ON ent_vs_commission (etablissement_id, jour DESC);

CREATE TABLE IF NOT EXISTS ent_vs_commission_dossier (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    commission_id UUID NOT NULL REFERENCES ent_vs_commission (id),
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    motifs TEXT NOT NULL,
    note_interne TEXT NOT NULL DEFAULT '',
    sanction_id UUID REFERENCES ent_sanction (id) ON DELETE SET NULL,
    incident_id UUID REFERENCES ent_incident (id) ON DELETE SET NULL,
    decision TEXT,
    decision_texte TEXT NOT NULL DEFAULT '',
    points_retires INTEGER NOT NULL DEFAULT 0,
    signale_famille BOOLEAN NOT NULL DEFAULT false,
    decided_at TIMESTAMPTZ,
    decided_by_user_id UUID REFERENCES ent_user (id),
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_vs_commission_dossier_decision_chk
        CHECK (decision IS NULL OR decision IN (
            'avertissement', 'exclusion_temporaire', 'maintien', 'autre'
        )),
    CONSTRAINT ent_vs_commission_dossier_points_chk
        CHECK (points_retires >= 0 AND points_retires <= 20),
    CONSTRAINT ent_vs_commission_dossier_eleve_uq UNIQUE (commission_id, eleve_id)
);

CREATE INDEX IF NOT EXISTS ent_vs_commission_dossier_eleve_idx
    ON ent_vs_commission_dossier (etablissement_id, eleve_id);
