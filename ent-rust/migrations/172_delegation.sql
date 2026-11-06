-- S2-5 (ADM-04) : délégations de droits datées.
-- Une délégation accorde à un personnel, pour une période dont la fin est obligatoire, un
-- ensemble de droits du catalogue (D-017). Le moteur de droits (`droits_effectifs`) la lit à
-- chaque requête : elle cesse de produire effet le lendemain de sa date de fin, sans tâche de
-- fond ni intervention ; `statut` n'est qu'un reflet (révoquée / expirée) pour l'affichage et
-- l'événement `delegation.expiree` (publié une seule fois par le balayage).
-- Migration additive.

CREATE TABLE IF NOT EXISTS ent_delegation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    delegue_user_id UUID NOT NULL REFERENCES ent_user (id),
    delegant_user_id UUID NOT NULL REFERENCES ent_user (id),
    -- [{"action": "edt.gerer", "portee": "etablissement", "cibles": []}, …]
    droits JSONB NOT NULL,
    debut DATE NOT NULL,
    fin DATE NOT NULL,
    motif TEXT NOT NULL DEFAULT '',
    source TEXT NOT NULL DEFAULT 'manuelle',
    -- Remplacement : id de l'exception EDT qui l'a motivée.
    reference_id UUID,
    statut TEXT NOT NULL DEFAULT 'accordee',
    revoquee_le TIMESTAMPTZ,
    revoquee_par_user_id UUID REFERENCES ent_user (id),
    expiree_le TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_delegation_dates_chk CHECK (fin >= debut),
    CONSTRAINT ent_delegation_statut_chk CHECK (statut IN ('accordee', 'revoquee', 'expiree')),
    CONSTRAINT ent_delegation_source_chk CHECK (source IN ('manuelle', 'remplacement')),
    CONSTRAINT ent_delegation_soi_chk CHECK (delegue_user_id <> delegant_user_id)
);

CREATE INDEX IF NOT EXISTS ent_delegation_delegue_idx
    ON ent_delegation (etablissement_id, delegue_user_id, fin);

-- Une délégation de remplacement par exception (idempotence de l'organisation).
CREATE UNIQUE INDEX IF NOT EXISTS ent_delegation_remplacement_uq
    ON ent_delegation (etablissement_id, reference_id, delegue_user_id)
    WHERE source = 'remplacement' AND reference_id IS NOT NULL;
