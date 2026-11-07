-- Historique daté élève (dates d'effet) — vérité pour appel / cantine / facture.
-- Le jeudi n'écrase pas le mardi : périodes [date_debut, date_fin], date_fin NULL = ouverte.

ALTER TABLE ent_eleve
    ADD COLUMN IF NOT EXISTS date_entree DATE;

ALTER TABLE ent_eleve
    ADD COLUMN IF NOT EXISTS date_sortie DATE;

ALTER TABLE ent_eleve
    ADD COLUMN IF NOT EXISTS motif_sortie TEXT;

CREATE TABLE IF NOT EXISTS ent_eleve_periode (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    kind TEXT NOT NULL,
    valeur TEXT NOT NULL,
    date_debut DATE NOT NULL,
    date_fin DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_periode_kind_chk
        CHECK (kind IN ('classe', 'regime', 'mef', 'site')),
    CONSTRAINT ent_eleve_periode_dates_chk
        CHECK (date_fin IS NULL OR date_fin >= date_debut)
);

CREATE INDEX IF NOT EXISTS ent_eleve_periode_eleve_idx
    ON ent_eleve_periode (etablissement_id, eleve_id, kind, date_debut);

CREATE INDEX IF NOT EXISTS ent_eleve_periode_effectif_idx
    ON ent_eleve_periode (etablissement_id, kind, valeur, date_debut, date_fin);

-- Une seule période ouverte par attribut (classe / régime / MEF / site).
CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_periode_open_uq
    ON ent_eleve_periode (etablissement_id, eleve_id, kind)
    WHERE date_fin IS NULL;
