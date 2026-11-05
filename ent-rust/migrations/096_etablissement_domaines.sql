-- Socle 1 (C2, D-012) : l'établissement est déduit de l'adresse, jamais saisi.
--
-- `ent_etablissement` : registre des établissements (jusqu'ici, un établissement
-- « existait » seulement par ses memberships). `id` reprend l'identifiant déjà
-- utilisé partout (`etablissement_id`) ; aucune FK n'est ajoutée sur les tables
-- existantes (additif, compatible).
-- `ent_etablissement_domaine` : nom(s) d'hôte servant cet établissement
-- (ex. `lpnb.scolia.fr`), normalisés en minuscules, sans port ni point final.

CREATE TABLE IF NOT EXISTS ent_etablissement (
    id          UUID PRIMARY KEY,
    slug        TEXT NOT NULL UNIQUE
                CHECK (slug ~ '^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$'),
    nom         TEXT NOT NULL
                CHECK (length(btrim(nom)) BETWEEN 1 AND 200),
    actif       BOOLEAN NOT NULL DEFAULT true,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS ent_etablissement_domaine (
    domaine           TEXT PRIMARY KEY
                      CHECK (domaine = lower(domaine)
                             AND length(domaine) BETWEEN 1 AND 253
                             AND domaine ~ '^[a-z0-9]([a-z0-9.-]*[a-z0-9])?$'
                             AND domaine !~ '\.\.'),
    etablissement_id  UUID NOT NULL REFERENCES ent_etablissement(id) ON DELETE CASCADE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS ent_etablissement_domaine_etab_idx
    ON ent_etablissement_domaine (etablissement_id);
