-- Rappels automatiques d'échéances facture (J-3 / en retard).

CREATE TABLE IF NOT EXISTS ent_fac_rappel (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    foyer_id UUID NOT NULL REFERENCES ent_foyer (id),
    facture_id UUID NOT NULL REFERENCES ent_fac_facture (id) ON DELETE CASCADE,
    echeance_id UUID NOT NULL REFERENCES ent_fac_echeance (id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    canal TEXT NOT NULL DEFAULT 'message',
    corps TEXT NOT NULL,
    fil_id UUID,
    envoye_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by_user_id UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fac_rappel_kind_chk CHECK (kind IN ('j_moins_3', 'en_retard')),
    CONSTRAINT ent_fac_rappel_canal_chk CHECK (canal IN ('interne', 'message')),
    CONSTRAINT ent_fac_rappel_echeance_kind_uq UNIQUE (etablissement_id, echeance_id, kind)
);

CREATE INDEX IF NOT EXISTS ent_fac_rappel_etab_idx
    ON ent_fac_rappel (etablissement_id, envoye_at DESC);

CREATE INDEX IF NOT EXISTS ent_fac_rappel_foyer_idx
    ON ent_fac_rappel (etablissement_id, foyer_id, envoye_at DESC);
