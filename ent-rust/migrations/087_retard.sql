-- Retards (fait de vie scolaire canonique, distinct de l'absence).
-- Persistants : historique élève + compteurs + lecture famille.
-- N'alimentent PAS le compteur d'absences.
CREATE TABLE IF NOT EXISTS ent_retard (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    jour DATE NOT NULL,
    minutes INTEGER,
    motif TEXT,
    source_appel_id UUID,
    auteur_user_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_retard_minutes_chk CHECK (minutes IS NULL OR (minutes >= 0 AND minutes <= 600))
);

CREATE INDEX IF NOT EXISTS ent_retard_eleve_jour_idx
    ON ent_retard (etablissement_id, eleve_id, jour DESC);

-- Idempotence : un même appel ne crée qu'un retard par élève.
CREATE UNIQUE INDEX IF NOT EXISTS ent_retard_appel_eleve_uniq
    ON ent_retard (source_appel_id, eleve_id)
    WHERE source_appel_id IS NOT NULL;
