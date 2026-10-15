-- Ordonnances / PJ attachées au PAI (contenu borné en base locale).

CREATE TABLE IF NOT EXISTS ent_pai_piece (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    pai_id UUID NOT NULL REFERENCES ent_pai (id),
    nom TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    taille_octets INTEGER NOT NULL,
    contenu BYTEA NOT NULL,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_pai_piece_taille_chk CHECK (taille_octets > 0 AND taille_octets <= 262144)
);

CREATE INDEX IF NOT EXISTS ent_pai_piece_pai_idx
    ON ent_pai_piece (etablissement_id, pai_id);

CREATE INDEX IF NOT EXISTS ent_pai_piece_eleve_idx
    ON ent_pai_piece (etablissement_id, eleve_id);
