-- Autorisations de sortie : consentement permanent + demandes exceptionnelles + journal contrôle badge.

CREATE TABLE IF NOT EXISTS ent_sortie_autorisation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    matin BOOLEAN NOT NULL DEFAULT false,
    midi BOOLEAN NOT NULL DEFAULT false,
    soir BOOLEAN NOT NULL DEFAULT false,
    interdit_etablissement BOOLEAN NOT NULL DEFAULT false,
    updated_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_autorisation_eleve_uq UNIQUE (etablissement_id, eleve_id)
);

CREATE INDEX IF NOT EXISTS ent_sortie_autorisation_etab_idx
    ON ent_sortie_autorisation (etablissement_id);

CREATE TABLE IF NOT EXISTS ent_sortie_demande (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    jour DATE NOT NULL,
    creneau TEXT NOT NULL,
    motif TEXT NOT NULL,
    statut TEXT NOT NULL DEFAULT 'en_attente',
    demande_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    decided_by_user_id UUID REFERENCES ent_user (id),
    decided_at TIMESTAMPTZ,
    decision_note TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_demande_creneau_chk
        CHECK (creneau IN ('matin', 'midi', 'soir')),
    CONSTRAINT ent_sortie_demande_statut_chk
        CHECK (statut IN ('en_attente', 'acceptee', 'refusee')),
    CONSTRAINT ent_sortie_demande_motif_chk
        CHECK (char_length(trim(motif)) >= 1)
);

CREATE INDEX IF NOT EXISTS ent_sortie_demande_eleve_jour_idx
    ON ent_sortie_demande (etablissement_id, eleve_id, jour);

CREATE INDEX IF NOT EXISTS ent_sortie_demande_statut_idx
    ON ent_sortie_demande (etablissement_id, statut, jour DESC);

CREATE TABLE IF NOT EXISTS ent_sortie_controle (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    jour DATE NOT NULL,
    creneau TEXT NOT NULL,
    verdict TEXT NOT NULL,
    source TEXT NOT NULL,
    motif TEXT NOT NULL DEFAULT '',
    actor_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_controle_creneau_chk
        CHECK (creneau IN ('matin', 'midi', 'soir')),
    CONSTRAINT ent_sortie_controle_verdict_chk
        CHECK (verdict IN ('autorisee', 'refusee')),
    CONSTRAINT ent_sortie_controle_source_chk
        CHECK (source IN ('permanente', 'exception', 'interdit', 'aucune'))
);

CREATE INDEX IF NOT EXISTS ent_sortie_controle_jour_idx
    ON ent_sortie_controle (etablissement_id, jour DESC);

CREATE INDEX IF NOT EXISTS ent_sortie_controle_eleve_idx
    ON ent_sortie_controle (etablissement_id, eleve_id, jour DESC);
