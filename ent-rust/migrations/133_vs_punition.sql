-- Socle 2, lot S2-4 : punitions scolaires, DISTINCTES des sanctions disciplinaires
-- (VS-15). Donnée par un enseignant ou la vie scolaire, sans procédure ; la
-- sanction (ent_sanction) relève du chef d'établissement.

CREATE TABLE IF NOT EXISTS ent_punition_type (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    actif BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_punition_type_code_uq UNIQUE (etablissement_id, code),
    CONSTRAINT ent_punition_type_code_chk CHECK (code ~ '^[a-z0-9_]{2,40}$')
);

CREATE TABLE IF NOT EXISTS ent_punition (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    type_id UUID NOT NULL REFERENCES ent_punition_type (id) ON DELETE RESTRICT,
    jour DATE NOT NULL,
    motif TEXT NOT NULL,
    -- Retenue / devoir : date à laquelle la punition doit être effectuée.
    echeance DATE,
    duree_minutes INTEGER,
    incident_id UUID REFERENCES ent_incident (id) ON DELETE SET NULL,
    statut TEXT NOT NULL DEFAULT 'donnee',
    donnee_par_user_id UUID NOT NULL,
    effectuee_at TIMESTAMPTZ,
    annulee_par_user_id UUID,
    annulee_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_punition_statut_chk CHECK (statut IN ('donnee', 'effectuee', 'annulee')),
    CONSTRAINT ent_punition_duree_chk CHECK (duree_minutes IS NULL OR duree_minutes > 0)
);

CREATE INDEX IF NOT EXISTS ent_punition_eleve_idx
    ON ent_punition (etablissement_id, eleve_id, jour DESC);

CREATE INDEX IF NOT EXISTS ent_punition_echeance_idx
    ON ent_punition (etablissement_id, echeance)
    WHERE statut = 'donnee';
