-- Absences personnel / RH.

CREATE TABLE IF NOT EXISTS ent_absence_rh (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    user_id UUID NOT NULL REFERENCES ent_user (id),
    debut DATE NOT NULL,
    fin DATE NOT NULL,
    motif TEXT NOT NULL,
    statut TEXT NOT NULL DEFAULT 'signalee',
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absence_rh_dates_chk CHECK (fin >= debut),
    CONSTRAINT ent_absence_rh_statut_chk CHECK (statut IN ('signalee', 'validee', 'refusee'))
);

CREATE INDEX IF NOT EXISTS ent_absence_rh_etab_idx
    ON ent_absence_rh (etablissement_id, debut, fin);

CREATE INDEX IF NOT EXISTS ent_absence_rh_user_idx
    ON ent_absence_rh (etablissement_id, user_id, debut);
