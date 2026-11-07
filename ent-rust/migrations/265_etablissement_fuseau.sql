-- Socle 4, lot A0 bis, A0-a (`docs/socles/04_ADMINISTRATIF.md` R-A38, `PERSONAS_02-04.md` §0).
--
-- Fuseau horaire de l'établissement : les dates métier (vue `ent_eleve_lien_actif`, bord de
-- journée d'un lien) ne sont plus calculées en Europe/Paris « en dur » mais dans le fuseau du
-- paramètre d'établissement (La Réunion, Guyane, Mayotte, établissements à l'étranger).
-- Europe/Paris par défaut : un établissement sans ligne garde exactement le comportement du lot A0.
-- Cette table est le paramètre minimal du lot ; le socle 5 (STR-03) en fera un écran.
-- Migration additive et rejouable.

CREATE TABLE IF NOT EXISTS ent_etablissement_parametre (
    etablissement_id UUID PRIMARY KEY
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    -- Nom IANA (« Indian/Reunion »). `timezone()` lève une erreur sur un nom inconnu : la contrainte
    -- refuse donc tout fuseau que PostgreSQL ne connaît pas.
    fuseau TEXT NOT NULL DEFAULT 'Europe/Paris'
        CHECK (length(fuseau) BETWEEN 1 AND 64
               AND timezone(fuseau, TIMESTAMPTZ '2000-01-01 00:00:00+00') IS NOT NULL),
    updated_by UUID,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Jour métier d'un établissement. Établissement sans paramètre : Europe/Paris.
CREATE OR REPLACE FUNCTION ent_aujourdhui_etab(etab UUID, instant TIMESTAMPTZ DEFAULT now())
RETURNS DATE
LANGUAGE sql STABLE
AS $$
    SELECT (instant AT TIME ZONE COALESCE(
        (SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = etab),
        'Europe/Paris'))::date
$$;

-- Même définition que la migration 260 (mêmes colonnes) ; seul le jour métier change.
CREATE OR REPLACE VIEW ent_eleve_lien_actif AS
SELECT l.*
FROM ent_eleve_lien l
JOIN ent_personne p
  ON p.id = l.personne_id AND p.etablissement_id = l.etablissement_id
WHERE l.suspendu_le IS NULL
  AND (l.date_debut IS NULL OR l.date_debut <= ent_aujourdhui_etab(l.etablissement_id))
  AND (l.date_fin IS NULL OR l.date_fin >= ent_aujourdhui_etab(l.etablissement_id))
  AND p.fusionnee_dans IS NULL;
