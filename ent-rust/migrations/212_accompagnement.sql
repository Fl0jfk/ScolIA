-- 212 — Accompagnement (socle 2, lot S2-11) : PAP / PPS / PPRE / ULIS, aménagements,
-- AESH, équipe éducative (ACC-01 → ACC-05). Additif et idempotent.
--
-- Réutilise `ent_eleve_dispositif` (migration 016) : types PPRE et ULIS ajoutés,
-- statut et échéance de révision. Les aménagements sont des CODES d'un catalogue
-- fermé (jamais de diagnostic), seuls visibles des professeurs de l'élève.

ALTER TABLE ent_eleve_dispositif DROP CONSTRAINT IF EXISTS ent_eleve_dispositif_kind_chk;
ALTER TABLE ent_eleve_dispositif ADD CONSTRAINT ent_eleve_dispositif_kind_chk
    CHECK (kind IN ('pap', 'pps', 'pai', 'gevasco', 'ppre', 'ulis', 'autre'));

ALTER TABLE ent_eleve_dispositif ADD COLUMN IF NOT EXISTS statut TEXT NOT NULL DEFAULT 'actif';
ALTER TABLE ent_eleve_dispositif DROP CONSTRAINT IF EXISTS ent_eleve_dispositif_statut_chk;
ALTER TABLE ent_eleve_dispositif ADD CONSTRAINT ent_eleve_dispositif_statut_chk
    CHECK (statut IN ('actif', 'clos'));
ALTER TABLE ent_eleve_dispositif ADD COLUMN IF NOT EXISTS date_revision DATE;
ALTER TABLE ent_eleve_dispositif ADD COLUMN IF NOT EXISTS ouvert_par_user_id UUID;
ALTER TABLE ent_eleve_dispositif ADD COLUMN IF NOT EXISTS clos_at TIMESTAMPTZ;
ALTER TABLE ent_eleve_dispositif ADD COLUMN IF NOT EXISTS revise_at TIMESTAMPTZ;

CREATE TABLE IF NOT EXISTS ent_amenagement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    dispositif_id UUID REFERENCES ent_eleve_dispositif (id) ON DELETE SET NULL,
    code TEXT NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{2,39}$'),
    portee TEXT NOT NULL CHECK (portee IN ('cours', 'examen', 'les_deux')),
    date_debut DATE NOT NULL,
    date_fin DATE,
    actif BOOLEAN NOT NULL DEFAULT true,
    defini_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    retire_at TIMESTAMPTZ,
    CONSTRAINT ent_amenagement_dates_chk CHECK (date_fin IS NULL OR date_fin >= date_debut)
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_amenagement_actif_uq
    ON ent_amenagement (etablissement_id, eleve_id, code)
    WHERE actif;
CREATE INDEX IF NOT EXISTS ent_amenagement_etab_idx
    ON ent_amenagement (etablissement_id, actif);

CREATE TABLE IF NOT EXISTS ent_aesh_affectation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    dispositif_id UUID REFERENCES ent_eleve_dispositif (id) ON DELETE SET NULL,
    aesh_nom TEXT NOT NULL CHECK (length(btrim(aesh_nom)) BETWEEN 1 AND 120),
    aesh_user_id UUID REFERENCES ent_user (id),
    heures_hebdo SMALLINT CHECK (heures_hebdo IS NULL OR heures_hebdo BETWEEN 1 AND 60),
    date_debut DATE NOT NULL,
    date_fin DATE,
    affecte_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_aesh_dates_chk CHECK (date_fin IS NULL OR date_fin >= date_debut)
);
CREATE INDEX IF NOT EXISTS ent_aesh_eleve_idx
    ON ent_aesh_affectation (etablissement_id, eleve_id);

CREATE TABLE IF NOT EXISTS ent_equipe_reunion (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    dispositif_id UUID REFERENCES ent_eleve_dispositif (id) ON DELETE SET NULL,
    kind TEXT NOT NULL CHECK (kind IN ('equipe_educative', 'equipe_suivi')),
    date_reunion DATE NOT NULL,
    statut TEXT NOT NULL DEFAULT 'planifiee' CHECK (statut IN ('planifiee', 'tenue', 'annulee')),
    -- Compte rendu chiffré (socle 1, D3) ; jamais dans un événement.
    compte_rendu TEXT NOT NULL DEFAULT '',
    cree_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_equipe_reunion_eleve_idx
    ON ent_equipe_reunion (etablissement_id, eleve_id, date_reunion DESC);
