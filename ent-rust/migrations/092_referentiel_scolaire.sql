-- Référentiel scolaire (fondation) : année, classe, groupe, affectation, service
-- d'enseignement. ADDITIF : aucune colonne existante n'est modifiée ; les champs TEXT
-- `classe` de ent_eleve / ent_edt_creneau / ent_devoir restent la compatibilité.
-- Toutes les tables portent etablissement_id (cloisonnement).

CREATE TABLE IF NOT EXISTS ent_annee_scolaire (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    libelle TEXT NOT NULL,
    date_debut DATE NOT NULL,
    date_fin DATE NOT NULL,
    courante BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_annee_scolaire_libelle_uq UNIQUE (etablissement_id, libelle),
    CONSTRAINT ent_annee_scolaire_dates_chk CHECK (date_fin > date_debut)
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_annee_scolaire_courante_uq
    ON ent_annee_scolaire (etablissement_id) WHERE courante;

-- Le niveau (6e, 4e, 2nde…) est un simple champ de la classe : pas d'entité à part
-- (la nomenclature varie selon l'établissement, aucune règle ne s'y attache encore).
CREATE TABLE IF NOT EXISTS ent_classe (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    annee_id UUID NOT NULL REFERENCES ent_annee_scolaire (id),
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    niveau TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_classe_code_uq UNIQUE (etablissement_id, annee_id, code),
    CONSTRAINT ent_classe_code_chk CHECK (length(btrim(code)) > 0)
);

CREATE INDEX IF NOT EXISTS ent_classe_etab_annee_idx
    ON ent_classe (etablissement_id, annee_id);

-- Demi-groupe (classe_id renseigné) ou groupe transverse (classe_id nul).
CREATE TABLE IF NOT EXISTS ent_groupe (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    annee_id UUID NOT NULL REFERENCES ent_annee_scolaire (id),
    code TEXT NOT NULL,
    libelle TEXT NOT NULL,
    classe_id UUID REFERENCES ent_classe (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_groupe_code_uq UNIQUE (etablissement_id, annee_id, code),
    CONSTRAINT ent_groupe_code_chk CHECK (length(btrim(code)) > 0)
);

CREATE INDEX IF NOT EXISTS ent_groupe_etab_annee_idx
    ON ent_groupe (etablissement_id, annee_id);

-- Appartenance d'un élève à une classe OU à un groupe, sur une période
-- (date_fin incluse ; nulle = en cours).
CREATE TABLE IF NOT EXISTS ent_affectation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    annee_id UUID NOT NULL REFERENCES ent_annee_scolaire (id),
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    classe_id UUID REFERENCES ent_classe (id),
    groupe_id UUID REFERENCES ent_groupe (id),
    date_debut DATE NOT NULL,
    date_fin DATE,
    source TEXT NOT NULL DEFAULT 'saisie',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_affectation_cible_chk CHECK ((classe_id IS NULL) <> (groupe_id IS NULL)),
    CONSTRAINT ent_affectation_dates_chk CHECK (date_fin IS NULL OR date_fin >= date_debut),
    CONSTRAINT ent_affectation_source_chk CHECK (source IN ('saisie', 'amorcage', 'sync_classe'))
);

CREATE INDEX IF NOT EXISTS ent_affectation_classe_idx
    ON ent_affectation (etablissement_id, classe_id) WHERE classe_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS ent_affectation_groupe_idx
    ON ent_affectation (etablissement_id, groupe_id) WHERE groupe_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS ent_affectation_eleve_idx
    ON ent_affectation (etablissement_id, eleve_id);

-- Une seule classe « en cours » par élève et par année.
CREATE UNIQUE INDEX IF NOT EXISTS ent_affectation_classe_ouverte_uq
    ON ent_affectation (etablissement_id, eleve_id, annee_id)
    WHERE classe_id IS NOT NULL AND date_fin IS NULL;

-- Qui enseigne quoi à qui : professeur x (matière) x (classe OU groupe), par année.
CREATE TABLE IF NOT EXISTS ent_service_enseignement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    annee_id UUID NOT NULL REFERENCES ent_annee_scolaire (id),
    prof_user_id UUID NOT NULL REFERENCES ent_user (id),
    matiere_id UUID REFERENCES ent_matiere (id),
    -- Libellé d'affichage quand aucune matière du référentiel n'est liée.
    matiere_libelle TEXT NOT NULL DEFAULT '',
    classe_id UUID REFERENCES ent_classe (id),
    groupe_id UUID REFERENCES ent_groupe (id),
    professeur_principal BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_service_cible_chk CHECK ((classe_id IS NULL) <> (groupe_id IS NULL)),
    CONSTRAINT ent_service_pp_chk CHECK (NOT professeur_principal OR classe_id IS NOT NULL)
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_service_unique_idx
    ON ent_service_enseignement (
        etablissement_id, annee_id, prof_user_id,
        coalesce(classe_id, groupe_id),
        coalesce(matiere_id, '00000000-0000-0000-0000-000000000000'::uuid),
        matiere_libelle
    );

-- Un seul professeur principal par classe.
CREATE UNIQUE INDEX IF NOT EXISTS ent_service_pp_uq
    ON ent_service_enseignement (classe_id) WHERE professeur_principal;

CREATE INDEX IF NOT EXISTS ent_service_prof_idx
    ON ent_service_enseignement (etablissement_id, prof_user_id, annee_id);
