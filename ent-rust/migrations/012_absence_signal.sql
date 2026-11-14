-- Signal VS : absences répétées (seuil métier, une file ouverte par élève / année).

CREATE TABLE IF NOT EXISTS ent_absence_signal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    annee_scolaire TEXT NOT NULL,
    seuil INTEGER NOT NULL,
    compteur INTEGER NOT NULL,
    statut TEXT NOT NULL DEFAULT 'ouvert',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absence_signal_statut_chk CHECK (statut IN ('ouvert', 'clos')),
    CONSTRAINT ent_absence_signal_eleve_annee_uq UNIQUE (etablissement_id, eleve_id, annee_scolaire)
);

CREATE INDEX IF NOT EXISTS ent_absence_signal_ouverts_idx
    ON ent_absence_signal (etablissement_id, statut)
    WHERE statut = 'ouvert';
