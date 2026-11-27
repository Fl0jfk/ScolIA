-- S2-5 (EDT-01→08) : administration de l'emploi du temps.
--   * un créneau peut viser un groupe (option, atelier, demi-groupe) au lieu de la classe entière ;
--   * un déplacement de cours est tracé (le cours d'origine est annulé, la séance déplacée est
--     une séance datée de source « deplacement », jamais écrasée par la régénération).
-- Migration additive : aucune donnée existante n'est modifiée.

ALTER TABLE ent_edt_creneau
    ADD COLUMN IF NOT EXISTS groupe_id UUID REFERENCES ent_groupe (id);

CREATE INDEX IF NOT EXISTS ent_edt_creneau_groupe_idx
    ON ent_edt_creneau (etablissement_id, groupe_id)
    WHERE groupe_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS ent_edt_deplacement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    creneau_id UUID NOT NULL REFERENCES ent_edt_creneau (id),
    jour_origine DATE NOT NULL,
    nouveau_jour DATE NOT NULL,
    nouvelle_heure_debut TIME NOT NULL,
    nouvelle_heure_fin TIME NOT NULL,
    nouvelle_salle TEXT,
    cree_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_edt_deplacement_uq UNIQUE (etablissement_id, creneau_id, jour_origine),
    CONSTRAINT ent_edt_deplacement_heures_chk CHECK (nouvelle_heure_fin > nouvelle_heure_debut)
);

CREATE INDEX IF NOT EXISTS ent_edt_deplacement_jour_idx
    ON ent_edt_deplacement (etablissement_id, nouveau_jour);
