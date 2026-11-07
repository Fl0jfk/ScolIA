-- Socle 2 (l'élève), lot E3 / étape M4 : numéro d'élève de l'établissement.
--
-- Numéro interne, attribué à la création (trigger : tout chemin d'insertion), unique DANS
-- l'établissement, jamais réutilisé (compteur monotone, aucune suppression d'élève) et
-- immuable. Format paramétrable par établissement (`ent_param_eleve_numero`) ; le réglage
-- par l'administrateur arrive avec le lot E5 (paramétrage) — défaut : `{AAAA}-{seq:5}`
-- (année d'attribution, compteur remis à zéro chaque année).
--
-- Additif, idempotent, rejoué à chaque démarrage.

ALTER TABLE ent_eleve ADD COLUMN IF NOT EXISTS numero TEXT;

CREATE TABLE IF NOT EXISTS ent_param_eleve_numero (
    etablissement_id        UUID PRIMARY KEY,
    format                  TEXT NOT NULL DEFAULT '{AAAA}-{seq:5}',
    remise_a_zero_annuelle  BOOLEAN NOT NULL DEFAULT true,
    modifie_le              TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_par             UUID,
    -- Un format doit contenir un compteur `{seq:N}` (N de 1 à 9), n'utiliser que
    -- lettres, chiffres et `{}:_-`, et — si le compteur est remis à zéro chaque année —
    -- contenir `{AAAA}` (sinon deux années donneraient le même numéro).
    CONSTRAINT ent_param_eleve_numero_format_chk CHECK (
        char_length(format) <= 40
        AND format ~ '^[A-Za-z0-9{}:_-]+$'
        AND format ~ '\{seq:[1-9]\}'
        AND (NOT remise_a_zero_annuelle OR format LIKE '%{AAAA}%')
    )
);
ALTER TABLE ent_param_eleve_numero ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_param_eleve_numero FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_param_eleve_numero;
CREATE POLICY ent_rls_etablissement ON ent_param_eleve_numero FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));

CREATE TABLE IF NOT EXISTS ent_eleve_numero_compteur (
    etablissement_id UUID NOT NULL,
    cle              INTEGER NOT NULL,
    derniere         BIGINT NOT NULL,
    PRIMARY KEY (etablissement_id, cle)
);
ALTER TABLE ent_eleve_numero_compteur ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_eleve_numero_compteur FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_eleve_numero_compteur;
CREATE POLICY ent_rls_etablissement ON ent_eleve_numero_compteur FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));

-- Attribue le numéro suivant (compteur atomique par établissement et par année).
CREATE OR REPLACE FUNCTION ent_eleve_numero_attribuer(p_etab UUID, p_jour DATE) RETURNS TEXT
LANGUAGE plpgsql AS $$
DECLARE
    fmt TEXT := '{AAAA}-{seq:5}';
    raz BOOLEAN := true;
    v_annee INTEGER := extract(year FROM p_jour)::int;
    v_cle INTEGER;
    v_seq BIGINT;
    v_largeur INTEGER;
    v_compteur TEXT;
BEGIN
    SELECT p.format, p.remise_a_zero_annuelle INTO fmt, raz
    FROM ent_param_eleve_numero p WHERE p.etablissement_id = p_etab;
    IF NOT FOUND THEN fmt := '{AAAA}-{seq:5}'; raz := true; END IF;
    v_cle := CASE WHEN raz THEN v_annee ELSE 0 END;
    INSERT INTO ent_eleve_numero_compteur AS c (etablissement_id, cle, derniere)
    VALUES (p_etab, v_cle, 1)
    ON CONFLICT (etablissement_id, cle) DO UPDATE SET derniere = c.derniere + 1
    RETURNING c.derniere INTO v_seq;
    v_largeur := substring(fmt FROM '\{seq:([1-9])\}')::int;
    -- Au-delà de la largeur, le numéro s'allonge (jamais tronqué : l'unicité prime).
    v_compteur := CASE WHEN char_length(v_seq::text) >= v_largeur THEN v_seq::text ELSE lpad(v_seq::text, v_largeur, '0') END;
    RETURN regexp_replace(replace(fmt, '{AAAA}', v_annee::text), '\{seq:[1-9]\}', v_compteur);
END $$;

-- Tout insert reçoit un numéro, sauf la branche « existe déjà » d'un
-- INSERT … ON CONFLICT DO UPDATE (on ne brûle pas un numéro à chaque mise à jour).
CREATE OR REPLACE FUNCTION ent_eleve_numero_avant_insert() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.numero IS NULL AND NOT EXISTS (SELECT 1 FROM ent_eleve e WHERE e.id = NEW.id) THEN
        NEW.numero := ent_eleve_numero_attribuer(NEW.etablissement_id, current_date);
    END IF;
    RETURN NEW;
END $$;

DROP TRIGGER IF EXISTS ent_eleve_numero_avant_insert ON ent_eleve;
CREATE TRIGGER ent_eleve_numero_avant_insert
    BEFORE INSERT ON ent_eleve
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_numero_avant_insert();

-- Immuable : posé une fois (NULL → valeur), jamais modifié ni retiré.
CREATE OR REPLACE FUNCTION ent_eleve_numero_immuable() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF OLD.numero IS NOT NULL AND NEW.numero IS DISTINCT FROM OLD.numero THEN
        RAISE EXCEPTION 'numéro d''élève : immuable' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END $$;

DROP TRIGGER IF EXISTS ent_eleve_numero_immuable ON ent_eleve;
CREATE TRIGGER ent_eleve_numero_immuable
    BEFORE UPDATE ON ent_eleve
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_numero_immuable();

-- Reprise : numéro attribué aux élèves existants, dans l'ordre de création, avec
-- l'année de leur création. Idempotente (ne traite que les élèves sans numéro).
CREATE OR REPLACE FUNCTION ent_eleve_numero_reprendre() RETURNS BIGINT
LANGUAGE plpgsql AS $$
DECLARE n BIGINT := 0; r RECORD;
BEGIN
    FOR r IN SELECT etablissement_id, id, created_at FROM ent_eleve WHERE numero IS NULL
             ORDER BY etablissement_id, created_at, id
    LOOP
        UPDATE ent_eleve SET numero = ent_eleve_numero_attribuer(r.etablissement_id, r.created_at::date)
        WHERE etablissement_id = r.etablissement_id AND id = r.id AND numero IS NULL;
        n := n + 1;
    END LOOP;
    RETURN n;
END $$;

SELECT ent_eleve_numero_reprendre();

-- Unicité DANS l'établissement (créée après la reprise).
CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_numero_uniq
    ON ent_eleve (etablissement_id, numero) WHERE numero IS NOT NULL;
