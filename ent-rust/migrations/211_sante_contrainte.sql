-- 211 — Contraintes publiées sans donnée médicale (SAN-06). Additif et idempotent.
--
-- Principe de minimisation : cette table ne contient JAMAIS de texte libre ni de
-- diagnostic. Une contrainte est un CODE d'un catalogue fermé (Rust,
-- `sante_acc::CATALOGUE_CONTRAINTES` : « repas_sans_arachide », « ne_pas_courir »,
-- « trousse_urgence »…) et des audiences (restauration, enseignants, sorties).
-- Le lien avec le PAI (`source_id`) n'est lisible que du personnel de santé.

CREATE TABLE IF NOT EXISTS ent_contrainte_publiee (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    code TEXT NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{2,39}$'),
    audiences TEXT[] NOT NULL
        CHECK (cardinality(audiences) BETWEEN 1 AND 3
               AND audiences <@ ARRAY['restauration', 'enseignants', 'sorties']::text[]),
    source TEXT NOT NULL CHECK (source IN ('pai', 'pap', 'pps', 'manuel')),
    source_id UUID,
    date_debut DATE NOT NULL,
    date_fin DATE,
    actif BOOLEAN NOT NULL DEFAULT true,
    publie_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    retire_at TIMESTAMPTZ,
    CONSTRAINT ent_contrainte_publiee_dates_chk CHECK (date_fin IS NULL OR date_fin >= date_debut)
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_contrainte_publiee_active_uq
    ON ent_contrainte_publiee (etablissement_id, eleve_id, code)
    WHERE actif;

CREATE INDEX IF NOT EXISTS ent_contrainte_publiee_etab_idx
    ON ent_contrainte_publiee (etablissement_id, actif);
