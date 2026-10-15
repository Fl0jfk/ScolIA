-- Journal des modifications métier (audit lisible staff / famille).

CREATE TABLE IF NOT EXISTS ent_journal_modif (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID REFERENCES ent_eleve (id),
    domaine TEXT NOT NULL,
    action TEXT NOT NULL,
    libelle TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    visible_famille BOOLEAN NOT NULL DEFAULT false,
    actor_user_id UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_journal_modif_domaine_chk CHECK (char_length(trim(domaine)) >= 1),
    CONSTRAINT ent_journal_modif_action_chk CHECK (char_length(trim(action)) >= 1),
    CONSTRAINT ent_journal_modif_libelle_chk CHECK (char_length(trim(libelle)) >= 1)
);

CREATE INDEX IF NOT EXISTS ent_journal_modif_eleve_idx
    ON ent_journal_modif (etablissement_id, eleve_id, created_at DESC);

CREATE INDEX IF NOT EXISTS ent_journal_modif_etab_idx
    ON ent_journal_modif (etablissement_id, created_at DESC);
