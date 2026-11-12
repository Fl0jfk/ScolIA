-- 2780 — Socle 39 (stages), lot ST0 : correctifs de l'embryon 082 (RS-01, RS-06 à RS-09).
-- Réf. : docs/socles/39_STAGES.md §2.6. Migration ADDITIVE et REJOUABLE (rejouée à chaque démarrage,
-- E8) ; aucune migration existante n'est modifiée (PILOTAGE règle 15). Aucun JSON (D-036).
--
-- * RS-01 : le statut `declaree` remplace `active` (stage connu, SANS convention signée dans
--   ScolIA) ; les lignes `active` existantes passent `declaree`.
-- * RS-06 : clés étrangères composées (etablissement_id, …) en RESTRICT ; `cree_par_personne_id`
--   à côté de `created_by_user_id` (D-020).
-- * RS-07 : `modifie_le` et `version` (verrou optimiste).
-- * RS-08 : clé de contrôle du SIRET (fonction SQL identique au code Rust) et unicité par
--   établissement, posée seulement s'il n'existe aucun doublon (rapport : `ent_stage_siret_doublons`).
-- * RS-09 : un élève n'a pas deux stages non annulés qui se chevauchent (exclusion btree_gist).

CREATE EXTENSION IF NOT EXISTS btree_gist;

-- ───────────── RS-01 : statut `declaree` ─────────────
DO $$
BEGIN
    -- Retire l'ancienne contrainte (liste `active/cloturee/annulee`) avant de convertir les lignes.
    IF EXISTS (SELECT 1 FROM pg_constraint
                WHERE conname = 'ent_convention_statut_chk'
                  AND conrelid = 'ent_convention_stage'::regclass
                  AND pg_get_constraintdef(oid) LIKE '%active%') THEN
        ALTER TABLE ent_convention_stage DROP CONSTRAINT ent_convention_statut_chk;
    END IF;
END
$$;

UPDATE ent_convention_stage SET statut = 'declaree' WHERE statut = 'active';

-- Gardes « déjà fait » : une migration rejouée à chaque démarrage ne doit pas reprendre un verrou
-- exclusif sur des tables en service (interblocages avec le trafic en cours).
DO $$
BEGIN
    IF (SELECT column_default FROM information_schema.columns
         WHERE table_schema = 'public' AND table_name = 'ent_convention_stage' AND column_name = 'statut')
       IS DISTINCT FROM '''declaree''::text' THEN
        ALTER TABLE ent_convention_stage ALTER COLUMN statut SET DEFAULT 'declaree';
    END IF;
END
$$;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint
                    WHERE conname = 'ent_convention_statut_chk'
                      AND conrelid = 'ent_convention_stage'::regclass) THEN
        ALTER TABLE ent_convention_stage
            ADD CONSTRAINT ent_convention_statut_chk
            CHECK (statut IN ('declaree', 'cloturee', 'annulee'));
    END IF;
END
$$;

-- ───────────── RS-07 : version et horodatage de modification ─────────────
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                    WHERE table_schema = 'public' AND table_name = 'ent_entreprise' AND column_name = 'version') THEN
        ALTER TABLE ent_entreprise
            ADD COLUMN modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
            ADD COLUMN version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
            ADD COLUMN cree_par_personne_id UUID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                    WHERE table_schema = 'public' AND table_name = 'ent_convention_stage' AND column_name = 'version') THEN
        ALTER TABLE ent_convention_stage
            ADD COLUMN modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
            ADD COLUMN version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
            ADD COLUMN cree_par_personne_id UUID;
    END IF;
END
$$;

-- ───────────── RS-06 : clés composées (cloisonnement d'établissement garanti en base) ─────────────
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_entreprise_etab_id_uq') THEN
        ALTER TABLE ent_entreprise ADD CONSTRAINT ent_entreprise_etab_id_uq UNIQUE (etablissement_id, id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_convention_stage_etab_id_uq') THEN
        ALTER TABLE ent_convention_stage
            ADD CONSTRAINT ent_convention_stage_etab_id_uq UNIQUE (etablissement_id, id);
    END IF;

    -- Élève : (etablissement_id, eleve_id) → ent_eleve, RESTRICT. Posée NOT VALID puis validée
    -- seulement s'il n'y a aucun orphelin (jamais de suppression de ligne ici).
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_convention_stage_eleve_etab_fk') THEN
        ALTER TABLE ent_convention_stage
            ADD CONSTRAINT ent_convention_stage_eleve_etab_fk
            FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id)
            ON DELETE RESTRICT NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM ent_convention_stage c
                    WHERE NOT EXISTS (SELECT 1 FROM ent_eleve e
                                       WHERE e.id = c.eleve_id AND e.etablissement_id = c.etablissement_id)) THEN
        ALTER TABLE ent_convention_stage VALIDATE CONSTRAINT ent_convention_stage_eleve_etab_fk;
    ELSE
        RAISE NOTICE 'ent_convention_stage : orphelins d''élève, clé composée non validée';
    END IF;

    -- Organisme : remplace la clé simple par la clé composée.
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_convention_stage_entreprise_etab_fk') THEN
        ALTER TABLE ent_convention_stage
            ADD CONSTRAINT ent_convention_stage_entreprise_etab_fk
            FOREIGN KEY (etablissement_id, entreprise_id) REFERENCES ent_entreprise (etablissement_id, id)
            ON DELETE RESTRICT NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM ent_convention_stage c
                    WHERE NOT EXISTS (SELECT 1 FROM ent_entreprise e
                                       WHERE e.id = c.entreprise_id AND e.etablissement_id = c.etablissement_id)) THEN
        ALTER TABLE ent_convention_stage VALIDATE CONSTRAINT ent_convention_stage_entreprise_etab_fk;
        -- La clé simple (par défaut NO ACTION, sans garantie d'établissement) devient redondante.
        ALTER TABLE ent_convention_stage DROP CONSTRAINT IF EXISTS ent_convention_stage_entreprise_id_fkey;
    ELSE
        RAISE NOTICE 'ent_convention_stage : orphelins d''organisme, clé composée non validée';
    END IF;

    -- Personne qui a créé (D-020) : clé composée vers ent_personne, facultative.
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_entreprise_cree_par_personne_fk') THEN
        ALTER TABLE ent_entreprise
            ADD CONSTRAINT ent_entreprise_cree_par_personne_fk
            FOREIGN KEY (etablissement_id, cree_par_personne_id) REFERENCES ent_personne (etablissement_id, id)
            ON DELETE RESTRICT NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_convention_stage_cree_par_personne_fk') THEN
        ALTER TABLE ent_convention_stage
            ADD CONSTRAINT ent_convention_stage_cree_par_personne_fk
            FOREIGN KEY (etablissement_id, cree_par_personne_id) REFERENCES ent_personne (etablissement_id, id)
            ON DELETE RESTRICT NOT VALID;
    END IF;
END
$$;

-- ───────────── RS-08 : clé de contrôle du SIRET ─────────────
-- Luhn sur 14 chiffres, exception de La Poste (SIREN 356000000 : somme des chiffres multiple de 5).
-- Identique à `domain_eleve::siret_cle_valide`.
CREATE OR REPLACE FUNCTION ent_siret_cle_valide(s TEXT) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
    SELECT CASE
        WHEN s IS NULL OR s !~ '^[0-9]{14}$' THEN false
        WHEN left(s, 9) = '356000000' THEN
            (SELECT sum(substr(s, i, 1)::int) % 5 = 0 FROM generate_series(1, 14) AS i)
        ELSE
            (SELECT sum(CASE WHEN i % 2 = 1
                             THEN CASE WHEN substr(s, i, 1)::int * 2 > 9
                                       THEN substr(s, i, 1)::int * 2 - 9
                                       ELSE substr(s, i, 1)::int * 2 END
                             ELSE substr(s, i, 1)::int END) % 10 = 0
               FROM generate_series(1, 14) AS i)
    END
$$;

-- Les lignes existantes ne sont pas réécrites : la contrainte ne vise que les nouvelles écritures
-- (NOT VALID) ; l'ancienne contrainte de longueur reste en place.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_entreprise_siret_cle_chk') THEN
        ALTER TABLE ent_entreprise
            ADD CONSTRAINT ent_entreprise_siret_cle_chk
            CHECK (siret IS NULL OR ent_siret_cle_valide(siret)) NOT VALID;
    END IF;
END
$$;

-- Rapport (sans copie ni fusion automatique) : SIRET portés par plusieurs organismes d'un établissement.
CREATE OR REPLACE FUNCTION ent_stage_siret_doublons()
RETURNS TABLE (etablissement_id UUID, siret TEXT, nombre BIGINT)
LANGUAGE sql STABLE AS $$
    SELECT e.etablissement_id, e.siret, count(*)
      FROM ent_entreprise e
     WHERE e.siret IS NOT NULL
     GROUP BY e.etablissement_id, e.siret
    HAVING count(*) > 1
$$;

-- Unicité partielle posée seulement s'il n'y a aucun doublon ; sinon, la création applicative
-- refuse quand même un SIRET déjà présent, et le rapport ci-dessus liste les doublons à traiter.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'ent_entreprise_siret_uq') THEN
        IF EXISTS (SELECT 1 FROM ent_stage_siret_doublons()) THEN
            RAISE NOTICE 'ent_entreprise : doublons de SIRET, index unique différé (voir ent_stage_siret_doublons())';
        ELSE
            CREATE UNIQUE INDEX ent_entreprise_siret_uq
                ON ent_entreprise (etablissement_id, siret) WHERE siret IS NOT NULL;
        END IF;
    END IF;
END
$$;

-- ───────────── RS-09 : pas de chevauchement de deux stages d'un même élève ─────────────
-- Les stages annulés ne comptent pas. Posée seulement si les données existantes la respectent
-- (sinon un NOTICE : la création applicative refuse déjà le chevauchement).
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_convention_stage_sans_chevauchement') THEN
        IF EXISTS (SELECT 1 FROM ent_convention_stage a JOIN ent_convention_stage b
                       ON a.etablissement_id = b.etablissement_id AND a.eleve_id = b.eleve_id AND a.id < b.id
                    WHERE a.statut <> 'annulee' AND b.statut <> 'annulee'
                      AND daterange(a.debut, a.fin, '[]') && daterange(b.debut, b.fin, '[]')) THEN
            RAISE NOTICE 'ent_convention_stage : chevauchements existants, exclusion différée';
        ELSE
            ALTER TABLE ent_convention_stage
                ADD CONSTRAINT ent_convention_stage_sans_chevauchement
                EXCLUDE USING gist (
                    etablissement_id WITH =,
                    eleve_id WITH =,
                    daterange(debut, fin, '[]') WITH &&
                ) WHERE (statut <> 'annulee');
        END IF;
    END IF;
END
$$;

DO $$
BEGIN
    IF to_regclass('ent_convention_stage_statut_idx') IS NULL THEN
        CREATE INDEX ent_convention_stage_statut_idx ON ent_convention_stage (etablissement_id, statut);
    END IF;
END
$$;
