-- Règles d'autorisation de sortie CONFIGURABLE par régime (EXT / DP / INT).

CREATE TABLE IF NOT EXISTS ent_sortie_regle_regime (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    regime TEXT NOT NULL,
    matin_autorise BOOLEAN NOT NULL DEFAULT false,
    midi_autorise BOOLEAN NOT NULL DEFAULT true,
    soir_autorise BOOLEAN NOT NULL DEFAULT true,
    choix_parent_interdit BOOLEAN NOT NULL DEFAULT false,
    updated_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_regle_regime_uq UNIQUE (etablissement_id, regime),
    CONSTRAINT ent_sortie_regle_regime_chk
        CHECK (regime IN ('EXT', 'DP', 'INT'))
);

CREATE INDEX IF NOT EXISTS ent_sortie_regle_regime_etab_idx
    ON ent_sortie_regle_regime (etablissement_id);
