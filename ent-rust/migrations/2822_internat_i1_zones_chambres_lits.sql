-- Socle 41 (internat), lot I1 — migration 2822 : zones, chambres étendues, lits.
-- Réf. : docs/socles/41_INTERNAT.md §8.1, §24 ; RI-07, DI-15, DI-23 ; R-IT11, R-IT32.
-- Additive et rejouable (E8). Les colonnes de 066 (`batiment` texte, `genre`, `actif`) sont CONSERVÉES
-- jusqu'au lot I14. Aucune colonne JSON (D-036), aucun BYTEA (E5).

-- Cible de clé composite vers le bâtiment du socle 5 (E2).
CREATE UNIQUE INDEX IF NOT EXISTS ent_batiment_etab_id_uq ON ent_batiment (etablissement_id, id);

-- ───────────────────────── Zones (étage, aile) d'un bâtiment ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_internat_zone (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    batiment_id UUID NOT NULL,
    code TEXT NOT NULL CHECK (length(btrim(code)) BETWEEN 1 AND 40),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    public TEXT NOT NULL CHECK (public IN ('filles', 'garcons', 'commun')),
    ordre SMALLINT NOT NULL DEFAULT 0,
    ferme_du DATE,
    ferme_au DATE,
    cree_par_user_id UUID REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_zone_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_zone_code_uq UNIQUE (etablissement_id, batiment_id, code),
    CONSTRAINT ent_internat_zone_batiment_cfk FOREIGN KEY (etablissement_id, batiment_id)
        REFERENCES ent_batiment (etablissement_id, id),
    CONSTRAINT ent_internat_zone_fermeture_chk CHECK (
        (ferme_du IS NULL AND ferme_au IS NULL)
        OR (ferme_du IS NOT NULL AND (ferme_au IS NULL OR ferme_au >= ferme_du)))
);
CREATE INDEX IF NOT EXISTS ent_internat_zone_batiment_idx
    ON ent_internat_zone (etablissement_id, batiment_id, ordre);

-- ───────────────────────── Chambres étendues ─────────────────────────
ALTER TABLE ent_internat_chambre ADD COLUMN IF NOT EXISTS zone_id UUID;
ALTER TABLE ent_internat_chambre ADD COLUMN IF NOT EXISTS public TEXT;
ALTER TABLE ent_internat_chambre ADD COLUMN IF NOT EXISTS accessible_pmr BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_internat_chambre ADD COLUMN IF NOT EXISTS douche BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_internat_chambre ADD COLUMN IF NOT EXISTS lavabo BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_internat_chambre ADD COLUMN IF NOT EXISTS bureau BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_internat_chambre ADD COLUMN IF NOT EXISTS etat TEXT NOT NULL DEFAULT 'active';
ALTER TABLE ent_internat_chambre ADD COLUMN IF NOT EXISTS ferme_du DATE;
ALTER TABLE ent_internat_chambre ADD COLUMN IF NOT EXISTS ferme_au DATE;
-- Notes d'entretien : jamais de nom d'élève (R-IT32) ; texte court.
ALTER TABLE ent_internat_chambre ADD COLUMN IF NOT EXISTS notes_entretien TEXT;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_chambre_public_chk') THEN
        -- Aucune chambre mixte (R-IT11). Les anciennes chambres « mixte » restent sans public
        -- (NULL) : aucune affectation tant que la direction n'a pas choisi filles ou garçons.
        ALTER TABLE ent_internat_chambre
            ADD CONSTRAINT ent_internat_chambre_public_chk CHECK (public IS NULL OR public IN ('filles', 'garcons'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_chambre_etat_chk') THEN
        ALTER TABLE ent_internat_chambre
            ADD CONSTRAINT ent_internat_chambre_etat_chk CHECK (etat IN ('active', 'fermee', 'retiree'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_chambre_fermeture_chk') THEN
        ALTER TABLE ent_internat_chambre
            ADD CONSTRAINT ent_internat_chambre_fermeture_chk CHECK (
                ferme_au IS NULL OR (ferme_du IS NOT NULL AND ferme_au >= ferme_du));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_chambre_notes_chk') THEN
        ALTER TABLE ent_internat_chambre
            ADD CONSTRAINT ent_internat_chambre_notes_chk CHECK (
                notes_entretien IS NULL OR char_length(notes_entretien) <= 300);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_chambre_zone_cfk') THEN
        ALTER TABLE ent_internat_chambre
            ADD CONSTRAINT ent_internat_chambre_zone_cfk
            FOREIGN KEY (etablissement_id, zone_id) REFERENCES ent_internat_zone (etablissement_id, id);
    END IF;
END
$$;

-- Reprise des colonnes de 066 : public depuis `genre` (sauf « mixte »), état depuis `actif`.
UPDATE ent_internat_chambre SET public = genre
 WHERE public IS NULL AND genre IN ('filles', 'garcons');
UPDATE ent_internat_chambre SET etat = 'retiree'
 WHERE actif = false AND etat = 'active';

-- ───────────────────────── Lits (unité d'affectation) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_internat_lit (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    chambre_id UUID NOT NULL,
    numero SMALLINT NOT NULL CHECK (numero BETWEEN 1 AND 64),
    neutralise_du DATE,
    neutralise_au DATE,
    motif_code TEXT CHECK (motif_code IS NULL OR motif_code IN ('casse', 'accueil_urgence', 'travaux', 'autre')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_lit_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_lit_numero_uq UNIQUE (etablissement_id, chambre_id, numero),
    CONSTRAINT ent_internat_lit_chambre_cfk FOREIGN KEY (etablissement_id, chambre_id)
        REFERENCES ent_internat_chambre (etablissement_id, id),
    CONSTRAINT ent_internat_lit_neutralisation_chk CHECK (
        neutralise_au IS NULL OR (neutralise_du IS NOT NULL AND neutralise_au >= neutralise_du))
);

-- Un lit par place déclarée dans `capacite` (identifiants posés ici, à la reprise ; le code applicatif
-- génère ensuite ses propres UUID, E3). Rejouable : les numéros déjà présents ne sont pas recréés.
INSERT INTO ent_internat_lit (id, etablissement_id, chambre_id, numero)
SELECT gen_random_uuid(), c.etablissement_id, c.id, n.numero
  FROM ent_internat_chambre c
  CROSS JOIN LATERAL generate_series(1, c.capacite) AS n(numero)
ON CONFLICT (etablissement_id, chambre_id, numero) DO NOTHING;

SELECT ent_rls_poser(ARRAY['ent_internat_zone', 'ent_internat_lit']);
