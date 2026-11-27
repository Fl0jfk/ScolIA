-- S2-5 (EDT-05, EDT-08, RH-04) : remplacements et annulations en masse.
--   * `ent_edt_annulation_masse` : un lot (grève, fermeture, sortie de niveau) ; la clé
--     d'idempotence du client empêche un double envoi de créer deux lots ;
--   * `ent_edt_exception` : rattachement facultatif au lot, à l'absence de personnel
--     d'origine et à l'organisateur (traçabilité ; le motif libre n'est jamais diffusé).
-- Migration additive.

CREATE TABLE IF NOT EXISTS ent_edt_annulation_masse (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    cle TEXT,
    du DATE NOT NULL,
    au DATE NOT NULL,
    portee TEXT NOT NULL,
    cibles UUID[] NOT NULL DEFAULT '{}',
    heure_debut TIME,
    heure_fin TIME,
    motif_public TEXT NOT NULL,
    nb_seances INTEGER NOT NULL DEFAULT 0,
    nb_classes INTEGER NOT NULL DEFAULT 0,
    cree_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_edt_annulation_masse_portee_chk CHECK (portee IN ('etablissement', 'niveau', 'classes')),
    CONSTRAINT ent_edt_annulation_masse_dates_chk CHECK (au >= du),
    CONSTRAINT ent_edt_annulation_masse_motif_chk CHECK (length(btrim(motif_public)) BETWEEN 1 AND 200)
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_annulation_masse_cle_uq
    ON ent_edt_annulation_masse (etablissement_id, cle)
    WHERE cle IS NOT NULL;

ALTER TABLE ent_edt_exception
    ADD COLUMN IF NOT EXISTS lot_id UUID REFERENCES ent_edt_annulation_masse (id),
    ADD COLUMN IF NOT EXISTS absence_rh_id UUID REFERENCES ent_absence_rh (id),
    ADD COLUMN IF NOT EXISTS nature TEXT;

CREATE INDEX IF NOT EXISTS ent_edt_exception_lot_idx
    ON ent_edt_exception (etablissement_id, lot_id)
    WHERE lot_id IS NOT NULL;
