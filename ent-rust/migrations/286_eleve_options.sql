-- Socle 2 (l'élève), lot E4 / étape M6 : options et enseignements suivis = affectations à
-- des GROUPES du référentiel (une option est un groupe).
--
-- `ent_eleve_option` (dates) et `ent_eleve_option_annee` (propositions par année) ne sont pas
-- supprimées : un pont à sens unique (ancien → affectation) fait de chaque option héritée
-- une affectation de groupe, de sorte que les lecteurs qui passent aux affectations voient
-- TOUT. Les nouvelles commandes (changement d'options) écrivent les affectations.
-- `ent_eleve_option_groupe` relie chaque ligne héritée à son groupe et son affectation
-- (comparaison avant / après, idempotence). Les lignes qu'on ne sait pas reprendre sont
-- rangées dans `ent_eleve_option_a_traiter` (rien n'est supprimé).
--
-- Additif, idempotent, rejoué à chaque démarrage.

-- Les affectations issues d'options héritées ou des commandes portent la source `saisie` (la
-- contrainte de source d'`ent_affectation` est rejouée par la migration 110 à chaque démarrage :
-- on n'y ajoute aucune valeur). L'origine « option héritée » se lit dans `ent_eleve_option_groupe`.

CREATE TABLE IF NOT EXISTS ent_eleve_option_groupe (
    source            TEXT NOT NULL CHECK (source IN ('option', 'option_annee')),
    option_id         UUID NOT NULL,
    etablissement_id  UUID NOT NULL,
    eleve_id          UUID NOT NULL,
    groupe_id         UUID NOT NULL REFERENCES ent_groupe (id),
    affectation_id    UUID NOT NULL REFERENCES ent_affectation (id),
    cree_le           TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (source, option_id)
);
CREATE TABLE IF NOT EXISTS ent_eleve_option_a_traiter (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    etablissement_id UUID NOT NULL,
    eleve_id         UUID NOT NULL,
    source           TEXT NOT NULL,
    option_id        UUID NOT NULL,
    cause            TEXT NOT NULL,
    detecte_le       TIMESTAMPTZ NOT NULL DEFAULT now(),
    traite_le        TIMESTAMPTZ,
    UNIQUE (source, option_id, cause)
);
DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_eleve_option_groupe', 'ent_eleve_option_a_traiter']
    LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('DROP POLICY IF EXISTS ent_rls_etablissement ON %I', t);
        EXECUTE format($p$CREATE POLICY ent_rls_etablissement ON %I FOR ALL
            USING (etablissement_id = (SELECT ent_rls_etablissement())
                OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
            WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
                OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))$p$, t);
    END LOOP;
END $$;

-- Groupe de l'année portant ce code (créé s'il manque).
CREATE OR REPLACE FUNCTION ent_eleve_option_groupe_assurer(p_etab UUID, p_annee UUID, p_code TEXT, p_libelle TEXT)
RETURNS UUID LANGUAGE plpgsql AS $$
DECLARE v_id UUID; v_code TEXT := btrim(p_code);
BEGIN
    SELECT g.id INTO v_id FROM ent_groupe g WHERE g.etablissement_id = p_etab AND g.annee_id = p_annee AND g.code = v_code;
    IF v_id IS NULL THEN
        INSERT INTO ent_groupe (id, etablissement_id, annee_id, code, libelle)
        VALUES (gen_random_uuid(), p_etab, p_annee, v_code, coalesce(nullif(btrim(p_libelle), ''), v_code))
        ON CONFLICT DO NOTHING;
        SELECT g.id INTO v_id FROM ent_groupe g WHERE g.etablissement_id = p_etab AND g.annee_id = p_annee AND g.code = v_code;
    END IF;
    RETURN v_id;
END $$;

-- Option héritée (dates) → affectation de groupe, rattachée à l'année qui contient la date de début.
CREATE OR REPLACE FUNCTION ent_eleve_option_vers_affectation(o ent_eleve_option) RETURNS UUID
LANGUAGE plpgsql AS $$
DECLARE v_annee UUID; v_groupe UUID; v_aff UUID; v_fin DATE;
BEGIN
    IF btrim(coalesce(o.code, '')) = '' THEN
        INSERT INTO ent_eleve_option_a_traiter (etablissement_id, eleve_id, source, option_id, cause)
        VALUES (o.etablissement_id, o.eleve_id, 'option', o.id, 'code_vide') ON CONFLICT DO NOTHING;
        RETURN NULL;
    END IF;
    SELECT a.id, least(coalesce(o.date_fin, a.date_fin), a.date_fin) INTO v_annee, v_fin
    FROM ent_annee_scolaire a WHERE a.id = ent_assurer_annee(o.etablissement_id, o.date_debut);
    v_groupe := ent_eleve_option_groupe_assurer(o.etablissement_id, v_annee, o.code, o.libelle);
    SELECT affectation_id INTO v_aff FROM ent_eleve_option_groupe WHERE source = 'option' AND option_id = o.id;
    IF v_aff IS NULL THEN
        v_aff := gen_random_uuid();
        INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, groupe_id, date_debut, date_fin, source)
        VALUES (v_aff, o.etablissement_id, v_annee, o.eleve_id, v_groupe, o.date_debut,
                CASE WHEN v_fin >= o.date_debut THEN v_fin END, 'saisie');
        INSERT INTO ent_eleve_option_groupe (source, option_id, etablissement_id, eleve_id, groupe_id, affectation_id)
        VALUES ('option', o.id, o.etablissement_id, o.eleve_id, v_groupe, v_aff);
    ELSE
        UPDATE ent_affectation SET date_fin = CASE WHEN v_fin >= date_debut THEN v_fin END
        WHERE id = v_aff AND etablissement_id = o.etablissement_id;
    END IF;
    RETURN v_aff;
END $$;

-- Option retenue pour une année (propositions N+1 acceptées) → affectation sur l'année entière.
-- `proposee` n'est pas une affectation ; `retiree` clôt l'affectation à la date du jour.
CREATE OR REPLACE FUNCTION ent_eleve_option_annee_vers_affectation(o ent_eleve_option_annee) RETURNS UUID
LANGUAGE plpgsql AS $$
DECLARE v_an ent_annee_scolaire; v_groupe UUID; v_aff UUID;
BEGIN
    SELECT affectation_id INTO v_aff FROM ent_eleve_option_groupe WHERE source = 'option_annee' AND option_id = o.id;
    IF o.statut = 'proposee' THEN RETURN NULL; END IF;
    IF o.statut = 'retiree' THEN
        IF v_aff IS NOT NULL THEN
            UPDATE ent_affectation SET date_fin = least(coalesce(date_fin, current_date), greatest(date_debut, current_date))
            WHERE id = v_aff AND etablissement_id = o.etablissement_id AND (date_fin IS NULL OR date_fin > current_date);
        END IF;
        RETURN v_aff;
    END IF;
    IF v_aff IS NOT NULL THEN RETURN v_aff; END IF;
    SELECT a.* INTO v_an FROM ent_annee_scolaire a
    WHERE a.etablissement_id = o.etablissement_id AND a.libelle = o.annee_scolaire;
    IF v_an.id IS NULL OR btrim(coalesce(o.code, '')) = '' THEN
        INSERT INTO ent_eleve_option_a_traiter (etablissement_id, eleve_id, source, option_id, cause)
        VALUES (o.etablissement_id, o.eleve_id, 'option_annee', o.id,
                CASE WHEN v_an.id IS NULL THEN 'annee_inconnue' ELSE 'code_vide' END) ON CONFLICT DO NOTHING;
        RETURN NULL;
    END IF;
    v_groupe := ent_eleve_option_groupe_assurer(o.etablissement_id, v_an.id, o.code, o.libelle);
    v_aff := gen_random_uuid();
    INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, groupe_id, date_debut, date_fin, source)
    VALUES (v_aff, o.etablissement_id, v_an.id, o.eleve_id, v_groupe, v_an.date_debut, v_an.date_fin, 'saisie');
    INSERT INTO ent_eleve_option_groupe (source, option_id, etablissement_id, eleve_id, groupe_id, affectation_id)
    VALUES ('option_annee', o.id, o.etablissement_id, o.eleve_id, v_groupe, v_aff);
    RETURN v_aff;
END $$;

CREATE OR REPLACE FUNCTION ent_eleve_option_pont() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_TABLE_NAME = 'ent_eleve_option' THEN
        PERFORM ent_eleve_option_vers_affectation(NEW);
    ELSE
        PERFORM ent_eleve_option_annee_vers_affectation(NEW);
    END IF;
    RETURN NULL;
END $$;
DROP TRIGGER IF EXISTS ent_eleve_option_pont ON ent_eleve_option;
CREATE TRIGGER ent_eleve_option_pont AFTER INSERT OR UPDATE ON ent_eleve_option
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_option_pont();
DROP TRIGGER IF EXISTS ent_eleve_option_annee_pont ON ent_eleve_option_annee;
CREATE TRIGGER ent_eleve_option_annee_pont AFTER INSERT OR UPDATE ON ent_eleve_option_annee
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_option_pont();

-- Reprise idempotente de l'existant ; renvoie le nombre d'affectations créées.
CREATE OR REPLACE FUNCTION ent_eleve_option_reprendre() RETURNS BIGINT
LANGUAGE plpgsql AS $$
DECLARE n BIGINT := 0; o ent_eleve_option; a ent_eleve_option_annee;
BEGIN
    FOR o IN SELECT * FROM ent_eleve_option x
             WHERE NOT EXISTS (SELECT 1 FROM ent_eleve_option_groupe g WHERE g.source = 'option' AND g.option_id = x.id)
               AND NOT EXISTS (SELECT 1 FROM ent_eleve_option_a_traiter t WHERE t.source = 'option' AND t.option_id = x.id)
             ORDER BY x.created_at, x.id
    LOOP
        IF ent_eleve_option_vers_affectation(o) IS NOT NULL THEN n := n + 1; END IF;
    END LOOP;
    FOR a IN SELECT * FROM ent_eleve_option_annee x
             WHERE x.statut = 'retenue'
               AND NOT EXISTS (SELECT 1 FROM ent_eleve_option_groupe g WHERE g.source = 'option_annee' AND g.option_id = x.id)
               AND NOT EXISTS (SELECT 1 FROM ent_eleve_option_a_traiter t WHERE t.source = 'option_annee' AND t.option_id = x.id)
             ORDER BY x.created_at, x.id
    LOOP
        IF ent_eleve_option_annee_vers_affectation(a) IS NOT NULL THEN n := n + 1; END IF;
    END LOOP;
    RETURN n;
END $$;

SELECT ent_eleve_option_reprendre();

-- Comparaison avant / après : une ligne par ancienne option reprise ou à traiter.
CREATE OR REPLACE FUNCTION ent_eleve_option_comparaison(p_etab UUID)
RETURNS TABLE (heritees BIGINT, reprises BIGINT, a_traiter BIGINT) LANGUAGE sql STABLE AS $$
    SELECT
        (SELECT count(*) FROM ent_eleve_option WHERE etablissement_id = p_etab)
          + (SELECT count(*) FROM ent_eleve_option_annee WHERE etablissement_id = p_etab AND statut = 'retenue'),
        (SELECT count(*) FROM ent_eleve_option_groupe WHERE etablissement_id = p_etab),
        (SELECT count(*) FROM ent_eleve_option_a_traiter WHERE etablissement_id = p_etab AND traite_le IS NULL)
$$;
