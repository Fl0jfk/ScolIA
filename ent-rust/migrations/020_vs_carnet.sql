-- Carnet de liaison / observations VS (P0) — consultation fiche + accusé famille.

CREATE TABLE IF NOT EXISTS ent_carnet_entree (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    date_entree DATE NOT NULL,
    categorie TEXT NOT NULL,
    titre TEXT NOT NULL,
    corps TEXT NOT NULL,
    visible_famille BOOLEAN NOT NULL DEFAULT true,
    created_by_user_id UUID REFERENCES ent_user (id),
    signe_at TIMESTAMPTZ,
    signe_par_user_id UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_carnet_categorie_chk CHECK (
        categorie IN ('correspondance', 'accompagnement', 'information')
    )
);

CREATE INDEX IF NOT EXISTS ent_carnet_eleve_idx
    ON ent_carnet_entree (etablissement_id, eleve_id, date_entree DESC);

CREATE INDEX IF NOT EXISTS ent_carnet_non_signe_idx
    ON ent_carnet_entree (etablissement_id, eleve_id)
    WHERE visible_famille AND signe_at IS NULL;
