-- Documents à signer (règlement, chartes…) — publication staff + signature parent.

CREATE TABLE IF NOT EXISTS ent_doc_a_signer (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    code TEXT NOT NULL,
    titre TEXT NOT NULL,
    corps TEXT NOT NULL,
    annee_scolaire TEXT NOT NULL,
    statut TEXT NOT NULL DEFAULT 'a_signer',
    publie_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    publie_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    echeance DATE,
    signe_at TIMESTAMPTZ,
    signe_par_user_id UUID REFERENCES ent_user (id),
    CONSTRAINT ent_doc_a_signer_statut_chk
        CHECK (statut IN ('a_signer', 'signe', 'annule')),
    CONSTRAINT ent_doc_a_signer_titre_chk
        CHECK (char_length(trim(titre)) >= 1)
);

CREATE INDEX IF NOT EXISTS ent_doc_a_signer_eleve_idx
    ON ent_doc_a_signer (etablissement_id, eleve_id, statut);

CREATE INDEX IF NOT EXISTS ent_doc_a_signer_pending_idx
    ON ent_doc_a_signer (etablissement_id, statut, publie_at DESC)
    WHERE statut = 'a_signer';
