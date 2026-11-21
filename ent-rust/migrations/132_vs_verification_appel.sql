-- Socle 2, lot S2-4 : vérification de l'appel (VS-04, A2). Les appels non faits
-- et les incohérences sont calculés à la demande ; seule la relance est tracée.

CREATE TABLE IF NOT EXISTS ent_appel_relance (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    creneau_id UUID NOT NULL,
    jour DATE NOT NULL,
    destinataire_user_id UUID NOT NULL,
    envoyee_par_user_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS ent_appel_relance_seance_idx
    ON ent_appel_relance (etablissement_id, creneau_id, jour);
