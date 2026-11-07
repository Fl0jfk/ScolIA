-- Module dossier élève (dictée métier + couverture Charlemagne filtrée).
-- Identité étendue, projections MEF/site, options, flags foyer payeur/sorties.

ALTER TABLE ent_eleve ADD COLUMN IF NOT EXISTS sexe TEXT;
ALTER TABLE ent_eleve ADD COLUMN IF NOT EXISTS date_naissance DATE;
ALTER TABLE ent_eleve ADD COLUMN IF NOT EXISTS numero_badge TEXT;
ALTER TABLE ent_eleve ADD COLUMN IF NOT EXISTS redoublant BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_eleve ADD COLUMN IF NOT EXISTS mef TEXT;
ALTER TABLE ent_eleve ADD COLUMN IF NOT EXISTS site TEXT;
ALTER TABLE ent_eleve ADD COLUMN IF NOT EXISTS photo_key TEXT;

ALTER TABLE ent_eleve_lien
    ADD COLUMN IF NOT EXISTS peut_autoriser_sorties BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_eleve_lien
    ADD COLUMN IF NOT EXISTS est_payeur BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_eleve_lien
    ADD COLUMN IF NOT EXISTS part_paiement SMALLINT;

ALTER TABLE ent_eleve_lien DROP CONSTRAINT IF EXISTS ent_eleve_lien_part_chk;
ALTER TABLE ent_eleve_lien
    ADD CONSTRAINT ent_eleve_lien_part_chk
    CHECK (part_paiement IS NULL OR (part_paiement >= 0 AND part_paiement <= 100));

CREATE TABLE IF NOT EXISTS ent_eleve_option (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    nature TEXT NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    date_debut DATE NOT NULL,
    date_fin DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_option_nature_chk CHECK (nature IN ('academique', 'interne'))
);

CREATE INDEX IF NOT EXISTS ent_eleve_option_eleve_idx
    ON ent_eleve_option (etablissement_id, eleve_id);

CREATE INDEX IF NOT EXISTS ent_eleve_badge_idx
    ON ent_eleve (etablissement_id, numero_badge)
    WHERE numero_badge IS NOT NULL;
