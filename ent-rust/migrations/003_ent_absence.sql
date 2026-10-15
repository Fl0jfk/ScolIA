-- Référentiel motifs (partagé). La table ent_absence est créée / reshaped en 004
-- (évite IF NOT EXISTS sur une vieille forme jour+source).

CREATE TABLE IF NOT EXISTS ent_absence_motif (
    code TEXT PRIMARY KEY,
    libelle TEXT NOT NULL
);

INSERT INTO ent_absence_motif (code, libelle) VALUES
    ('sejour', 'Séjour scolaire'),
    ('maladie', 'Maladie'),
    ('familial', 'Raison familiale'),
    ('autre', 'Autre')
ON CONFLICT (code) DO NOTHING;
