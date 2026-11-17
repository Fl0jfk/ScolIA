-- Socle 28, lot R2 (MS4 c) — migration 2565 : table de correspondance des identifiants.
-- Réf. : docs/socles/28_MISE_EN_SERVICE.md §15 (rep_correspondance), T0 §8.4.
--
-- Un identifiant du Next.js (source, objet source, identifiant source) donne UN identifiant ScolIA, de
-- façon stable et rejouable : unique par établissement. `cible_id` est l'identifiant dérivé par l'outil
-- (SHA-256, jamais aléatoire) tant qu'aucune décision humaine ne l'a remplacé. `empreinte_source` permet
-- de savoir si la source a changé depuis la dernière reprise ; `modifie_depuis_reprise` interdit tout
-- écrasement. Aucune clé étrangère vers le métier : la cible dépend de `cible_objet` (domaine propriétaire).
-- Pas de fusion ni de suppression automatique. Migration additive et rejouable ; RLS d'établissement.

CREATE TABLE IF NOT EXISTS rep_correspondance (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    source TEXT NOT NULL CHECK (source ~ '^[a-z][a-z_]{0,29}$'),
    source_objet TEXT NOT NULL CHECK (length(source_objet) BETWEEN 1 AND 120),
    source_id TEXT NOT NULL CHECK (length(source_id) BETWEEN 1 AND 400),
    cible_objet TEXT NOT NULL CHECK (length(cible_objet) BETWEEN 1 AND 60),
    cible_id UUID NOT NULL,
    regle_code TEXT NOT NULL CHECK (regle_code ~ '^[a-z][a-z0-9_]{1,59}$'),
    statut TEXT NOT NULL CHECK (statut IN ('rapprochee', 'a_decider', 'ignoree', 'creee')),
    -- Dernière exécution qui a posé ou confirmé la correspondance (information, sans clé étrangère :
    -- l'historique des exécutions peut être purgé sans toucher aux correspondances).
    execution_id UUID,
    empreinte_source TEXT CHECK (empreinte_source IS NULL OR length(empreinte_source) = 64),
    modifie_depuis_reprise BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rep_correspondance_source_uq UNIQUE (etablissement_id, source, source_objet, source_id)
);
CREATE INDEX IF NOT EXISTS rep_correspondance_cible_idx ON rep_correspondance (etablissement_id, cible_objet, cible_id);

SELECT ent_rls_poser(ARRAY['rep_correspondance']);
