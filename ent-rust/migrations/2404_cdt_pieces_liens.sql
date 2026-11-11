-- Socle 20, lot C1 — migration 2404 : pièces jointes (objets du socle 7) et liens. Réf. : 20_CAHIER_DE_TEXTES.md §24.1, R-C9, R-C22.
-- Le nom `ent_cdt_piece` est pris par la table du lot S2-6 (150, avec `storage_key`) jusqu'à son retrait (2419) :
-- la table cible s'appelle `ent_cdt_piece_jointe` ; elle ne porte plus de clé de stockage (E5) mais une référence à
-- `ent_objet` (taille, empreinte SHA-256, type). Aucune donnée binaire en base.

CREATE TABLE IF NOT EXISTS ent_cdt_piece_jointe (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    objet_id UUID NOT NULL,
    porteur TEXT NOT NULL CHECK (porteur IN ('entree', 'contribution', 'travail', 'variante', 'depot', 'retour')),
    -- Référence polymorphe : l'existence du porteur, dans le même établissement, est garantie par le déclencheur
    -- ci-dessous (pas de clé étrangère possible vers six tables).
    porteur_id UUID NOT NULL,
    nom TEXT NOT NULL CHECK (length(btrim(nom)) BETWEEN 1 AND 255),
    ordre SMALLINT NOT NULL DEFAULT 0,
    retiree_le TIMESTAMPTZ,
    retiree_par_user_id UUID,
    ancien_id UUID UNIQUE,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_piece_jointe_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_piece_jointe_retrait_chk CHECK ((retiree_le IS NULL) = (retiree_par_user_id IS NULL)),
    CONSTRAINT ent_cdt_piece_jointe_objet_fk FOREIGN KEY (etablissement_id, objet_id)
        REFERENCES ent_objet (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_cdt_piece_jointe_porteur_idx
    ON ent_cdt_piece_jointe (etablissement_id, porteur, porteur_id) WHERE retiree_le IS NULL;

CREATE OR REPLACE FUNCTION ent_cdt_porteur_existe(p_etab UUID, p_porteur TEXT, p_id UUID) RETURNS BOOLEAN
LANGUAGE plpgsql STABLE AS $$
BEGIN
    RETURN CASE p_porteur
        WHEN 'entree' THEN EXISTS (SELECT 1 FROM ent_cdt_entree WHERE etablissement_id = p_etab AND id = p_id)
        WHEN 'contribution' THEN EXISTS (SELECT 1 FROM ent_cdt_contribution WHERE etablissement_id = p_etab AND id = p_id)
        WHEN 'travail' THEN EXISTS (SELECT 1 FROM ent_cdt_travail WHERE etablissement_id = p_etab AND id = p_id)
        WHEN 'variante' THEN EXISTS (SELECT 1 FROM ent_cdt_variante WHERE etablissement_id = p_etab AND id = p_id)
        WHEN 'depot' THEN EXISTS (SELECT 1 FROM ent_cdt_depot WHERE etablissement_id = p_etab AND id = p_id)
        WHEN 'retour' THEN EXISTS (SELECT 1 FROM ent_cdt_retour WHERE etablissement_id = p_etab AND id = p_id)
        ELSE false
    END;
END
$$;

CREATE OR REPLACE FUNCTION ent_cdt_piece_jointe_garde() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_cdt_piece_jointe : suppression interdite (retiree_le)' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'INSERT' OR NEW.porteur IS DISTINCT FROM OLD.porteur OR NEW.porteur_id IS DISTINCT FROM OLD.porteur_id THEN
        IF NOT ent_cdt_porteur_existe(NEW.etablissement_id, NEW.porteur, NEW.porteur_id) THEN
            RAISE EXCEPTION 'ent_cdt_piece_jointe : porteur % introuvable dans cet établissement', NEW.porteur
                USING ERRCODE = '23503';
        END IF;
    END IF;
    IF TG_OP = 'UPDATE' AND (NEW.objet_id IS DISTINCT FROM OLD.objet_id
                             OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
                             OR (OLD.retiree_le IS NOT NULL AND NEW.retiree_le IS DISTINCT FROM OLD.retiree_le)) THEN
        RAISE EXCEPTION 'ent_cdt_piece_jointe : l''objet d''une pièce ne change jamais' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS ent_cdt_piece_jointe_garde_trg ON ent_cdt_piece_jointe;
CREATE TRIGGER ent_cdt_piece_jointe_garde_trg BEFORE INSERT OR UPDATE OR DELETE ON ent_cdt_piece_jointe
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_piece_jointe_garde();

CREATE TABLE IF NOT EXISTS ent_cdt_lien (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    porteur TEXT NOT NULL CHECK (porteur IN ('entree', 'contribution', 'travail', 'variante')),
    porteur_id UUID NOT NULL,
    -- Liens https seulement (R-C22), jamais javascript: ni data:.
    url TEXT NOT NULL CHECK (url ~ '^https://[^\s]+$' AND length(url) <= 2000),
    libelle TEXT CHECK (libelle IS NULL OR length(btrim(libelle)) BETWEEN 1 AND 200),
    -- Ressource du GAR : sans clé (table d'un autre socle).
    ressource_gar_id UUID,
    ordre SMALLINT NOT NULL DEFAULT 0,
    retire_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_lien_etab_id_uq UNIQUE (etablissement_id, id)
);

CREATE OR REPLACE FUNCTION ent_cdt_lien_garde() RETURNS trigger AS $$
BEGIN
    IF NOT ent_cdt_porteur_existe(NEW.etablissement_id, NEW.porteur, NEW.porteur_id) THEN
        RAISE EXCEPTION 'ent_cdt_lien : porteur % introuvable dans cet établissement', NEW.porteur USING ERRCODE = '23503';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS ent_cdt_lien_garde_trg ON ent_cdt_lien;
CREATE TRIGGER ent_cdt_lien_garde_trg BEFORE INSERT ON ent_cdt_lien
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_lien_garde();

SELECT ent_rls_poser(ARRAY['ent_cdt_piece_jointe', 'ent_cdt_lien']);
