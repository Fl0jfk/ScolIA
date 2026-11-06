-- Socle 2, lot S2-4 : discipline (VS-16 à VS-18) et signalement d'assiduité (VS-21).
-- Délais, seuils et règles d'effacement : [H] à vérifier sur les textes officiels,
-- paramétrables par établissement (D-019 : public et privé).

ALTER TABLE ent_sanction_type
    ADD COLUMN IF NOT EXISTS categorie TEXT;
ALTER TABLE ent_sanction_type DROP CONSTRAINT IF EXISTS ent_sanction_type_categorie_chk;
ALTER TABLE ent_sanction_type
    ADD CONSTRAINT ent_sanction_type_categorie_chk CHECK (categorie IS NULL OR categorie IN (
        'avertissement', 'blame', 'mesure_responsabilisation',
        'exclusion_temporaire_classe', 'exclusion_temporaire_etablissement',
        'exclusion_definitive', 'autre'
    ));

ALTER TABLE ent_sanction ADD COLUMN IF NOT EXISTS effacee_at TIMESTAMPTZ;
ALTER TABLE ent_sanction ADD COLUMN IF NOT EXISTS effacee_motif TEXT;

CREATE TABLE IF NOT EXISTS ent_regle_effacement (
    etablissement_id UUID NOT NULL,
    categorie TEXT NOT NULL,
    mode TEXT NOT NULL,
    delai_mois INTEGER,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, categorie),
    CONSTRAINT ent_regle_effacement_mode_chk CHECK (mode IN ('fin_annee_scolaire', 'delai_mois', 'jamais')),
    CONSTRAINT ent_regle_effacement_delai_chk CHECK (
        (mode = 'delai_mois' AND delai_mois BETWEEN 1 AND 120) OR (mode <> 'delai_mois')
    )
);

CREATE TABLE IF NOT EXISTS ent_procedure_disciplinaire (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    incident_id UUID REFERENCES ent_incident (id) ON DELETE SET NULL,
    -- 'chef_etablissement' : décision du chef d'établissement après procédure
    -- contradictoire ; 'conseil_discipline' : conseil convoqué avec délai.
    instance TEXT NOT NULL,
    statut TEXT NOT NULL DEFAULT 'engagee',
    faits TEXT NOT NULL,
    engagee_le DATE NOT NULL,
    engagee_par_user_id UUID NOT NULL,
    observations_recueillies_le DATE,
    convocation_envoyee_le DATE,
    conseil_le DATE,
    delai_convocation_jours INTEGER NOT NULL DEFAULT 8,
    decision TEXT,
    decision_le DATE,
    decision_par_user_id UUID,
    sanction_id UUID REFERENCES ent_sanction (id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_procedure_instance_chk CHECK (instance IN ('chef_etablissement', 'conseil_discipline')),
    CONSTRAINT ent_procedure_statut_chk CHECK (statut IN ('engagee', 'convoquee', 'decidee', 'close'))
);

CREATE INDEX IF NOT EXISTS ent_procedure_eleve_idx
    ON ent_procedure_disciplinaire (etablissement_id, eleve_id, engagee_le DESC);

CREATE TABLE IF NOT EXISTS ent_signalement_assiduite (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    mois DATE NOT NULL,
    demi_journees INTEGER NOT NULL,
    seuil INTEGER NOT NULL,
    statut TEXT NOT NULL DEFAULT 'propose',
    propose_par_user_id UUID,
    decide_par_user_id UUID,
    decide_at TIMESTAMPTZ,
    dossier JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_signalement_statut_chk CHECK (statut IN ('propose', 'valide', 'refuse', 'emis')),
    CONSTRAINT ent_signalement_mois_chk CHECK (EXTRACT(DAY FROM mois) = 1),
    CONSTRAINT ent_signalement_uq UNIQUE (etablissement_id, eleve_id, mois)
);
