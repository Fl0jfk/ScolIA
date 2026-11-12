-- 442 — Socle 10 (emploi du temps), lot T1 : placements et lieux (docs/socles/10_EMPLOI_DU_TEMPS.md §21.2).
--
-- Un placement dit « ce cours, ce jour de la semaine, à ce créneau de la grille, dans ces lieux »
-- dans UNE version. Il vise le **cours** (socle 5) ; les intervenants viennent du socle 6 à la date
-- (co-enseignement : plusieurs services sur un cours, aucune colonne professeur ici). Le créneau de
-- grille est un **code** (« M2 »), pas une heure : un changement de grille déplace les heures sans
-- toucher au plan. Un créneau historique dont l'heure ne tombe sur aucun code de grille est repris
-- « hors grille » (heures libres, signalé) — jamais deviné.
--
-- `ent_edt_creneau` n'est PAS remplacée ici : elle reste la source des lecteurs actuels (appel,
-- cahier, notes) tant que T11 n'a pas basculé ; `creneau_id` relie le placement repris à son créneau
-- (reprise rejouable, aucune perte). E1, E2, E3, E6, E8, E10 (aucun JSON ; l'explication typée de la
-- génération arrive en T8) ; RLS posée en 452.

CREATE TABLE IF NOT EXISTS ent_edt_placement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    version_id UUID NOT NULL,
    piece_id UUID NOT NULL,
    -- Dénormalisé depuis le besoin de la pièce (cohérence vérifiée par déclencheur) : les lectures
    -- « placements d'un cours » et les contraintes n'ont pas à joindre trois tables.
    cours_id UUID NOT NULL,
    jour_semaine SMALLINT NOT NULL CHECK (jour_semaine BETWEEN 1 AND 7),
    creneau_debut_code TEXT CHECK (creneau_debut_code IS NULL OR char_length(btrim(creneau_debut_code)) BETWEEN 1 AND 20),
    duree_creneaux SMALLINT CHECK (duree_creneaux IS NULL OR duree_creneaux BETWEEN 1 AND 12),
    -- Hors grille (reprise d'un créneau aux heures libres) : signalé, jamais deviné.
    heure_debut_libre TIME,
    heure_fin_libre TIME,
    semaine_type TEXT NOT NULL DEFAULT 'toutes' CHECK (char_length(semaine_type) BETWEEN 1 AND 20),
    verrouille BOOLEAN NOT NULL DEFAULT false,
    origine TEXT NOT NULL DEFAULT 'main' CHECK (origine IN ('generation', 'main', 'import', 'reprise')),
    -- Créneau historique d'où vient le placement (reprise M2) ; nul pour un placement créé en brouillon.
    creneau_id UUID,
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_edt_placement_position_chk CHECK (
        (creneau_debut_code IS NOT NULL AND duree_creneaux IS NOT NULL
            AND heure_debut_libre IS NULL AND heure_fin_libre IS NULL)
        OR (creneau_debut_code IS NULL AND duree_creneaux IS NULL
            AND heure_debut_libre IS NOT NULL AND heure_fin_libre IS NOT NULL
            AND heure_fin_libre > heure_debut_libre)
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_placement_id_etab_uq ON ent_edt_placement (id, etablissement_id);
-- Reprise idempotente : un créneau historique donne au plus un placement par version.
CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_placement_creneau_uq
    ON ent_edt_placement (version_id, creneau_id) WHERE creneau_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_edt_placement_version_idx ON ent_edt_placement (etablissement_id, version_id, jour_semaine);
CREATE INDEX IF NOT EXISTS ent_edt_placement_cours_idx ON ent_edt_placement (etablissement_id, cours_id);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_placement_version_fk') THEN
        ALTER TABLE ent_edt_placement ADD CONSTRAINT ent_edt_placement_version_fk
            FOREIGN KEY (version_id, etablissement_id) REFERENCES ent_edt_version (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_placement_piece_fk') THEN
        ALTER TABLE ent_edt_placement ADD CONSTRAINT ent_edt_placement_piece_fk
            FOREIGN KEY (piece_id, etablissement_id) REFERENCES ent_edt_piece (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_placement_cours_fk') THEN
        ALTER TABLE ent_edt_placement ADD CONSTRAINT ent_edt_placement_cours_fk
            FOREIGN KEY (cours_id, etablissement_id) REFERENCES ent_cours (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_placement_creneau_fk') THEN
        ALTER TABLE ent_edt_placement ADD CONSTRAINT ent_edt_placement_creneau_fk
            FOREIGN KEY (creneau_id) REFERENCES ent_edt_creneau (id) ON DELETE SET NULL;
    END IF;
END
$$;

-- Le cours du placement est celui du besoin de sa pièce, et le besoin est de la même version.
CREATE OR REPLACE FUNCTION ent_edt_placement_coherence() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    b_cours UUID;
    b_version UUID;
BEGIN
    SELECT b.cours_id, b.version_id INTO b_cours, b_version
      FROM ent_edt_piece p
      JOIN ent_edt_besoin b ON b.id = p.besoin_id AND b.etablissement_id = p.etablissement_id
     WHERE p.id = NEW.piece_id AND p.etablissement_id = NEW.etablissement_id;
    IF b_cours IS NULL OR b_cours <> NEW.cours_id OR b_version <> NEW.version_id THEN
        RAISE EXCEPTION 'placement incohérent : cours ou version différents de ceux de la pièce'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS ent_edt_placement_coherence_tg ON ent_edt_placement;
CREATE TRIGGER ent_edt_placement_coherence_tg BEFORE INSERT OR UPDATE OF piece_id, cours_id, version_id
    ON ent_edt_placement FOR EACH ROW EXECUTE FUNCTION ent_edt_placement_coherence();

-- Les lieux d'un placement (une séance peut occuper plusieurs lieux : salle + gymnase…).
CREATE TABLE IF NOT EXISTS ent_edt_placement_lieu (
    placement_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    lieu_id UUID NOT NULL,
    principal BOOLEAN NOT NULL DEFAULT false,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (placement_id, lieu_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_placement_lieu_principal_uq
    ON ent_edt_placement_lieu (placement_id) WHERE principal;
CREATE INDEX IF NOT EXISTS ent_edt_placement_lieu_lieu_idx ON ent_edt_placement_lieu (etablissement_id, lieu_id);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_placement_lieu_placement_fk') THEN
        ALTER TABLE ent_edt_placement_lieu ADD CONSTRAINT ent_edt_placement_lieu_placement_fk
            FOREIGN KEY (placement_id, etablissement_id) REFERENCES ent_edt_placement (id, etablissement_id)
            ON DELETE CASCADE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_placement_lieu_lieu_fk') THEN
        ALTER TABLE ent_edt_placement_lieu ADD CONSTRAINT ent_edt_placement_lieu_lieu_fk
            FOREIGN KEY (lieu_id, etablissement_id) REFERENCES ent_salle (id, etablissement_id);
    END IF;
END
$$;
