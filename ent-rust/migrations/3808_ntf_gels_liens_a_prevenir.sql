-- Socle 8, lot N2 (destinataires = personnes) — migration 3808 : gels, liens nominatifs, tâches « à prévenir autrement ».
-- Réf. : docs/socles/08_NOTIFICATIONS.md §19 (3808), §9.1 (qui), §15 (personnes à protéger), CN-12, CN-13, CN-33.
--
-- Additive et rejouable. Aucune donnée semi-structurée (D-036). Le jeton d'un lien nominatif n'est JAMAIS stocké
-- en clair : seul son empreinte SHA-256 l'est ; il expire, s'ouvre un nombre limité de fois et se révoque.

-- Foyer du destinataire (regroupement de la fratrie et des remises, lot N3) : colonne ajoutée ici, la migration 3801
-- étant déjà livrée.
ALTER TABLE ent_ntf_destinataire ADD COLUMN IF NOT EXISTS foyer_id UUID;

-- Gel : tant qu'un gel est posé sur un sujet, aucun avis automatique ne le concerne (décès de l'élève, CN-13).
CREATE TABLE IF NOT EXISTS ent_ntf_gel (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sujet_type TEXT NOT NULL CHECK (sujet_type IN ('eleve', 'personne')),
    sujet_id UUID NOT NULL,
    motif_code TEXT NOT NULL CHECK (motif_code ~ '^[a-z][a-z0-9_]{1,59}$'),
    pose_par UUID,
    pose_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    leve_par UUID,
    leve_le TIMESTAMPTZ,
    CONSTRAINT ent_ntf_gel_leve_chk CHECK ((leve_le IS NULL) = (leve_par IS NULL))
);
-- Un seul gel actif par sujet et par motif (le décès se rejoue sans doublon).
CREATE UNIQUE INDEX IF NOT EXISTS ent_ntf_gel_actif_uq
    ON ent_ntf_gel (etablissement_id, sujet_type, sujet_id, motif_code) WHERE leve_le IS NULL;

CREATE TABLE IF NOT EXISTS ent_ntf_lien (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    destinataire_id UUID NOT NULL,
    jeton_hash TEXT NOT NULL CHECK (jeton_hash ~ '^[0-9a-f]{64}$'),
    code_requis BOOLEAN NOT NULL DEFAULT false,
    expire_le TIMESTAMPTZ NOT NULL,
    ouvertures_max SMALLINT NOT NULL DEFAULT 3 CHECK (ouvertures_max BETWEEN 1 AND 20),
    ouvertures SMALLINT NOT NULL DEFAULT 0 CHECK (ouvertures >= 0),
    revoque_le TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_ntf_lien_hash_uq UNIQUE (jeton_hash),
    CONSTRAINT ent_ntf_lien_dest_cfk FOREIGN KEY (etablissement_id, destinataire_id)
        REFERENCES ent_ntf_destinataire (etablissement_id, id) ON DELETE CASCADE
);

-- Personne qu'aucun canal n'atteint alors que la nature l'exige : tâche manuelle (« à prévenir autrement »).
CREATE TABLE IF NOT EXISTS ent_ntf_a_prevenir (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    destinataire_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    echeance TIMESTAMPTZ NOT NULL,
    statut TEXT NOT NULL DEFAULT 'a_faire' CHECK (statut IN ('a_faire', 'fait', 'abandonne')),
    resultat TEXT CHECK (resultat IS NULL OR resultat ~ '^[a-z][a-z0-9_]{1,59}$'),
    par UUID,
    le TIMESTAMPTZ,
    CONSTRAINT ent_ntf_a_prevenir_dest_uq UNIQUE (destinataire_id),
    CONSTRAINT ent_ntf_a_prevenir_dest_cfk FOREIGN KEY (etablissement_id, destinataire_id)
        REFERENCES ent_ntf_destinataire (etablissement_id, id) ON DELETE CASCADE,
    CONSTRAINT ent_ntf_a_prevenir_clos_chk CHECK ((statut = 'a_faire') = (le IS NULL))
);
CREATE INDEX IF NOT EXISTS ent_ntf_a_prevenir_ouvert_idx
    ON ent_ntf_a_prevenir (etablissement_id, echeance) WHERE statut = 'a_faire';

SELECT ent_rls_poser(ARRAY[
    'ent_ntf_gel',
    'ent_ntf_lien',
    'ent_ntf_a_prevenir'
]);
