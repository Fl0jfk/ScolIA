-- Planning RDV psychologie scolaire (P0) — créneaux planifiés, distincts des entretiens réalisés.

CREATE TABLE IF NOT EXISTS ent_psy_rdv (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    jour DATE NOT NULL,
    heure_debut TIME NOT NULL,
    heure_fin TIME NOT NULL,
    kind TEXT NOT NULL,
    motif TEXT NOT NULL DEFAULT '',
    note_interne TEXT NOT NULL DEFAULT '',
    statut TEXT NOT NULL DEFAULT 'planifie',
    signale_famille BOOLEAN NOT NULL DEFAULT false,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_psy_rdv_kind_chk
        CHECK (kind IN ('individuel', 'famille', 'orientation', 'crise')),
    CONSTRAINT ent_psy_rdv_statut_chk
        CHECK (statut IN ('planifie', 'honore', 'annule', 'manque')),
    CONSTRAINT ent_psy_rdv_horaires_chk
        CHECK (heure_fin > heure_debut)
);

CREATE INDEX IF NOT EXISTS ent_psy_rdv_jour_idx
    ON ent_psy_rdv (etablissement_id, jour, heure_debut);

CREATE INDEX IF NOT EXISTS ent_psy_rdv_eleve_idx
    ON ent_psy_rdv (etablissement_id, eleve_id, jour DESC);

CREATE INDEX IF NOT EXISTS ent_psy_rdv_actifs_idx
    ON ent_psy_rdv (etablissement_id, statut)
    WHERE statut = 'planifie';
