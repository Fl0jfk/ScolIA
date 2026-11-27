-- D-036 (lot 2) — `ent_parametre.valeur` (JSONB) devient trois colonnes typées selon le type
-- déclaré au registre du code (entier, booléen, heure) : `type_param` + `valeur_entier` /
-- `valeur_booleen` / `valeur_heure`, avec une contrainte « une seule valeur, du bon type ».
-- Le JSON d'échange (`valeur`) est reconstruit à la lecture.
--
-- Reprise idempotente : seules les lignes SANS type sont reprises (valeur d'un type inconnu ou
-- hors format ignorée : la ligne reste sans type, donc lue comme « jamais modifiée »).
-- L'ancienne colonne devient facultative et en LECTURE SEULE (déclencheur).

ALTER TABLE ent_parametre ADD COLUMN IF NOT EXISTS type_param TEXT;
ALTER TABLE ent_parametre ADD COLUMN IF NOT EXISTS valeur_entier BIGINT;
ALTER TABLE ent_parametre ADD COLUMN IF NOT EXISTS valeur_booleen BOOLEAN;
ALTER TABLE ent_parametre ADD COLUMN IF NOT EXISTS valeur_heure TIME;
ALTER TABLE ent_parametre ALTER COLUMN valeur DROP NOT NULL;

ALTER TABLE ent_parametre DROP CONSTRAINT IF EXISTS ent_parametre_valeur_typee_chk;
ALTER TABLE ent_parametre ADD CONSTRAINT ent_parametre_valeur_typee_chk CHECK (
    (type_param IS NULL AND valeur_entier IS NULL AND valeur_booleen IS NULL AND valeur_heure IS NULL)
    OR (type_param = 'entier' AND valeur_entier IS NOT NULL AND valeur_booleen IS NULL AND valeur_heure IS NULL)
    OR (type_param = 'booleen' AND valeur_booleen IS NOT NULL AND valeur_entier IS NULL AND valeur_heure IS NULL)
    OR (type_param = 'heure' AND valeur_heure IS NOT NULL AND valeur_entier IS NULL AND valeur_booleen IS NULL));

UPDATE ent_parametre SET type_param = 'entier', valeur_entier = (valeur #>> '{}')::bigint
 WHERE type_param IS NULL AND jsonb_typeof(valeur) = 'number' AND (valeur #>> '{}') ~ '^-?[0-9]{1,15}$';
UPDATE ent_parametre SET type_param = 'booleen', valeur_booleen = (valeur #>> '{}')::boolean
 WHERE type_param IS NULL AND jsonb_typeof(valeur) = 'boolean';
UPDATE ent_parametre SET type_param = 'heure', valeur_heure = (valeur #>> '{}')::time
 WHERE type_param IS NULL AND jsonb_typeof(valeur) = 'string' AND (valeur #>> '{}') ~ '^([01][0-9]|2[0-3]):[0-5][0-9]$';

-- Valeur au format d'échange de l'API (NULL pour une ligne sans type).
CREATE OR REPLACE FUNCTION ent_parametre_valeur_json(p ent_parametre) RETURNS jsonb
LANGUAGE sql IMMUTABLE AS $$
    SELECT CASE p.type_param
        WHEN 'entier' THEN to_jsonb(p.valeur_entier)
        WHEN 'booleen' THEN to_jsonb(p.valeur_booleen)
        WHEN 'heure' THEN to_jsonb(to_char(p.valeur_heure, 'HH24:MI'))
    END
$$;

CREATE OR REPLACE FUNCTION ent_parametre_valeur_lecture_seule() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.valeur IS NOT NULL AND (TG_OP = 'INSERT' OR NEW.valeur IS DISTINCT FROM OLD.valeur) THEN
        RAISE EXCEPTION 'ent_parametre.valeur est en lecture seule (D-036) : utiliser les colonnes typées'
            USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_parametre_valeur_lecture_seule ON ent_parametre;
CREATE TRIGGER ent_parametre_valeur_lecture_seule
    BEFORE INSERT OR UPDATE OF valeur ON ent_parametre
    FOR EACH ROW EXECUTE FUNCTION ent_parametre_valeur_lecture_seule();
