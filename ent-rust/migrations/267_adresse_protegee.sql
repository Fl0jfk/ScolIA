-- Socle 4, lot A0 bis, A0-b (R-A39). Adresse à protéger : elle n'est JAMAIS écrite dans
-- `ent_personne.adresse` ni dans le dossier d'inscription. Elle est « parquée » ici, scellée
-- (AES-256-GCM, table/colonne déclarées dans `champs_chiffres::CHAMPS_CHIFFRES`), lisible par
-- la direction seule, avec une alerte (`statut = 'a_traiter'`) jusqu'au lot A3 qui en fera une
-- adresse protégée à part entière. Migration additive et rejouable.

CREATE TABLE IF NOT EXISTS ent_adresse_protegee (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    personne_id UUID NOT NULL,
    adresse TEXT NOT NULL,                      -- scellée : `enc:v1:…`
    origine TEXT NOT NULL CHECK (origine IN ('declaration', 'saisie')),
    source_ref UUID,
    statut TEXT NOT NULL DEFAULT 'a_traiter' CHECK (statut IN ('a_traiter', 'traitee')),
    cree_par UUID,
    traitee_le TIMESTAMPTZ,
    traitee_par UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_adresse_protegee_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ent_adresse_protegee_alerte_idx
    ON ent_adresse_protegee (etablissement_id, statut, created_at);
