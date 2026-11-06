-- Socle 2, lot S2-7 : conseil de classe outillé (EVA-08, EVA-09), avis du conseil
-- sur les vœux et réponse de la famille (ORI-02, ORI-03), bilan de fin de cycle
-- (EVA-11). Migration ADDITIVE.

ALTER TABLE ent_conseil_classe ADD COLUMN IF NOT EXISTS periode_id UUID REFERENCES ent_note_periode (id);
ALTER TABLE ent_conseil_classe ADD COLUMN IF NOT EXISTS president_user_id UUID REFERENCES ent_user (id);

-- Décision du conseil par élève [H : liste à vérifier sur les textes officiels].
ALTER TABLE ent_conseil_classe_dossier ADD COLUMN IF NOT EXISTS decision_code TEXT;
ALTER TABLE ent_conseil_classe_dossier ADD COLUMN IF NOT EXISTS avis_orientation TEXT;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_conseil_dossier_decision_chk') THEN
        ALTER TABLE ent_conseil_classe_dossier ADD CONSTRAINT ent_conseil_dossier_decision_chk
            CHECK (decision_code IS NULL OR decision_code IN (
                'aucune', 'passage', 'redoublement', 'orientation', 'reserve'));
    END IF;
END
$$;

-- Avis du conseil sur un vœu d'orientation, puis réponse de la famille.
CREATE TABLE IF NOT EXISTS ent_orientation_avis (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    voeu_id UUID NOT NULL REFERENCES ent_voeu_inscription (id) ON DELETE CASCADE,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    conseil_id UUID NOT NULL REFERENCES ent_conseil_classe (id),
    avis TEXT NOT NULL,
    motivation TEXT NOT NULL DEFAULT '',
    donne_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    donne_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    reponse_famille TEXT,
    reponse_par_user_id UUID REFERENCES ent_user (id),
    reponse_le TIMESTAMPTZ,
    CONSTRAINT ent_orientation_avis_uq UNIQUE (etablissement_id, voeu_id, conseil_id),
    CONSTRAINT ent_orientation_avis_chk CHECK (avis IN ('favorable', 'reserve', 'defavorable')),
    CONSTRAINT ent_orientation_reponse_chk
        CHECK (reponse_famille IS NULL OR reponse_famille IN ('accord', 'desaccord'))
);
CREATE INDEX IF NOT EXISTS ent_orientation_avis_eleve_idx
    ON ent_orientation_avis (etablissement_id, eleve_id);

-- Bilan de fin de cycle : 8 composantes du socle, niveau 1 à 4 [NV : codes LSU].
CREATE TABLE IF NOT EXISTS ent_bilan_cycle (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    annee_id UUID NOT NULL REFERENCES ent_annee_scolaire (id),
    cycle SMALLINT NOT NULL,
    synthese TEXT NOT NULL DEFAULT '',
    statut TEXT NOT NULL DEFAULT 'brouillon',
    saisi_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    valide_par_user_id UUID REFERENCES ent_user (id),
    valide_le TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_bilan_cycle_uq UNIQUE (etablissement_id, eleve_id, annee_id, cycle),
    CONSTRAINT ent_bilan_cycle_cycle_chk CHECK (cycle IN (2, 3, 4)),
    CONSTRAINT ent_bilan_cycle_statut_chk CHECK (statut IN ('brouillon', 'valide'))
);

CREATE TABLE IF NOT EXISTS ent_bilan_cycle_composante (
    etablissement_id UUID NOT NULL,
    bilan_id UUID NOT NULL REFERENCES ent_bilan_cycle (id) ON DELETE CASCADE,
    composante TEXT NOT NULL,
    niveau SMALLINT NOT NULL,
    PRIMARY KEY (bilan_id, composante),
    CONSTRAINT ent_bilan_composante_chk CHECK (composante IN (
        'CPD_FRA', 'CPD_ETR', 'CPD_SCI', 'CPD_ART', 'MET_APP', 'FRM_CIT', 'SYS_NAT', 'REP_MND')),
    CONSTRAINT ent_bilan_niveau_chk CHECK (niveau BETWEEN 1 AND 4)
);
