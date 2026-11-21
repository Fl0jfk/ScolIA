-- Socle 2, lot S2-4 : déclarations d'absence (accueil, famille) et rapprochement
-- avec le constat fait à l'appel (VS-06, VS-07, VS-08, A1).

CREATE TABLE IF NOT EXISTS ent_absence_declaration (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    source TEXT NOT NULL,
    -- Période annoncée. Heure absente = début / fin de journée.
    debut_jour DATE NOT NULL,
    debut_heure TIME,
    fin_jour DATE NOT NULL,
    fin_heure TIME,
    motif TEXT NOT NULL,
    commentaire TEXT,
    -- Accueil : identité de la personne qui a appelé / est venue (D-014 : peut ne
    -- pas avoir de compte). Famille : compte du déclarant.
    declarant_nom TEXT,
    declarant_qualite TEXT,
    declarant_user_id UUID,
    -- Pièce (document de l'élève déjà déposé, vérifié à l'écriture).
    piece_document_id UUID,
    statut TEXT NOT NULL DEFAULT 'active',
    annulee_par_user_id UUID,
    annulee_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absence_declaration_source_chk CHECK (source IN ('accueil', 'famille')),
    CONSTRAINT ent_absence_declaration_statut_chk CHECK (statut IN ('active', 'annulee')),
    CONSTRAINT ent_absence_declaration_jours_chk CHECK (fin_jour >= debut_jour),
    CONSTRAINT ent_absence_declaration_heures_chk CHECK (
        fin_jour > debut_jour OR debut_heure IS NULL OR fin_heure IS NULL OR fin_heure > debut_heure
    )
);

CREATE INDEX IF NOT EXISTS ent_absence_declaration_eleve_idx
    ON ent_absence_declaration (etablissement_id, eleve_id, debut_jour, fin_jour);

CREATE INDEX IF NOT EXISTS ent_absence_declaration_jour_idx
    ON ent_absence_declaration (etablissement_id, debut_jour, fin_jour)
    WHERE statut = 'active';

CREATE TABLE IF NOT EXISTS ent_absence_rapprochement (
    etablissement_id UUID NOT NULL,
    declaration_id UUID NOT NULL REFERENCES ent_absence_declaration (id) ON DELETE CASCADE,
    absence_seance_id UUID NOT NULL REFERENCES ent_absence_seance (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (declaration_id, absence_seance_id)
);

CREATE INDEX IF NOT EXISTS ent_absence_rapprochement_seance_idx
    ON ent_absence_rapprochement (absence_seance_id);
