-- Pièces jointes messagerie (P0) : contenu borné en base locale.

CREATE TABLE IF NOT EXISTS ent_msg_piece (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    fil_id UUID NOT NULL REFERENCES ent_msg_fil (id),
    message_id UUID NOT NULL REFERENCES ent_msg (id),
    nom TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    taille_octets INTEGER NOT NULL,
    contenu BYTEA NOT NULL,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_msg_piece_taille_chk CHECK (taille_octets > 0 AND taille_octets <= 262144)
);

CREATE INDEX IF NOT EXISTS ent_msg_piece_msg_idx
    ON ent_msg_piece (etablissement_id, message_id);

CREATE INDEX IF NOT EXISTS ent_msg_piece_fil_idx
    ON ent_msg_piece (etablissement_id, fil_id);
