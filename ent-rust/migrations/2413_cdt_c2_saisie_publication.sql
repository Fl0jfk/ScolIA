-- Socle 20, lot C2 — migration 2413 : saisie et publication. Réf. : 20_CAHIER_DE_TEXTES.md §8.3 (plage calme), §9.1,
-- R-C5, R-C29, R-C30. Aucune nouvelle table (RLS et registre inchangés) ; aucune colonne JSON (D-036).

-- 1. Jour fermé pour la plage calme : fin de semaine, vacances, fermeture, férié de l'établissement (jour local).
CREATE OR REPLACE FUNCTION ent_cdt_jour_ferme(p_etab UUID, p_jour DATE)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
    SELECT EXTRACT(ISODOW FROM p_jour) >= 6
        OR EXISTS (SELECT 1 FROM ent_periode_vacances v
                    WHERE v.etablissement_id = p_etab AND p_jour BETWEEN v.debut AND v.fin)
        OR EXISTS (SELECT 1 FROM ent_jour_calendrier j
                    WHERE j.etablissement_id = p_etab AND j.jour = p_jour AND j.site_id IS NULL
                      AND j.kind IN ('ferme', 'ferie', 'vacances'))
$$;

-- 2. Fin de la plage calme à partir d'un instant (R-C5, arrêté du 7-9-2026) : défaut national 20 h–7 h du lundi au
-- vendredi, du vendredi 20 h au lundi 7 h, vacances. Hors plage : l'instant lui-même (visible tout de suite).
-- Heure et jour de l'établissement (fuseau de `ent_etablissement_parametre`, Europe/Paris par défaut).
CREATE OR REPLACE FUNCTION ent_cdt_visible_a(p_etab UUID, p_instant TIMESTAMPTZ)
RETURNS TIMESTAMPTZ LANGUAGE plpgsql STABLE AS $$
DECLARE
    v_fuseau TEXT := COALESCE((SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = p_etab),
                              'Europe/Paris');
    v_local TIMESTAMP := p_instant AT TIME ZONE v_fuseau;
    v_jour DATE := v_local::date;
    v_heure TIME := v_local::time;
    v_i INTEGER := 0;
BEGIN
    IF NOT ent_cdt_jour_ferme(p_etab, v_jour) AND v_heure >= TIME '07:00' AND v_heure < TIME '20:00' THEN
        RETURN p_instant;
    END IF;
    -- Avant 7 h d'un jour ouvré : le jour même à 7 h ; sinon le premier jour ouvré suivant à 7 h.
    IF NOT ent_cdt_jour_ferme(p_etab, v_jour) AND v_heure < TIME '07:00' THEN
        RETURN (v_jour + TIME '07:00') AT TIME ZONE v_fuseau;
    END IF;
    v_jour := v_jour + 1;
    WHILE ent_cdt_jour_ferme(p_etab, v_jour) AND v_i < 400 LOOP
        v_jour := v_jour + 1;
        v_i := v_i + 1;
    END LOOP;
    RETURN (v_jour + TIME '07:00') AT TIME ZONE v_fuseau;
END
$$;

-- 2 bis. Heure et jour de l'établissement : instant d'un jour et d'une heure locaux, et heure locale d'un instant.
-- Le client ne convertit rien et n'envoie jamais d'heure UTC saisie à la main.
CREATE OR REPLACE FUNCTION ent_cdt_instant_local(p_etab UUID, p_jour DATE, p_heure TIME)
RETURNS TIMESTAMPTZ LANGUAGE sql STABLE AS $$
    SELECT (p_jour + p_heure) AT TIME ZONE COALESCE(
        (SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = p_etab), 'Europe/Paris')
$$;

CREATE OR REPLACE FUNCTION ent_cdt_heure_locale(p_etab UUID, p_instant TIMESTAMPTZ)
RETURNS TIME LANGUAGE sql STABLE AS $$
    SELECT (p_instant AT TIME ZONE COALESCE(
        (SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = p_etab), 'Europe/Paris'))::time
$$;

-- 3. Versions lisibles des lecteurs (R-C29) : « modification mineure, sans avis » ne retire que l'avis.
-- `sans_avis` : la modification qui a remplacé cette version n'a pas prévenu les lecteurs.
ALTER TABLE ent_cdt_entree_version ADD COLUMN IF NOT EXISTS sans_avis BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_cdt_travail_version ADD COLUMN IF NOT EXISTS sans_avis BOOLEAN NOT NULL DEFAULT false;

-- 4. Copie de séance (CDT-118) : l'entrée d'origine est conservée en référence.
ALTER TABLE ent_cdt_entree ADD COLUMN IF NOT EXISTS copie_de_id UUID;
DO $$ BEGIN
    ALTER TABLE ent_cdt_entree ADD CONSTRAINT ent_cdt_entree_copie_fk FOREIGN KEY (etablissement_id, copie_de_id)
        REFERENCES ent_cdt_entree (etablissement_id, id) ON DELETE RESTRICT;
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

-- 5. Projection paresseuse des publications échues (programmées ou différées par la plage calme).
CREATE INDEX IF NOT EXISTS ent_cdt_entree_echu_idx ON ent_cdt_entree (etablissement_id, visible_a)
    WHERE ancien_id IS NULL AND etat IN ('programmee', 'publiee');
CREATE INDEX IF NOT EXISTS ent_cdt_travail_echu_idx ON ent_cdt_travail (etablissement_id, visible_a)
    WHERE ancien_id IS NULL AND public = 'cours' AND etat IN ('programme', 'publie');
