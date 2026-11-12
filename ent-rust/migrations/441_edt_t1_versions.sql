-- 441 — Socle 10 (emploi du temps), lot T1 : versions d'emploi du temps, besoins et pièces
-- (docs/socles/10_EMPLOI_DU_TEMPS.md §8.4, §21.2, §21.6).
--
-- Un emploi du temps n'est plus une grille unique : c'est une **version** (brouillon, publiée,
-- remplacée, archivée) qui vise des **cours** (socle 5, `ent_cours`), jamais un texte de classe.
-- * version « de base » : plan de l'année ; au plus UNE version de base publiée par date ;
-- * version « temporaire » (§8.4) : remplace le plan d'un périmètre (cours, intervenants ou lieux)
--   sur une plage bornée, puis le plan de base revient seul ; au plus une temporaire publiée par
--   cours et par date (contrainte posée en 442, sur les placements).
-- * D-036 : le périmètre d'une version temporaire est une table (pas de JSONB).
-- E1 (etablissement_id partout), E2 (clés composites), E3 (UUID applicatif), E6 (cree_le, modifie_le),
-- E13 (RLS posée en 452), E8 (additive, rejouable).

CREATE EXTENSION IF NOT EXISTS btree_gist;

-- Cible des clés composites : l'année scolaire.
CREATE UNIQUE INDEX IF NOT EXISTS ent_annee_scolaire_id_etab_t1_uq ON ent_annee_scolaire (id, etablissement_id);

CREATE TABLE IF NOT EXISTS ent_edt_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 160),
    nature TEXT NOT NULL DEFAULT 'base' CHECK (nature IN ('base', 'temporaire')),
    -- Version de base que la temporaire surcharge (obligatoire pour une temporaire, interdite sinon).
    base_version_id UUID,
    periode_libelle TEXT CHECK (periode_libelle IS NULL OR char_length(periode_libelle) <= 120),
    etat TEXT NOT NULL DEFAULT 'brouillon'
        CHECK (etat IN ('brouillon', 'publiee', 'remplacee', 'archivee', 'abandonnee')),
    -- Posées à la publication (dates incluses). `valable_au` nul = jusqu'à la fin de l'année.
    valable_du DATE,
    valable_au DATE,
    -- Diffusion aux familles différée (publication en deux temps, T5) et version provisoire.
    diffusion_familles_le TIMESTAMPTZ,
    provisoire BOOLEAN NOT NULL DEFAULT false,
    -- 'saisie' (à la main), 'reprise' (créneaux existants, M2), 'generation' (T8), 'import' (T10).
    origine TEXT NOT NULL DEFAULT 'saisie' CHECK (origine IN ('saisie', 'reprise', 'generation', 'import')),
    publiee_par UUID,
    publiee_le TIMESTAMPTZ,
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_edt_version_dates_chk CHECK (valable_au IS NULL OR valable_du IS NULL OR valable_au >= valable_du),
    CONSTRAINT ent_edt_version_publiee_chk CHECK (etat NOT IN ('publiee', 'remplacee') OR valable_du IS NOT NULL),
    CONSTRAINT ent_edt_version_publiee_trace_chk CHECK (etat NOT IN ('publiee', 'remplacee') OR publiee_le IS NOT NULL),
    -- Une temporaire en brouillon peut ne pas avoir encore sa plage ; publiée, elle est bornée.
    CONSTRAINT ent_edt_version_base_chk CHECK (
        (nature = 'base' AND base_version_id IS NULL)
        OR (nature = 'temporaire' AND base_version_id IS NOT NULL
            AND (etat IN ('brouillon', 'abandonnee') OR (valable_du IS NOT NULL AND valable_au IS NOT NULL)))
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_version_id_etab_uq ON ent_edt_version (id, etablissement_id);
CREATE INDEX IF NOT EXISTS ent_edt_version_annee_idx ON ent_edt_version (etablissement_id, annee_id, etat);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_version_annee_fk') THEN
        ALTER TABLE ent_edt_version ADD CONSTRAINT ent_edt_version_annee_fk
            FOREIGN KEY (annee_id, etablissement_id) REFERENCES ent_annee_scolaire (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_version_base_fk') THEN
        ALTER TABLE ent_edt_version ADD CONSTRAINT ent_edt_version_base_fk
            FOREIGN KEY (base_version_id, etablissement_id) REFERENCES ent_edt_version (id, etablissement_id);
    END IF;
    -- Une seule version DE BASE publiée par date (les temporaires se superposent à la base).
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_version_base_publiee_excl') THEN
        ALTER TABLE ent_edt_version ADD CONSTRAINT ent_edt_version_base_publiee_excl
            EXCLUDE USING gist (
                etablissement_id WITH =, annee_id WITH =,
                daterange(valable_du, valable_au, '[]') WITH &&
            ) WHERE (etat = 'publiee' AND nature = 'base');
    END IF;
END
$$;

-- Périmètre d'une version temporaire (§8.4) : cours, intervenants ou lieux touchés. Exactement une
-- cible par ligne. Aucune ligne pour une version de base.
CREATE TABLE IF NOT EXISTS ent_edt_version_perimetre (
    version_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    cours_id UUID,
    agent_id UUID,
    lieu_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_edt_version_perimetre_une_cible_chk CHECK (
        (cours_id IS NOT NULL)::int + (agent_id IS NOT NULL)::int + (lieu_id IS NOT NULL)::int = 1
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_version_perimetre_uq ON ent_edt_version_perimetre (
    version_id,
    coalesce(cours_id, '00000000-0000-0000-0000-000000000000'::uuid),
    coalesce(agent_id, '00000000-0000-0000-0000-000000000000'::uuid),
    coalesce(lieu_id, '00000000-0000-0000-0000-000000000000'::uuid)
);
CREATE INDEX IF NOT EXISTS ent_edt_version_perimetre_cours_idx
    ON ent_edt_version_perimetre (etablissement_id, cours_id) WHERE cours_id IS NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_version_perimetre_version_fk') THEN
        ALTER TABLE ent_edt_version_perimetre ADD CONSTRAINT ent_edt_version_perimetre_version_fk
            FOREIGN KEY (version_id, etablissement_id) REFERENCES ent_edt_version (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_version_perimetre_cours_fk') THEN
        ALTER TABLE ent_edt_version_perimetre ADD CONSTRAINT ent_edt_version_perimetre_cours_fk
            FOREIGN KEY (cours_id, etablissement_id) REFERENCES ent_cours (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_version_perimetre_lieu_fk') THEN
        ALTER TABLE ent_edt_version_perimetre ADD CONSTRAINT ent_edt_version_perimetre_lieu_fk
            FOREIGN KEY (lieu_id, etablissement_id) REFERENCES ent_salle (id, etablissement_id);
    END IF;
END
$$;

-- Besoin horaire d'un cours pour une version (une ligne par cours et par version).
CREATE TABLE IF NOT EXISTS ent_edt_besoin (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    version_id UUID NOT NULL,
    annee_id UUID NOT NULL,
    cours_id UUID NOT NULL,
    heures_hebdo NUMERIC(4, 2) NOT NULL CHECK (heures_hebdo >= 0 AND heures_hebdo <= 40),
    -- 'cours' (heures du cours, socle 5) ou 'ajuste' (reprise : somme des créneaux, ou saisie).
    source TEXT NOT NULL DEFAULT 'cours' CHECK (source IN ('cours', 'ajuste')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_besoin_version_cours_uq ON ent_edt_besoin (version_id, cours_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_besoin_id_etab_uq ON ent_edt_besoin (id, etablissement_id);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_besoin_version_fk') THEN
        ALTER TABLE ent_edt_besoin ADD CONSTRAINT ent_edt_besoin_version_fk
            FOREIGN KEY (version_id, etablissement_id) REFERENCES ent_edt_version (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_besoin_cours_fk') THEN
        ALTER TABLE ent_edt_besoin ADD CONSTRAINT ent_edt_besoin_cours_fk
            FOREIGN KEY (cours_id, etablissement_id) REFERENCES ent_cours (id, etablissement_id);
    END IF;
END
$$;

-- Pièce : un morceau du besoin à placer (deux heures consécutives, une heure en semaine B…).
CREATE TABLE IF NOT EXISTS ent_edt_piece (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    besoin_id UUID NOT NULL,
    duree_creneaux SMALLINT NOT NULL CHECK (duree_creneaux BETWEEN 1 AND 12),
    -- 'toutes' ou étiquette de semaine du socle 5 (A, B…).
    semaine_type TEXT NOT NULL DEFAULT 'toutes' CHECK (char_length(semaine_type) BETWEEN 1 AND 20),
    consecutif BOOLEAN NOT NULL DEFAULT true,
    type_lieu TEXT CHECK (type_lieu IS NULL OR char_length(type_lieu) <= 40),
    equipements TEXT[] NOT NULL DEFAULT '{}' CHECK (cardinality(equipements) <= 40),
    lieu_impose_id UUID,
    ordre SMALLINT NOT NULL DEFAULT 1 CHECK (ordre >= 1),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_piece_id_etab_uq ON ent_edt_piece (id, etablissement_id);
CREATE INDEX IF NOT EXISTS ent_edt_piece_besoin_idx ON ent_edt_piece (etablissement_id, besoin_id);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_piece_besoin_fk') THEN
        ALTER TABLE ent_edt_piece ADD CONSTRAINT ent_edt_piece_besoin_fk
            FOREIGN KEY (besoin_id, etablissement_id) REFERENCES ent_edt_besoin (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_piece_lieu_fk') THEN
        ALTER TABLE ent_edt_piece ADD CONSTRAINT ent_edt_piece_lieu_fk
            FOREIGN KEY (lieu_impose_id, etablissement_id) REFERENCES ent_salle (id, etablissement_id);
    END IF;
END
$$;
