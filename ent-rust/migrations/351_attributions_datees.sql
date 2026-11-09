-- Socle 6 (personnels), lot P0, migration 351 : attributions de rôle identifiées, datées, jamais
-- écrasées (`docs/socles/06_PERSONNELS.md` §2.2 n° 9, §8.9, §17).
--
-- Avant : clé `(role_id, user_id)`, `ON CONFLICT … DO UPDATE` qui écrasait la précédente, début
-- forcé à aujourd'hui, suppression physique au retrait. « Qui avait ce rôle le 12 mars ? » restait
-- sans réponse et la rentrée ne pouvait pas se préparer.
--
-- Après : une attribution = une ligne identifiée (`id`), avec début éventuellement futur, fin
-- éventuelle, origine (`source`, `source_ref`) et clôture (`clos_le`, `clos_par_user_id`). Un retrait
-- ou une modification **clôt** la ligne au lieu de l'effacer ; une contrainte d'exclusion interdit
-- deux attributions non closes qui se chevauchent pour le même rôle et la même personne.
-- Les droits ne lisent que les lignes non closes, dans leurs dates.
--
-- Additive et rejouable. Chaque ligne existante devient une attribution `manuelle` non close.

ALTER TABLE ent_role_attribution
    ADD COLUMN IF NOT EXISTS id UUID NOT NULL DEFAULT gen_random_uuid(),
    ADD COLUMN IF NOT EXISTS source TEXT NOT NULL DEFAULT 'manuelle',
    ADD COLUMN IF NOT EXISTS source_ref UUID,
    ADD COLUMN IF NOT EXISTS clos_le TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS clos_par_user_id UUID REFERENCES ent_user (id);

CREATE EXTENSION IF NOT EXISTS btree_gist;

DO $$
BEGIN
    -- Clé primaire : (role_id, user_id) → id.
    IF EXISTS (SELECT 1 FROM pg_constraint
                WHERE conrelid = 'ent_role_attribution'::regclass AND conname = 'ent_role_attribution_pkey'
                  AND pg_get_constraintdef(oid) NOT LIKE '%(id)%') THEN
        ALTER TABLE ent_role_attribution DROP CONSTRAINT ent_role_attribution_pkey;
        ALTER TABLE ent_role_attribution ADD CONSTRAINT ent_role_attribution_pkey PRIMARY KEY (id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint
                    WHERE conrelid = 'ent_role_attribution'::regclass AND conname = 'ent_role_attribution_source_chk') THEN
        ALTER TABLE ent_role_attribution
            ADD CONSTRAINT ent_role_attribution_source_chk CHECK (source IN ('manuelle', 'modification', 'systeme'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint
                    WHERE conrelid = 'ent_role_attribution'::regclass AND conname = 'ent_role_attribution_sans_chevauchement') THEN
        ALTER TABLE ent_role_attribution
            ADD CONSTRAINT ent_role_attribution_sans_chevauchement
            EXCLUDE USING gist (
                role_id WITH =,
                user_id WITH =,
                daterange(debut, fin, '[]') WITH &&
            ) WHERE (clos_le IS NULL);
    END IF;
END $$;

-- Lecture des droits d'une personne (hot path de `droits_effectifs`) et historique d'un rôle.
CREATE INDEX IF NOT EXISTS ent_role_attribution_actives_idx
    ON ent_role_attribution (etablissement_id, user_id) WHERE clos_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_role_attribution_role_idx
    ON ent_role_attribution (etablissement_id, role_id, debut);
