-- Socle 2, lot S2-8 : rendez-vous et rencontres parents-professeurs (D21,
-- AGD-02, AGD-03). Une rencontre propose des créneaux par enseignant ; une
-- famille réserve un créneau POUR UN de ses enfants suivi par cet enseignant.

CREATE TABLE IF NOT EXISTS ent_rencontre (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    titre TEXT NOT NULL,
    classe_id UUID NOT NULL REFERENCES ent_classe (id),
    statut TEXT NOT NULL DEFAULT 'brouillon',
    cree_par UUID NOT NULL,
    ouverte_le TIMESTAMPTZ,
    close_le TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_rencontre_statut_chk CHECK (statut IN ('brouillon', 'ouverte', 'close')),
    CONSTRAINT ent_rencontre_titre_chk CHECK (length(btrim(titre)) > 0 AND length(titre) <= 200)
);

CREATE INDEX IF NOT EXISTS ent_rencontre_etab_idx
    ON ent_rencontre (etablissement_id, created_at DESC);

CREATE TABLE IF NOT EXISTS ent_creneau_rdv (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    rencontre_id UUID NOT NULL REFERENCES ent_rencontre (id) ON DELETE CASCADE,
    enseignant_user_id UUID NOT NULL REFERENCES ent_user (id),
    debut TIMESTAMPTZ NOT NULL,
    fin TIMESTAMPTZ NOT NULL,
    statut TEXT NOT NULL DEFAULT 'libre',
    reserve_par_user_id UUID,
    eleve_id UUID,
    reserve_le TIMESTAMPTZ,
    rappel_envoye_le TIMESTAMPTZ,
    CONSTRAINT ent_creneau_rdv_statut_chk CHECK (statut IN ('libre', 'reserve')),
    CONSTRAINT ent_creneau_rdv_duree_chk CHECK (fin > debut),
    CONSTRAINT ent_creneau_rdv_reserve_chk CHECK (
        (statut = 'libre' AND reserve_par_user_id IS NULL AND eleve_id IS NULL)
        OR (statut = 'reserve' AND reserve_par_user_id IS NOT NULL AND eleve_id IS NOT NULL)
    ),
    CONSTRAINT ent_creneau_rdv_unique UNIQUE (rencontre_id, enseignant_user_id, debut)
);

-- Un seul rendez-vous par enseignant et par enfant dans une rencontre.
CREATE UNIQUE INDEX IF NOT EXISTS ent_creneau_rdv_un_par_eleve_idx
    ON ent_creneau_rdv (rencontre_id, enseignant_user_id, eleve_id) WHERE statut = 'reserve';
CREATE INDEX IF NOT EXISTS ent_creneau_rdv_etab_idx
    ON ent_creneau_rdv (etablissement_id, rencontre_id, debut);
CREATE INDEX IF NOT EXISTS ent_creneau_rdv_rappel_idx
    ON ent_creneau_rdv (debut) WHERE statut = 'reserve' AND rappel_envoye_le IS NULL;
