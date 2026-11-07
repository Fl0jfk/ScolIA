-- Suivi étapes inscription + historique IBAN foyer.

CREATE TABLE IF NOT EXISTS ent_inscription_suivi (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id) ON DELETE CASCADE,
    annee_scolaire TEXT NOT NULL,
    etape TEXT NOT NULL,
    motif_code TEXT,
    commentaire TEXT NOT NULL DEFAULT '',
    updated_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_suivi_etape_chk
        CHECK (etape IN ('dossier', 'pieces', 'entretien', 'decision', 'paiement', 'finalise')),
    CONSTRAINT ent_inscription_suivi_uq UNIQUE (etablissement_id, eleve_id, annee_scolaire)
);

CREATE INDEX IF NOT EXISTS ent_inscription_suivi_etape_idx
    ON ent_inscription_suivi (etablissement_id, etape, annee_scolaire);

CREATE TABLE IF NOT EXISTS ent_foyer_iban_histo (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    foyer_id UUID NOT NULL REFERENCES ent_foyer (id) ON DELETE CASCADE,
    iban TEXT NOT NULL,
    bic TEXT,
    titulaire TEXT NOT NULL DEFAULT '',
    mandat_id UUID,
    created_by_user_id UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS ent_foyer_iban_histo_foyer_idx
    ON ent_foyer_iban_histo (etablissement_id, foyer_id, created_at DESC);
