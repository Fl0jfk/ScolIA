-- Vacances / jours fermés établissement.

CREATE TABLE IF NOT EXISTS ent_periode_vacances (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    libelle TEXT NOT NULL,
    kind TEXT NOT NULL,
    debut DATE NOT NULL,
    fin DATE NOT NULL,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_periode_vacances_kind_chk
        CHECK (kind IN ('vacances', 'ferie', 'fermeture')),
    CONSTRAINT ent_periode_vacances_dates_chk CHECK (fin >= debut)
);

CREATE INDEX IF NOT EXISTS ent_periode_vacances_etab_idx
    ON ent_periode_vacances (etablissement_id, debut);
