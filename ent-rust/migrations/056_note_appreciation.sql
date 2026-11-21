-- Appréciations bulletin (matière ou générale) rattachées à une période.

CREATE TABLE IF NOT EXISTS ent_note_appreciation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    periode_id UUID NOT NULL REFERENCES ent_note_periode (id),
    eleve_id UUID NOT NULL,
    matiere_id UUID REFERENCES ent_matiere (id),
    texte TEXT NOT NULL,
    auteur_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_note_appreciation_texte_chk CHECK (char_length(trim(texte)) > 0)
);

-- Une appréciation matière par (période, élève, matière) ; une générale si matiere_id IS NULL.
CREATE UNIQUE INDEX IF NOT EXISTS ent_note_appreciation_matiere_uq
    ON ent_note_appreciation (etablissement_id, periode_id, eleve_id, matiere_id)
    WHERE matiere_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS ent_note_appreciation_generale_uq
    ON ent_note_appreciation (etablissement_id, periode_id, eleve_id)
    WHERE matiere_id IS NULL;

CREATE INDEX IF NOT EXISTS ent_note_appreciation_eleve_idx
    ON ent_note_appreciation (etablissement_id, eleve_id, periode_id);
