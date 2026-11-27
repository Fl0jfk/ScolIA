-- 642 — Socle 42, lot V0a : encadrants (personnes du socle 6, jamais de nom libre — VD-19, VD-40).
-- Réf. : 42_SORTIES_VOYAGES.md §10 ; NX-32, NX-79 (« à préciser » : colonne
-- ent_sortie.places_encadrant_a_pourvoir, migration 640).

CREATE TABLE IF NOT EXISTS ent_sortie_encadrant (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    personne_id UUID NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('responsable', 'encadrant', 'non_compte', 'intervenant')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_sortie_encadrant_uq UNIQUE (sortie_id, personne_id),
    CONSTRAINT ent_sortie_encadrant_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_encadrant_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_encadrant_personne_idx
    ON ent_sortie_encadrant (etablissement_id, personne_id);

SELECT ent_rls_poser(ARRAY['ent_sortie_encadrant']);
