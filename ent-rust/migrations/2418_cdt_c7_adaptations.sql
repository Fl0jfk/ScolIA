-- Socle 20, lot C7 — migration 2418 : adaptations. Réf. : 20_CAHIER_DE_TEXTES.md §13, R-C8, R-C36, CDT-266, CDT-267, CDT-284.
-- Additif et rejouable ; aucune migration existante n'est modifiée ; aucune colonne JSON (D-036).
-- La plage 2414–2419 donnée au lot : 2414 (C3), 2415 (C4), 2416 (C5), 2417 (C6), 2418 (C7) ; 2419 reste libre.

-- 1. Une variante est **la seule version** que voit l'élève : jamais un indicateur, jamais un compteur côté élève. Un élève n'a qu'une
-- variante active par travail (garde d'intégrité).
CREATE OR REPLACE FUNCTION ent_cdt_variante_unique() RETURNS trigger AS $$
BEGIN
    IF NEW.retiree_le IS NULL AND EXISTS (
        SELECT 1 FROM ent_cdt_variante_eleve ve
          JOIN ent_cdt_variante v ON v.etablissement_id = ve.etablissement_id AND v.id = ve.variante_id
          JOIN ent_cdt_variante n ON n.etablissement_id = NEW.etablissement_id AND n.id = NEW.variante_id AND n.travail_id = v.travail_id
         WHERE ve.etablissement_id = NEW.etablissement_id AND ve.eleve_id = NEW.eleve_id AND ve.retiree_le IS NULL
           AND NOT (ve.variante_id = NEW.variante_id)) THEN
        RAISE EXCEPTION 'ent_cdt_variante_eleve : l''élève a déjà une variante active pour ce travail' USING ERRCODE = '23505';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS ent_cdt_variante_unique_trg ON ent_cdt_variante_eleve;
CREATE TRIGGER ent_cdt_variante_unique_trg BEFORE INSERT OR UPDATE ON ent_cdt_variante_eleve
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_variante_unique();

-- Variante active d'un élève pour un travail (ou aucune).
CREATE OR REPLACE FUNCTION ent_cdt_variante_de(p_etab UUID, p_travail UUID, p_eleve UUID)
RETURNS UUID LANGUAGE sql STABLE AS $$
    SELECT v.id FROM ent_cdt_variante v
      JOIN ent_cdt_variante_eleve ve ON ve.etablissement_id = v.etablissement_id AND ve.variante_id = v.id
     WHERE v.etablissement_id = p_etab AND v.travail_id = p_travail AND ve.eleve_id = p_eleve AND ve.retiree_le IS NULL
     ORDER BY ve.attribuee_le DESC LIMIT 1
$$;

-- 2. Échéance effective : la prolongation individuelle la plus récente, sinon l'échéance de la variante de l'élève, sinon celle du travail.
CREATE OR REPLACE FUNCTION ent_cdt_echeance_effective(p_etab UUID, p_travail UUID, p_eleve UUID, p_echeance TIMESTAMPTZ)
RETURNS TIMESTAMPTZ LANGUAGE sql STABLE AS $$
    SELECT COALESCE(
        (SELECT x.echeance_le FROM ent_cdt_echeance_eleve x
          WHERE x.etablissement_id = p_etab AND x.travail_id = p_travail AND x.eleve_id = p_eleve
          ORDER BY x.cree_le DESC, x.id DESC LIMIT 1),
        (SELECT v.echeance_le FROM ent_cdt_variante v WHERE v.etablissement_id = p_etab AND v.id = ent_cdt_variante_de(p_etab, p_travail, p_eleve)),
        p_echeance)
$$;

-- 3. Réglages d'affichage de l'élève (accessibilité, socle 11) : sans intervention du professeur, jamais lus par un adulte.
CREATE TABLE IF NOT EXISTS ent_cdt_affichage (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    taille_police TEXT NOT NULL DEFAULT 'normale' CHECK (taille_police IN ('normale', 'grande', 'tres_grande')),
    interligne TEXT NOT NULL DEFAULT 'normal' CHECK (interligne IN ('normal', 'large')),
    lecture_audio BOOLEAN NOT NULL DEFAULT false,
    animations_reduites BOOLEAN NOT NULL DEFAULT false,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (eleve_id),
    CONSTRAINT ent_cdt_affichage_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id) ON DELETE RESTRICT
);
SELECT ent_rls_poser(ARRAY['ent_cdt_affichage']);

-- 4. Index des notes d'accompagnement (lecture par travail ou par entrée).
CREATE INDEX IF NOT EXISTS ent_cdt_note_accompagnement_travail_idx ON ent_cdt_note_accompagnement (etablissement_id, travail_id) WHERE travail_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_cdt_note_accompagnement_entree_idx ON ent_cdt_note_accompagnement (etablissement_id, entree_id) WHERE entree_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_cdt_variante_travail_idx ON ent_cdt_variante (etablissement_id, travail_id);
