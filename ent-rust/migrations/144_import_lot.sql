-- Socle 2, lot S2-3 : moteur d'import avec prévisualisation et rejets ligne à
-- ligne (IOP-01 au format documenté `scolia_eleves_v1`, IOP-04 ; IOP-L1 de
-- `INTEROPERABILITE_NATIONALE.md`). Les formats nationaux (AAF, BEE) restent à
-- brancher sur ce moteur après réception des spécifications officielles.
--
-- Rapprochement : INE puis identité (nom, prénom, date de naissance) ; jamais
-- d'écrasement silencieux (un élève déjà présent n'est pas modifié).
-- Migration additive et rejouable.

CREATE TABLE IF NOT EXISTS ent_import_lot (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    format TEXT NOT NULL CHECK (format IN ('scolia_eleves_v1')),
    nom_fichier TEXT NOT NULL CHECK (length(nom_fichier) BETWEEN 1 AND 200),
    statut TEXT NOT NULL DEFAULT 'previsualise'
        CHECK (statut IN ('previsualise', 'applique', 'annule')),
    cree_par UUID REFERENCES ent_user (id) ON DELETE SET NULL,
    applique_par UUID REFERENCES ent_user (id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    applique_le TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS ent_import_lot_etab_idx
    ON ent_import_lot (etablissement_id, created_at DESC);

CREATE TABLE IF NOT EXISTS ent_import_ligne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    lot_id UUID NOT NULL REFERENCES ent_import_lot (id) ON DELETE CASCADE,
    numero INTEGER NOT NULL CHECK (numero >= 1),
    donnees JSONB NOT NULL,
    statut TEXT NOT NULL CHECK (statut IN ('a_creer', 'existant', 'rejet', 'creee', 'ignoree')),
    motifs TEXT[] NOT NULL DEFAULT '{}',
    eleve_id UUID REFERENCES ent_eleve (id),
    traite_par UUID REFERENCES ent_user (id) ON DELETE SET NULL,
    traite_le TIMESTAMPTZ,
    CONSTRAINT ent_import_ligne_uq UNIQUE (lot_id, numero)
);

CREATE INDEX IF NOT EXISTS ent_import_ligne_lot_idx
    ON ent_import_ligne (etablissement_id, lot_id, statut);
