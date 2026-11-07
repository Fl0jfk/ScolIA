-- Internat : chambres, affectations élèves, présence soir, suivis VS.

CREATE TABLE IF NOT EXISTS ent_internat_chambre (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    batiment TEXT NOT NULL,
    etage SMALLINT NOT NULL DEFAULT 0,
    code TEXT NOT NULL,
    capacite SMALLINT NOT NULL,
    genre TEXT NOT NULL DEFAULT 'mixte',
    actif BOOLEAN NOT NULL DEFAULT true,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_chambre_code_uq UNIQUE (etablissement_id, code),
    CONSTRAINT ent_internat_chambre_capacite_chk CHECK (capacite BETWEEN 1 AND 64),
    CONSTRAINT ent_internat_chambre_genre_chk CHECK (genre IN ('mixte', 'filles', 'garcons')),
    CONSTRAINT ent_internat_chambre_code_chk CHECK (char_length(trim(code)) >= 1)
);

CREATE INDEX IF NOT EXISTS ent_internat_chambre_etab_idx
    ON ent_internat_chambre (etablissement_id, actif);

CREATE TABLE IF NOT EXISTS ent_internat_affectation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    chambre_id UUID NOT NULL REFERENCES ent_internat_chambre (id),
    lit TEXT,
    debut DATE NOT NULL,
    fin DATE,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_affectation_fin_chk
        CHECK (fin IS NULL OR fin >= debut)
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_internat_affectation_active_uq
    ON ent_internat_affectation (etablissement_id, eleve_id)
    WHERE fin IS NULL;

CREATE INDEX IF NOT EXISTS ent_internat_affectation_chambre_idx
    ON ent_internat_affectation (etablissement_id, chambre_id, debut);

CREATE INDEX IF NOT EXISTS ent_internat_affectation_eleve_idx
    ON ent_internat_affectation (etablissement_id, eleve_id);

CREATE TABLE IF NOT EXISTS ent_internat_presence (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    affectation_id UUID NOT NULL REFERENCES ent_internat_affectation (id),
    jour DATE NOT NULL,
    statut TEXT NOT NULL,
    note TEXT NOT NULL DEFAULT '',
    actor_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_presence_statut_chk
        CHECK (statut IN ('present', 'absent', 'permis_sortie', 'retard')),
    CONSTRAINT ent_internat_presence_jour_uq UNIQUE (etablissement_id, eleve_id, jour)
);

CREATE INDEX IF NOT EXISTS ent_internat_presence_jour_idx
    ON ent_internat_presence (etablissement_id, jour);

CREATE TABLE IF NOT EXISTS ent_internat_suivi (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    jour DATE NOT NULL,
    texte TEXT NOT NULL,
    visible_famille BOOLEAN NOT NULL DEFAULT false,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_suivi_texte_chk CHECK (char_length(trim(texte)) >= 1)
);

CREATE INDEX IF NOT EXISTS ent_internat_suivi_eleve_idx
    ON ent_internat_suivi (etablissement_id, eleve_id, jour DESC);
