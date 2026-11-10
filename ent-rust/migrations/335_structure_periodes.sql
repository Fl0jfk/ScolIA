-- Socle 5, lot S7 (`docs/socles/05_STRUCTURE.md` §9.3, STR-31 à STR-33) : périodisations.
--
-- Une PÉRIODISATION est un découpage de l'année (3 trimestres, 2 semestres, périodes libres) ;
-- elle est ASSIGNÉE à des niveaux, des MEF, des classes, ou posée par défaut. Pour une classe
-- donnée, la périodisation d'évaluation est celle de la cible la plus précise : classe > MEF >
-- niveau > défaut (fonction `ent_periodisation_pour_classe`, UNE SEULE source de vérité, lue par
-- le port Rust `structure_periodes::structure::*`). Un lycée en semestres et un collège en
-- trimestres cohabitent dans le même établissement.
--
-- Principes tenus PAR LA BASE :
--   * clés composites (établissement, année) : une périodisation, ses cibles et ses périodes ne
--     croisent jamais deux établissements ni deux années ;
--   * une cible n'a qu'UNE périodisation d'évaluation par année (index uniques partiels) ;
--   * pas de chevauchement de périodes dans une périodisation (contrainte d'exclusion), dates
--     d'arrêt des notes et de conseil cohérentes ;
--   * les anciens écrivains de `ent_periode` (rapatriement des périodes de notes, reconduction
--     d'année, API historique) continuent de fonctionner : un déclencheur rattache toute période
--     insérée sans périodisation à la périodisation historique de sa cible (créée au besoin).
--
-- Additif et rejouable. Ne touche pas `ent_note_periode` (verrou de saisie : socle 21).

CREATE EXTENSION IF NOT EXISTS btree_gist;

-- ───────────────────────── Cibles des clés composites ─────────────────────────
CREATE UNIQUE INDEX IF NOT EXISTS ent_annee_scolaire_id_etab_uq ON ent_annee_scolaire (id, etablissement_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_niveau_id_etab_uq ON ent_niveau (id, etablissement_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_mef_id_etab_uq ON ent_mef (id, etablissement_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_classe_id_etab_annee_uq ON ent_classe (id, etablissement_id, annee_id);

-- ───────────────────────── Périodisations ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_periodisation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    code TEXT NOT NULL CHECK (char_length(btrim(code)) BETWEEN 1 AND 40),
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 120),
    -- `evaluation` : sert aux notes et aux bulletins ; `autre` : découpage utile à d'autres
    -- usages (projets, stages), jamais résolu pour les évaluations.
    usage TEXT NOT NULL DEFAULT 'evaluation' CHECK (usage IN ('evaluation', 'autre')),
    -- `reprise` : créée par la migration ou par un ancien écrivain de `ent_periode`.
    origine TEXT NOT NULL DEFAULT 'service' CHECK (origine IN ('service', 'reprise')),
    archivee_le TIMESTAMPTZ,
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_periodisation_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id),
    CONSTRAINT ent_periodisation_code_uq UNIQUE (etablissement_id, annee_id, code),
    CONSTRAINT ent_periodisation_id_etab_uq UNIQUE (id, etablissement_id),
    CONSTRAINT ent_periodisation_cible_uq UNIQUE (id, etablissement_id, annee_id, usage)
);
CREATE INDEX IF NOT EXISTS ent_periodisation_annee_idx ON ent_periodisation (etablissement_id, annee_id);

CREATE TABLE IF NOT EXISTS ent_periodisation_cible (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    -- Recopie de l'usage de la périodisation (clé étrangère composite) : permet l'unicité
    -- « une seule périodisation d'évaluation par cible et par année » sans déclencheur.
    usage TEXT NOT NULL,
    periodisation_id UUID NOT NULL,
    niveau_id UUID,
    mef_id UUID,
    classe_id UUID,
    defaut BOOLEAN NOT NULL DEFAULT false,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Exactement une cible : niveau, MEF, classe, ou défaut.
    CONSTRAINT ent_periodisation_cible_une_chk CHECK (
        (niveau_id IS NOT NULL)::int + (mef_id IS NOT NULL)::int
        + (classe_id IS NOT NULL)::int + defaut::int = 1),
    CONSTRAINT ent_periodisation_cible_pz_fk FOREIGN KEY (periodisation_id, etablissement_id, annee_id, usage)
        REFERENCES ent_periodisation (id, etablissement_id, annee_id, usage) ON DELETE CASCADE,
    CONSTRAINT ent_periodisation_cible_niveau_fk FOREIGN KEY (niveau_id, etablissement_id)
        REFERENCES ent_niveau (id, etablissement_id),
    CONSTRAINT ent_periodisation_cible_mef_fk FOREIGN KEY (mef_id, etablissement_id)
        REFERENCES ent_mef (id, etablissement_id),
    CONSTRAINT ent_periodisation_cible_classe_fk FOREIGN KEY (classe_id, etablissement_id, annee_id)
        REFERENCES ent_classe (id, etablissement_id, annee_id)
);
CREATE INDEX IF NOT EXISTS ent_periodisation_cible_pz_idx ON ent_periodisation_cible (etablissement_id, periodisation_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_periodisation_cible_niveau_uq
    ON ent_periodisation_cible (etablissement_id, annee_id, niveau_id)
    WHERE usage = 'evaluation' AND niveau_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_periodisation_cible_mef_uq
    ON ent_periodisation_cible (etablissement_id, annee_id, mef_id)
    WHERE usage = 'evaluation' AND mef_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_periodisation_cible_classe_uq
    ON ent_periodisation_cible (etablissement_id, annee_id, classe_id)
    WHERE usage = 'evaluation' AND classe_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_periodisation_cible_defaut_uq
    ON ent_periodisation_cible (etablissement_id, annee_id)
    WHERE usage = 'evaluation' AND defaut;

-- ───────────────────────── Périodes (existant 110, enrichi) ─────────────────────────
ALTER TABLE ent_periode ADD COLUMN IF NOT EXISTS periodisation_id UUID;
-- Date à partir de laquelle la saisie des notes de la période est arrêtée (informatif : le
-- verrou de saisie reste au socle 21, qui lit cette date par le port).
ALTER TABLE ent_periode ADD COLUMN IF NOT EXISTS date_arret_notes DATE;
ALTER TABLE ent_periode ADD COLUMN IF NOT EXISTS date_conseil DATE;
ALTER TABLE ent_periode ADD COLUMN IF NOT EXISTS origine TEXT NOT NULL DEFAULT 'service';
ALTER TABLE ent_periode ADD COLUMN IF NOT EXISTS modifie_le TIMESTAMPTZ NOT NULL DEFAULT now();

ALTER TABLE ent_periode DROP CONSTRAINT IF EXISTS ent_periode_s7_chk;
ALTER TABLE ent_periode ADD CONSTRAINT ent_periode_s7_chk CHECK (
    origine IN ('service', 'reprise')
    AND (date_arret_notes IS NULL OR date_arret_notes >= date_debut)
    AND (date_conseil IS NULL OR date_conseil >= date_debut)
    AND (date_arret_notes IS NULL OR date_conseil IS NULL OR date_conseil >= date_arret_notes));

-- ───────────────────────── Reprise de l'existant ─────────────────────────
-- Une périodisation d'évaluation par (établissement, année, niveau) déjà présent dans
-- `ent_periode`, ciblée sur ce niveau (ou « défaut » pour les périodes sans niveau).
DO $$
DECLARE r RECORD; v_pz UUID;
BEGIN
    FOR r IN
        SELECT DISTINCT p.etablissement_id, p.annee_id, p.niveau_id
          FROM ent_periode p WHERE p.periodisation_id IS NULL
    LOOP
        v_pz := gen_random_uuid();
        INSERT INTO ent_periodisation (id, etablissement_id, annee_id, code, libelle, usage, origine)
        VALUES (v_pz, r.etablissement_id, r.annee_id,
                'H-' || left(v_pz::text, 8),
                CASE WHEN r.niveau_id IS NULL THEN 'Périodes de l''établissement (reprise)'
                     ELSE 'Périodes d''un niveau (reprise)' END,
                'evaluation', 'reprise');
        INSERT INTO ent_periodisation_cible
            (id, etablissement_id, annee_id, usage, periodisation_id, niveau_id, defaut)
        VALUES (gen_random_uuid(), r.etablissement_id, r.annee_id, 'evaluation', v_pz,
                r.niveau_id, r.niveau_id IS NULL);
        UPDATE ent_periode
           SET periodisation_id = v_pz, origine = 'reprise'
         WHERE etablissement_id = r.etablissement_id AND annee_id = r.annee_id
           AND niveau_id IS NOT DISTINCT FROM r.niveau_id AND periodisation_id IS NULL;
    END LOOP;
END
$$;

ALTER TABLE ent_periode ALTER COLUMN periodisation_id SET NOT NULL;
ALTER TABLE ent_periode DROP CONSTRAINT IF EXISTS ent_periode_periodisation_fk;
ALTER TABLE ent_periode ADD CONSTRAINT ent_periode_periodisation_fk
    FOREIGN KEY (periodisation_id, etablissement_id)
    REFERENCES ent_periodisation (id, etablissement_id);

-- Le code d'une période est unique DANS sa périodisation (« T1 » peut exister dans deux
-- périodisations ciblant des niveaux différents). L'index GARDE son nom `ent_periode_code_uq` :
-- la migration 110, rejouée à chaque démarrage, fait `CREATE UNIQUE INDEX IF NOT EXISTS` sur ce
-- nom et recréerait l'ancienne définition (unicité par niveau), fausse dès qu'il y a plusieurs
-- périodisations.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE schemaname = current_schema()
                    AND indexname = 'ent_periode_code_uq' AND indexdef LIKE '%periodisation_id%') THEN
        DROP INDEX IF EXISTS ent_periode_code_uq;
        CREATE UNIQUE INDEX ent_periode_code_uq ON ent_periode (periodisation_id, code);
    END IF;
END
$$;
CREATE INDEX IF NOT EXISTS ent_periode_pz_idx ON ent_periode (etablissement_id, periodisation_id, date_debut);

-- Pas de chevauchement dans une périodisation (périodes écrites par le service). Les lignes
-- « reprise » (anciens écrivains) échappent à la contrainte : elles n'étaient pas contrôlées.
ALTER TABLE ent_periode DROP CONSTRAINT IF EXISTS ent_periode_chevauchement;
ALTER TABLE ent_periode ADD CONSTRAINT ent_periode_chevauchement
    EXCLUDE USING gist (periodisation_id WITH =, daterange(date_debut, date_fin, '[]') WITH &&)
    WHERE (origine = 'service');
-- Contrainte NON différable : les anciens écrivains utilisent `ON CONFLICT DO NOTHING`, que
-- Postgres refuse avec une contrainte différable. Pour décaler des périodes voisines, le service
-- passe d'abord les lignes de la périodisation en « reprise » (hors contrainte), les modifie, puis
-- les repasse en « service » : l'état final est contrôlé ligne par ligne.

-- ───────────────────────── Déclencheurs ─────────────────────────
-- Niveau « historique » de la période : le niveau de la cible quand la périodisation n'a qu'une
-- cible et que c'est un niveau, sinon NULL. Lu par les anciens lecteurs (reconduction d'année).
CREATE OR REPLACE FUNCTION ent_periodisation_niveau_historique(p_pz uuid) RETURNS uuid
LANGUAGE sql STABLE AS $$
    SELECT CASE WHEN count(*) = 1 THEN max(niveau_id::text)::uuid END
      FROM ent_periodisation_cible WHERE periodisation_id = p_pz
$$;

CREATE OR REPLACE FUNCTION ent_trg_periode_s7() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v_pz uuid; v_debut date; v_fin date;
BEGIN
    IF NEW.periodisation_id IS NULL THEN
        -- Chemin historique (période de notes, reconduction, ancienne route) : rattacher à la
        -- périodisation de la cible (niveau ou défaut). Ces chemins n'ont pas toujours posé
        -- l'établissement courant : on le pose (il vaut celui de la ligne insérée), sinon la RLS
        -- des périodisations refuserait l'écriture sous un rôle en mode strict.
        IF COALESCE(current_setting('app.etablissement_id', true), '') = '' THEN
            PERFORM set_config('app.etablissement_id', NEW.etablissement_id::text, true);
        END IF;
        SELECT pc.periodisation_id INTO v_pz
          FROM ent_periodisation_cible pc
         WHERE pc.etablissement_id = NEW.etablissement_id AND pc.annee_id = NEW.annee_id
           AND pc.usage = 'evaluation'
           AND ((NEW.niveau_id IS NULL AND pc.defaut) OR (NEW.niveau_id IS NOT NULL AND pc.niveau_id = NEW.niveau_id));
        IF v_pz IS NULL THEN
            v_pz := gen_random_uuid();
            INSERT INTO ent_periodisation (id, etablissement_id, annee_id, code, libelle, usage, origine)
            VALUES (v_pz, NEW.etablissement_id, NEW.annee_id, 'H-' || left(v_pz::text, 8),
                    CASE WHEN NEW.niveau_id IS NULL THEN 'Périodes de l''établissement (historique)'
                         ELSE 'Périodes d''un niveau (historique)' END,
                    'evaluation', 'reprise');
            INSERT INTO ent_periodisation_cible
                (id, etablissement_id, annee_id, usage, periodisation_id, niveau_id, defaut)
            VALUES (gen_random_uuid(), NEW.etablissement_id, NEW.annee_id, 'evaluation', v_pz,
                    NEW.niveau_id, NEW.niveau_id IS NULL);
        END IF;
        NEW.periodisation_id := v_pz;
        NEW.origine := 'reprise';
    ELSIF TG_OP = 'INSERT' THEN
        NEW.niveau_id := ent_periodisation_niveau_historique(NEW.periodisation_id);
    END IF;
    -- Une période écrite par le service reste dans les bornes de son année.
    IF NEW.origine = 'service' THEN
        SELECT a.date_debut, a.date_fin INTO v_debut, v_fin
          FROM ent_annee_scolaire a
         WHERE a.id = NEW.annee_id AND a.etablissement_id = NEW.etablissement_id;
        IF v_debut IS NULL OR NEW.date_debut < v_debut OR NEW.date_fin > v_fin THEN
            RAISE EXCEPTION 'periode_hors_annee: la période déborde de l''année' USING ERRCODE = '23514';
        END IF;
    END IF;
    NEW.modifie_le := now();
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_periode_s7 ON ent_periode;
CREATE TRIGGER ent_periode_s7 BEFORE INSERT OR UPDATE OF date_debut, date_fin, periodisation_id, origine
    ON ent_periode FOR EACH ROW EXECUTE FUNCTION ent_trg_periode_s7();

-- La périodisation d'une période ne change pas (on ne déplace pas une période entre deux
-- découpages : on remplace le découpage).
CREATE OR REPLACE FUNCTION ent_trg_periode_pz_fixe() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.periodisation_id IS DISTINCT FROM OLD.periodisation_id
       OR NEW.annee_id IS DISTINCT FROM OLD.annee_id
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id THEN
        RAISE EXCEPTION 'periode_pz_fixe: une période ne change pas de périodisation' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_periode_pz_fixe ON ent_periode;
CREATE TRIGGER ent_periode_pz_fixe BEFORE UPDATE ON ent_periode
    FOR EACH ROW EXECUTE FUNCTION ent_trg_periode_pz_fixe();

-- Quand les cibles d'une périodisation changent, le niveau « historique » de ses périodes suit.
CREATE OR REPLACE FUNCTION ent_trg_periodisation_cible_sync() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v_pz uuid := COALESCE(NEW.periodisation_id, OLD.periodisation_id);
BEGIN
    UPDATE ent_periode SET niveau_id = ent_periodisation_niveau_historique(v_pz)
     WHERE periodisation_id = v_pz
       AND niveau_id IS DISTINCT FROM ent_periodisation_niveau_historique(v_pz);
    RETURN NULL;
END
$$;
DROP TRIGGER IF EXISTS ent_periodisation_cible_sync ON ent_periodisation_cible;
CREATE TRIGGER ent_periodisation_cible_sync AFTER INSERT OR UPDATE OR DELETE ON ent_periodisation_cible
    FOR EACH ROW EXECUTE FUNCTION ent_trg_periodisation_cible_sync();

UPDATE ent_periode p SET niveau_id = ent_periodisation_niveau_historique(p.periodisation_id)
 WHERE p.niveau_id IS DISTINCT FROM ent_periodisation_niveau_historique(p.periodisation_id)
   AND p.origine = 'service';

-- ───────────────────────── Résolution pour une classe ─────────────────────────
-- LA règle (une seule implémentation) : classe > MEF > niveau > défaut. Le niveau d'une classe est
-- le sien, à défaut celui de son MEF. NULL si aucune cible ne s'applique.
CREATE OR REPLACE FUNCTION ent_periodisation_pour_classe(p_etab uuid, p_classe uuid) RETURNS uuid
LANGUAGE sql STABLE AS $$
    SELECT pc.periodisation_id
      FROM ent_classe c
      JOIN ent_periodisation_cible pc
        ON pc.etablissement_id = c.etablissement_id AND pc.annee_id = c.annee_id
       AND pc.usage = 'evaluation'
      JOIN ent_periodisation pz ON pz.id = pc.periodisation_id AND pz.archivee_le IS NULL
     WHERE c.id = p_classe AND c.etablissement_id = p_etab
       AND (pc.classe_id = c.id
            OR (pc.mef_id IS NOT NULL AND pc.mef_id = c.mef_id)
            OR (pc.niveau_id IS NOT NULL AND pc.niveau_id = COALESCE(
                    c.niveau_id, (SELECT m.niveau_id FROM ent_mef m
                                   WHERE m.id = c.mef_id AND m.etablissement_id = c.etablissement_id)))
            OR pc.defaut)
     ORDER BY CASE WHEN pc.classe_id IS NOT NULL THEN 0 WHEN pc.mef_id IS NOT NULL THEN 1
                   WHEN pc.niveau_id IS NOT NULL THEN 2 ELSE 3 END
     LIMIT 1
$$;

-- ───────────────────────── Journal de structure : nouveaux objets ─────────────────────────
-- La liste fermée des objets (332) est redéfinie par chaque lot. Les migrations sont REJOUÉES à
-- chaque démarrage : une migration placée avant 338 ne peut donc pas poser une liste plus courte
-- que celle des lignes déjà écrites (les objets de S5 existent déjà en base). Ici on assouplit en
-- contrôle de FORMAT (minuscules et `_`) ; la liste fermée complète — qui contient les objets
-- `periodisation`, `periode`, `grille` de ce lot — est reposée par 338, dernière à la redéfinir.
-- Règle d'intégration : la liste de la dernière migration qui la pose doit être l'union de tous les lots.
ALTER TABLE ent_structure_journal DROP CONSTRAINT IF EXISTS ent_structure_journal_objet_check;
ALTER TABLE ent_structure_journal ADD CONSTRAINT ent_structure_journal_objet_check
    CHECK (objet ~ '^[a-z_]{1,40}$');

-- ───────────────────────── RLS ─────────────────────────
DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_periodisation', 'ent_periodisation_cible'] LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('DROP POLICY IF EXISTS ent_rls_etablissement ON %I', t);
        EXECUTE format(
            'CREATE POLICY ent_rls_etablissement ON %I FOR ALL
                USING (etablissement_id = (SELECT ent_rls_etablissement())
                    OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
                WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
                    OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))', t);
    END LOOP;
END
$$;
