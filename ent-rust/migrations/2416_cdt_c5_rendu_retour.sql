-- Socle 20, lot C5 — migration 2416 : rendu et retour. Réf. : 20_CAHIER_DE_TEXTES.md §10, R-C9 à R-C13, CC-31 à CC-45,
-- CDT-184 à CDT-201. Additif et rejouable ; aucune migration existante n'est modifiée ; aucune colonne JSON (D-036).

-- 1. Brouillon de remise (§10.1) : l'élève prépare un texte et une réflexion avant de déposer ; seul le dépôt compte.
-- Les fichiers en attente sont des pièces dont le porteur est la remise (étape 2) ; le dépôt les rattache au dépôt.
ALTER TABLE ent_cdt_remise ADD COLUMN IF NOT EXISTS brouillon_texte TEXT
    CHECK (brouillon_texte IS NULL OR length(brouillon_texte) <= 20000);
ALTER TABLE ent_cdt_remise ADD COLUMN IF NOT EXISTS brouillon_commentaire TEXT
    CHECK (brouillon_commentaire IS NULL OR length(brouillon_commentaire) <= 2000);
ALTER TABLE ent_cdt_remise ADD COLUMN IF NOT EXISTS brouillon_le TIMESTAMPTZ;

-- 2. Une pièce peut avoir pour porteur la remise (fichiers du brouillon).
ALTER TABLE ent_cdt_piece_jointe DROP CONSTRAINT IF EXISTS ent_cdt_piece_jointe_porteur_check;
ALTER TABLE ent_cdt_piece_jointe ADD CONSTRAINT ent_cdt_piece_jointe_porteur_check
    CHECK (porteur IN ('entree', 'contribution', 'travail', 'variante', 'depot', 'retour', 'remise'));
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
        WHEN 'remise' THEN EXISTS (SELECT 1 FROM ent_cdt_remise WHERE etablissement_id = p_etab AND id = p_id)
        ELSE false
    END;
END
$$;

-- 3. Panne du service (R-C11) : une déclaration de la direction ; la prolongation d'un travail n'est faite qu'une fois
-- par panne (`panne_id` sur la prolongation individuelle, qui reste en ajout seul).
CREATE TABLE IF NOT EXISTS ent_cdt_panne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    debut_le TIMESTAMPTZ NOT NULL,
    fin_le TIMESTAMPTZ NOT NULL,
    declare_par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_panne_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_panne_duree_chk CHECK (fin_le > debut_le AND fin_le - debut_le <= interval '3 days')
);
ALTER TABLE ent_cdt_echeance_eleve ADD COLUMN IF NOT EXISTS panne_id UUID;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_cdt_echeance_eleve_panne_fk') THEN
        ALTER TABLE ent_cdt_echeance_eleve ADD CONSTRAINT ent_cdt_echeance_eleve_panne_fk
            FOREIGN KEY (etablissement_id, panne_id) REFERENCES ent_cdt_panne (etablissement_id, id) ON DELETE RESTRICT;
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS ent_cdt_echeance_eleve_panne_uq
    ON ent_cdt_echeance_eleve (etablissement_id, travail_id, eleve_id, panne_id) WHERE panne_id IS NOT NULL;

-- 4. Relance des non-rendus (CDT-196) : une trace par élève et par envoi, pour ne pas relancer deux fois le même jour.
CREATE TABLE IF NOT EXISTS ent_cdt_relance (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    travail_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    aux_familles BOOLEAN NOT NULL DEFAULT false,
    jour DATE NOT NULL,
    par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_relance_jour_uq UNIQUE (travail_id, eleve_id, jour),
    CONSTRAINT ent_cdt_relance_travail_fk FOREIGN KEY (etablissement_id, travail_id)
        REFERENCES ent_cdt_travail (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_relance_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id) ON DELETE RESTRICT
);

SELECT ent_rls_poser(ARRAY['ent_cdt_panne', 'ent_cdt_relance']);

-- 5. Une seule demande de délai ouverte par élève et par travail ; index du suivi.
CREATE UNIQUE INDEX IF NOT EXISTS ent_cdt_delai_demande_ouverte_uq
    ON ent_cdt_delai_demande (etablissement_id, travail_id, eleve_id) WHERE etat = 'ouverte';
CREATE INDEX IF NOT EXISTS ent_cdt_depot_remise_idx ON ent_cdt_depot (etablissement_id, remise_id, numero DESC);
CREATE INDEX IF NOT EXISTS ent_cdt_depot_signalement_idx ON ent_cdt_depot_signalement (etablissement_id, travail_id, eleve_id, cree_le DESC);
