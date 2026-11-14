-- Grille repas / suivi (cantine, étude…) + bourses élève.
-- Dictée dossier : cases mange/pas mange configurable ; bourses + procuration → déduction facture.
-- Forme Charlemagne : PA_GRILLE_REGIME / FAC_BOURSE_* — modèle souverain, pas de miroir 1:1.

CREATE TABLE IF NOT EXISTS ent_repas_activite (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    kind TEXT NOT NULL,
    actif BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_repas_activite_kind_chk
        CHECK (kind IN ('midi', 'soir', 'etude', 'garderie', 'autre')),
    CONSTRAINT ent_repas_activite_code_uq UNIQUE (etablissement_id, code)
);

CREATE INDEX IF NOT EXISTS ent_repas_activite_etab_idx
    ON ent_repas_activite (etablissement_id);

-- Inscription élève à une activité + masque jours (lun→dim) + dates d'effet.
CREATE TABLE IF NOT EXISTS ent_eleve_grille_repas (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    activite_id UUID NOT NULL REFERENCES ent_repas_activite (id) ON DELETE RESTRICT,
    lun BOOLEAN NOT NULL DEFAULT false,
    mar BOOLEAN NOT NULL DEFAULT false,
    mer BOOLEAN NOT NULL DEFAULT false,
    jeu BOOLEAN NOT NULL DEFAULT false,
    ven BOOLEAN NOT NULL DEFAULT false,
    sam BOOLEAN NOT NULL DEFAULT false,
    dim BOOLEAN NOT NULL DEFAULT false,
    date_debut DATE NOT NULL,
    date_fin DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_grille_repas_fin_chk
        CHECK (date_fin IS NULL OR date_fin >= date_debut)
);

CREATE INDEX IF NOT EXISTS ent_eleve_grille_repas_eleve_idx
    ON ent_eleve_grille_repas (etablissement_id, eleve_id);

CREATE INDEX IF NOT EXISTS ent_eleve_grille_repas_act_idx
    ON ent_eleve_grille_repas (etablissement_id, activite_id);

CREATE TABLE IF NOT EXISTS ent_bourse_type (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    actif BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_bourse_type_code_uq UNIQUE (etablissement_id, code)
);

CREATE INDEX IF NOT EXISTS ent_bourse_type_etab_idx
    ON ent_bourse_type (etablissement_id);

CREATE TABLE IF NOT EXISTS ent_eleve_bourse (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    bourse_type_id UUID NOT NULL REFERENCES ent_bourse_type (id) ON DELETE RESTRICT,
    echelon TEXT NOT NULL,
    avec_procuration BOOLEAN NOT NULL DEFAULT false,
    status TEXT NOT NULL DEFAULT 'demandee',
    date_debut DATE NOT NULL,
    date_fin DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_bourse_status_chk
        CHECK (status IN ('demandee', 'acceptee', 'refusee', 'cloturee')),
    CONSTRAINT ent_eleve_bourse_fin_chk
        CHECK (date_fin IS NULL OR date_fin >= date_debut)
);

CREATE INDEX IF NOT EXISTS ent_eleve_bourse_eleve_idx
    ON ent_eleve_bourse (etablissement_id, eleve_id);

CREATE INDEX IF NOT EXISTS ent_eleve_bourse_status_idx
    ON ent_eleve_bourse (etablissement_id, status);
