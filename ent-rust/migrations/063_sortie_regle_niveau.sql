-- Règles d'autorisation de sortie CONFIGURABLE par niveau (pas par classe).

CREATE TABLE IF NOT EXISTS ent_sortie_regle_niveau (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    niveau TEXT NOT NULL,
    matin_autorise BOOLEAN NOT NULL DEFAULT false,
    midi_autorise BOOLEAN NOT NULL DEFAULT true,
    soir_autorise BOOLEAN NOT NULL DEFAULT true,
    choix_parent_interdit BOOLEAN NOT NULL DEFAULT false,
    updated_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_regle_niveau_uq UNIQUE (etablissement_id, niveau),
    CONSTRAINT ent_sortie_regle_niveau_chk
        CHECK (char_length(trim(niveau)) >= 1 AND char_length(niveau) <= 32)
);

CREATE INDEX IF NOT EXISTS ent_sortie_regle_niveau_etab_idx
    ON ent_sortie_regle_niveau (etablissement_id);
