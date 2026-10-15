-- PAI structuré (une fiche par élève) — mesures famille + signature parent.

CREATE TABLE IF NOT EXISTS ent_pai (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    motif TEXT NOT NULL,
    mesures_accueil TEXT NOT NULL DEFAULT '',
    protocole_urgence TEXT NOT NULL DEFAULT '',
    note_interne TEXT NOT NULL DEFAULT '',
    date_debut DATE NOT NULL,
    date_fin DATE,
    actif BOOLEAN NOT NULL DEFAULT true,
    signe_famille_at TIMESTAMPTZ,
    signe_famille_user_id UUID REFERENCES ent_user (id),
    updated_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_pai_eleve_uq UNIQUE (etablissement_id, eleve_id),
    CONSTRAINT ent_pai_dates_chk CHECK (date_fin IS NULL OR date_fin >= date_debut)
);

CREATE INDEX IF NOT EXISTS ent_pai_actif_idx
    ON ent_pai (etablissement_id)
    WHERE actif = true;
