-- Justificatif parent / VS : l'absence reste une seule ligne (pas de file parallèle).

ALTER TABLE ent_absence
    ADD COLUMN IF NOT EXISTS statut TEXT NOT NULL DEFAULT 'a_traiter';

ALTER TABLE ent_absence
    ADD COLUMN IF NOT EXISTS motif_justif TEXT;

ALTER TABLE ent_absence
    ADD COLUMN IF NOT EXISTS justifie_par_user_id UUID;

ALTER TABLE ent_absence
    ADD COLUMN IF NOT EXISTS justifie_at TIMESTAMPTZ;

-- NB: l'ensemble des statuts est celui de 011_absence_decisions (a_traiter,
-- justifiee, refusee, annulee_present). migrate() rejoue tous les fichiers à chaque
-- démarrage : garder ici un ensemble étroit casserait les bases ayant déjà des
-- décisions (refusee / annulee_present).
ALTER TABLE ent_absence DROP CONSTRAINT IF EXISTS ent_absence_statut_chk;

ALTER TABLE ent_absence
    ADD CONSTRAINT ent_absence_statut_chk
    CHECK (statut IN ('a_traiter', 'justifiee', 'refusee', 'annulee_present'));

CREATE INDEX IF NOT EXISTS ent_absence_statut_idx
    ON ent_absence (etablissement_id, statut, debut);
