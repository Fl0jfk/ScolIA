-- Annonces de l'établissement (communication direction → familles / personnels).
-- Diffusion : « tous » / « parents » / « personnels » / « classe » (ciblée).
CREATE TABLE IF NOT EXISTS ent_annonce (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    titre TEXT NOT NULL,
    corps TEXT NOT NULL,
    audience TEXT NOT NULL,
    cible_classe TEXT,
    auteur_user_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_annonce_audience_chk
        CHECK (audience IN ('tous', 'parents', 'personnels', 'classe')),
    CONSTRAINT ent_annonce_classe_chk
        CHECK ((audience = 'classe') = (cible_classe IS NOT NULL))
);

CREATE INDEX IF NOT EXISTS ent_annonce_etab_created_idx
    ON ent_annonce (etablissement_id, created_at DESC);
