-- Socle 2, lot S2-1 (DOS-08) : contacts d'urgence et personnes autorisées à
-- récupérer l'élève. Un contact est une PERSONNE (D-020), avec ou sans compte,
-- sans lien de responsabilité ni droit sur le dossier.

CREATE TABLE IF NOT EXISTS ent_eleve_contact_urgence (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id) ON DELETE CASCADE,
    personne_id UUID NOT NULL REFERENCES ent_personne (id),
    -- Relation à l'élève (grand-parent, voisin, assistante maternelle…).
    relation TEXT NOT NULL DEFAULT 'autre'
        CHECK (length(btrim(relation)) BETWEEN 1 AND 60),
    ordre SMALLINT NOT NULL DEFAULT 1 CHECK (ordre BETWEEN 1 AND 20),
    peut_recuperer BOOLEAN NOT NULL DEFAULT false,
    -- `propose` : saisi par un responsable, à valider par l'établissement.
    statut TEXT NOT NULL DEFAULT 'valide' CHECK (statut IN ('valide', 'propose')),
    commentaire TEXT CHECK (commentaire IS NULL OR length(commentaire) <= 500),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_contact_urgence_uq UNIQUE (etablissement_id, eleve_id, personne_id)
);

CREATE INDEX IF NOT EXISTS ent_eleve_contact_urgence_eleve_idx
    ON ent_eleve_contact_urgence (etablissement_id, eleve_id, ordre);
CREATE INDEX IF NOT EXISTS ent_eleve_contact_urgence_personne_idx
    ON ent_eleve_contact_urgence (etablissement_id, personne_id);
