-- Documents élève + grants (verrouillés par défaut) + dispositifs (PAP…).
-- Dictée : droits fermes ; déverrouillage au cas par cas ; pas d'opacité Charlemagne.

CREATE TABLE IF NOT EXISTS ent_eleve_document (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    tiroir TEXT NOT NULL,
    titre TEXT NOT NULL,
    mime_type TEXT,
    storage_key TEXT,
    confidentialite TEXT NOT NULL DEFAULT 'restreint',
    source TEXT NOT NULL DEFAULT 'upload',
    created_by_user_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_document_conf_chk
        CHECK (confidentialite IN ('standard', 'restreint', 'sante')),
    CONSTRAINT ent_eleve_document_tiroir_chk
        CHECK (tiroir IN ('scolaire', 'administratif', 'sante', 'famille', 'autre'))
);

CREATE INDEX IF NOT EXISTS ent_eleve_document_eleve_idx
    ON ent_eleve_document (etablissement_id, eleve_id);

CREATE INDEX IF NOT EXISTS ent_eleve_document_tiroir_idx
    ON ent_eleve_document (etablissement_id, eleve_id, tiroir);

CREATE TABLE IF NOT EXISTS ent_document_grant (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    document_id UUID NOT NULL REFERENCES ent_eleve_document (id) ON DELETE CASCADE,
    grantee_user_id UUID NOT NULL REFERENCES ent_user (id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending',
    duration_days INTEGER NOT NULL DEFAULT 7,
    decided_by_user_id UUID REFERENCES ent_user (id) ON DELETE SET NULL,
    decided_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_document_grant_status_chk
        CHECK (status IN ('pending', 'granted', 'refused', 'expired'))
);

CREATE INDEX IF NOT EXISTS ent_document_grant_doc_idx
    ON ent_document_grant (etablissement_id, document_id);

CREATE INDEX IF NOT EXISTS ent_document_grant_grantee_idx
    ON ent_document_grant (etablissement_id, grantee_user_id, status);

CREATE TABLE IF NOT EXISTS ent_eleve_dispositif (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    kind TEXT NOT NULL,
    libelle TEXT,
    document_id UUID REFERENCES ent_eleve_document (id) ON DELETE SET NULL,
    date_debut DATE NOT NULL,
    date_fin DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_dispositif_kind_chk
        CHECK (kind IN ('pap', 'pps', 'pai', 'gevasco', 'autre'))
);

CREATE INDEX IF NOT EXISTS ent_eleve_dispositif_eleve_idx
    ON ent_eleve_dispositif (etablissement_id, eleve_id);
