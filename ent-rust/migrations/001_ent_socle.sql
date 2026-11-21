-- Socle ENT Rust (préfixe ent_ pour cohabiter avec ScolIA sur le même Postgres local).
-- À jouer UNIQUEMENT sur 127.0.0.1 — jamais prod.

CREATE TABLE IF NOT EXISTS ent_eleve (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    nom TEXT NOT NULL,
    prenom TEXT NOT NULL,
    ine TEXT,
    status TEXT NOT NULL,
    classe TEXT,
    regime TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS ent_eleve_etab_idx
    ON ent_eleve (etablissement_id);

CREATE INDEX IF NOT EXISTS ent_eleve_etab_id_idx
    ON ent_eleve (etablissement_id, id);

-- Effectif de classe = cette table, pas une projection séparée.
CREATE INDEX IF NOT EXISTS ent_eleve_etab_classe_idx
    ON ent_eleve (etablissement_id, classe)
    WHERE status = 'actif';

CREATE TABLE IF NOT EXISTS ent_metier_event (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    type TEXT NOT NULL,
    aggregate TEXT NOT NULL,
    aggregate_id UUID NOT NULL,
    eleve_id UUID,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    actor_user_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS ent_metier_event_etab_created_idx
    ON ent_metier_event (etablissement_id, created_at);

CREATE INDEX IF NOT EXISTS ent_metier_event_eleve_idx
    ON ent_metier_event (etablissement_id, eleve_id);
