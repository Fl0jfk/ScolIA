-- Table canonique ent_absence (période + motif).
--
-- Idempotent : pas de DROP ici. `migrate()` rejoue **tous** les fichiers à chaque
-- appel (lab, ent-api, tests parallèles) ; un DROP recréerait la table et
-- effacerait les absences, en plus de casser les runs concurrents
-- (« relation ent_absence does not exist » / colonnes manquantes transitoires).
-- Même convention IF NOT EXISTS que les migrations 005+.

CREATE TABLE IF NOT EXISTS ent_absence (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    debut DATE NOT NULL,
    fin DATE NOT NULL,
    motif TEXT NOT NULL REFERENCES ent_absence_motif (code),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (fin >= debut)
);

CREATE INDEX IF NOT EXISTS ent_absence_eleve_periode_idx
    ON ent_absence (etablissement_id, eleve_id, debut, fin);

CREATE INDEX IF NOT EXISTS ent_absence_periode_idx
    ON ent_absence (etablissement_id, debut, fin);
