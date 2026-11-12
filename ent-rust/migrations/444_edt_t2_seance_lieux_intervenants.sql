-- 444 — Socle 10 (emploi du temps), lot T2 : lieux, intervenants figés et fusions d'une séance
-- (docs/socles/10_EMPLOI_DU_TEMPS.md §21.1). Tables destinées au noyau (Q-B15).
--
-- * `ent_edt_seance_lieu` : une séance peut occuper plusieurs lieux (salle + gymnase). Source de
--   l'occupation d'un lieu (D-058, port `occupe`) ; le lieu est un identifiant de `ent_salle`.
-- * `ent_edt_seance_intervenant` : intervenants FIGÉS seulement quand la séance s'écarte du socle 6
--   (remplaçant, intervenant ajouté ou retiré) ; sinon les intervenants sont calculés à la date, jamais
--   copiés (R-E4). Remplie par T3 (modifications) et le socle 15 (remplacements).
-- * `ent_edt_seance_public_fusion` : séance fusionnée (deux cours réunis). Remplie par T3.
-- Additive, rejouable (E8). E1/E2/E3/E6. RLS en 452.

-- Cible de la clé composite (E2) vers l'agent (socle 6).
CREATE UNIQUE INDEX IF NOT EXISTS ent_agent_id_etab_t2_uq ON ent_agent (id, etablissement_id);

CREATE TABLE IF NOT EXISTS ent_edt_seance_lieu (
    seance_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    lieu_id UUID NOT NULL,
    principal BOOLEAN NOT NULL DEFAULT false,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (seance_id, lieu_id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_seance_lieu_principal_uq ON ent_edt_seance_lieu (seance_id) WHERE principal;
CREATE INDEX IF NOT EXISTS ent_edt_seance_lieu_lieu_idx ON ent_edt_seance_lieu (etablissement_id, lieu_id);

CREATE TABLE IF NOT EXISTS ent_edt_seance_intervenant (
    seance_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    agent_id UUID NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN ('remplacant', 'ajoute', 'retire')),
    -- L'intervenant est en saisie (c'est lui qui fait l'appel et remplit le cahier).
    saisie BOOLEAN NOT NULL DEFAULT true,
    -- Absence du socle 15 qui motive la suppléance (lien, jamais le motif).
    origine_ref UUID,
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (seance_id, agent_id, nature)
);
CREATE INDEX IF NOT EXISTS ent_edt_seance_intervenant_agent_idx ON ent_edt_seance_intervenant (etablissement_id, agent_id);

CREATE TABLE IF NOT EXISTS ent_edt_seance_public_fusion (
    seance_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    cours_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (seance_id, cours_id)
);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_seance_lieu_seance_fk') THEN
        ALTER TABLE ent_edt_seance_lieu ADD CONSTRAINT ent_edt_seance_lieu_seance_fk
            FOREIGN KEY (seance_id, etablissement_id) REFERENCES ent_edt_seance (id, etablissement_id) ON DELETE CASCADE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_seance_lieu_lieu_fk') THEN
        ALTER TABLE ent_edt_seance_lieu ADD CONSTRAINT ent_edt_seance_lieu_lieu_fk
            FOREIGN KEY (lieu_id, etablissement_id) REFERENCES ent_salle (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_seance_intervenant_seance_fk') THEN
        ALTER TABLE ent_edt_seance_intervenant ADD CONSTRAINT ent_edt_seance_intervenant_seance_fk
            FOREIGN KEY (seance_id, etablissement_id) REFERENCES ent_edt_seance (id, etablissement_id) ON DELETE CASCADE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_seance_intervenant_agent_fk') THEN
        ALTER TABLE ent_edt_seance_intervenant ADD CONSTRAINT ent_edt_seance_intervenant_agent_fk
            FOREIGN KEY (agent_id, etablissement_id) REFERENCES ent_agent (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_seance_public_fusion_seance_fk') THEN
        ALTER TABLE ent_edt_seance_public_fusion ADD CONSTRAINT ent_edt_seance_public_fusion_seance_fk
            FOREIGN KEY (seance_id, etablissement_id) REFERENCES ent_edt_seance (id, etablissement_id) ON DELETE CASCADE;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_seance_public_fusion_cours_fk') THEN
        ALTER TABLE ent_edt_seance_public_fusion ADD CONSTRAINT ent_edt_seance_public_fusion_cours_fk
            FOREIGN KEY (cours_id, etablissement_id) REFERENCES ent_cours (id, etablissement_id);
    END IF;
END
$$;
