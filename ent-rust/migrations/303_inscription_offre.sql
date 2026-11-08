-- Lot I2 (socle 3, §8.2, §16 M3) — offre et capacités.
--
-- Une offre = niveau × parcours (MEF, option, section, filière) × régime, avec une capacité
-- facultative (NULL = non limitée). Les offres sont propres à une campagne. Le niveau est
-- rapproché du référentiel (`niveau_id`) quand il existe ; sinon `niveau_code` seul est gardé
-- (la reprise range ces offres dans le rapport) : la capacité ne dépend pas du référentiel.
-- R-I15 : une offre de régime interne peut se décomposer en unités d'hébergement.

CREATE TABLE IF NOT EXISTS ent_inscription_offre (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    campagne_id UUID NOT NULL,
    niveau_code TEXT NOT NULL CHECK (char_length(btrim(niveau_code)) BETWEEN 1 AND 40),
    niveau_id UUID REFERENCES ent_niveau (id),
    mef_id UUID REFERENCES ent_mef (id),
    parcours_groupe_id UUID REFERENCES ent_groupe (id),
    -- Code stable du parcours (option, section, LV…) : vide quand l'offre n'a pas de parcours.
    parcours_code TEXT NOT NULL DEFAULT '' CHECK (char_length(parcours_code) <= 40),
    regime TEXT CHECK (regime IS NULL OR regime IN ('externe', 'demi_pension', 'interne')),
    libelle_public TEXT NOT NULL CHECK (char_length(btrim(libelle_public)) BETWEEN 1 AND 160),
    capacite INTEGER CHECK (capacite IS NULL OR capacite >= 0),
    places_reservees_reinscription BOOLEAN NOT NULL DEFAULT true,
    ouverte_candidature BOOLEAN NOT NULL DEFAULT true,
    conditions_acces TEXT CHECK (conditions_acces IS NULL OR char_length(conditions_acces) <= 300),
    ordre SMALLINT NOT NULL DEFAULT 0,
    archivee_le TIMESTAMPTZ,
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_offre_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_inscription_offre_campagne_fk
        FOREIGN KEY (etablissement_id, campagne_id)
        REFERENCES ent_inscription_campagne (etablissement_id, id) ON DELETE CASCADE
);
-- Une offre par (campagne, niveau, parcours, régime) : la clé fonctionnelle est unique.
CREATE UNIQUE INDEX IF NOT EXISTS ent_inscription_offre_cle_uq
    ON ent_inscription_offre (campagne_id, upper(niveau_code), parcours_code, COALESCE(regime, ''))
    WHERE archivee_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_inscription_offre_campagne_idx
    ON ent_inscription_offre (etablissement_id, campagne_id);

-- Traçabilité des changements de capacité (INS-18 : « capacité modifiable en cours (trace) »).
CREATE TABLE IF NOT EXISTS ent_inscription_offre_historique (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    offre_id UUID NOT NULL,
    capacite_avant INTEGER,
    capacite_apres INTEGER,
    motif TEXT CHECK (motif IS NULL OR char_length(motif) <= 300),
    modifie_par UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_offre_historique_fk
        FOREIGN KEY (etablissement_id, offre_id)
        REFERENCES ent_inscription_offre (etablissement_id, id) ON DELETE CASCADE
);

-- R-I15 : unités d'hébergement d'une offre de régime interne (dortoir, chambre adaptée…).
CREATE TABLE IF NOT EXISTS ent_inscription_offre_unite (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    offre_id UUID NOT NULL,
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 120),
    capacite INTEGER NOT NULL CHECK (capacite >= 0),
    conditions_acces TEXT CHECK (conditions_acces IS NULL OR char_length(conditions_acces) <= 300),
    CONSTRAINT ent_inscription_offre_unite_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_inscription_offre_unite_fk
        FOREIGN KEY (etablissement_id, offre_id)
        REFERENCES ent_inscription_offre (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ent_inscription_offre_unite_offre_idx
    ON ent_inscription_offre_unite (etablissement_id, offre_id);
