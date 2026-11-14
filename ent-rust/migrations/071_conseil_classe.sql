-- Conseil de classe : session + dossiers élèves + mentions.

CREATE TABLE IF NOT EXISTS ent_conseil_classe (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    classe TEXT NOT NULL,
    periode_libelle TEXT NOT NULL,
    jour DATE NOT NULL,
    heure TIME NOT NULL,
    lieu TEXT NOT NULL DEFAULT '',
    statut TEXT NOT NULL DEFAULT 'planifie',
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_conseil_classe_statut_chk
        CHECK (statut IN ('planifie', 'tenu', 'annule'))
);

CREATE INDEX IF NOT EXISTS ent_conseil_classe_etab_idx
    ON ent_conseil_classe (etablissement_id, jour DESC);

CREATE TABLE IF NOT EXISTS ent_conseil_classe_dossier (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    conseil_id UUID NOT NULL REFERENCES ent_conseil_classe (id),
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    appreciation TEXT NOT NULL DEFAULT '',
    mention TEXT,
    decision_texte TEXT NOT NULL DEFAULT '',
    visible_famille BOOLEAN NOT NULL DEFAULT false,
    decided_at TIMESTAMPTZ,
    decided_by_user_id UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_conseil_dossier_eleve_uq UNIQUE (conseil_id, eleve_id),
    CONSTRAINT ent_conseil_dossier_mention_chk
        CHECK (
            mention IS NULL OR mention IN (
                'felicitations', 'complement', 'encouragements',
                'avertissement_travail', 'avertissement_conduite', 'aucune'
            )
        )
);

CREATE INDEX IF NOT EXISTS ent_conseil_dossier_eleve_idx
    ON ent_conseil_classe_dossier (etablissement_id, eleve_id);
