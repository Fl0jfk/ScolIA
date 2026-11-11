-- Socle 20, lot C1 — migration 2410 : visa élargi (cours, classe ou agent), référence figée de ce qui a été vu,
-- observations et réponses, journal des consultations, alertes de charge ignorées.
-- Réf. : 20_CAHIER_DE_TEXTES.md §24.1, §14, R-C17, R-C18. L'accès d'inspection / tiers (`ent_cdt_acces_*`) est hors lot C1 (C8).
-- `ent_cdt_visa` (150) est ÉLARGIE, pas recréée (« reprise de 150, élargie ») : les visas existants restent des visas de classe
-- (« visa antérieur au cahier versionné » : aucune référence figée).

ALTER TABLE ent_cdt_visa ADD COLUMN IF NOT EXISTS cible TEXT NOT NULL DEFAULT 'classe';
ALTER TABLE ent_cdt_visa ADD COLUMN IF NOT EXISTS cours_id UUID;
ALTER TABLE ent_cdt_visa ADD COLUMN IF NOT EXISTS agent_id UUID;
ALTER TABLE ent_cdt_visa ADD COLUMN IF NOT EXISTS avec_observation BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_cdt_visa ALTER COLUMN classe_id DROP NOT NULL;
ALTER TABLE ent_cdt_visa DROP CONSTRAINT IF EXISTS ent_cdt_visa_cible_chk;
ALTER TABLE ent_cdt_visa ADD CONSTRAINT ent_cdt_visa_cible_chk CHECK (
    (cible = 'classe' AND classe_id IS NOT NULL)
    OR (cible = 'cours' AND cours_id IS NOT NULL)
    OR (cible = 'agent' AND agent_id IS NOT NULL));
CREATE UNIQUE INDEX IF NOT EXISTS ent_cdt_visa_etab_id_uq ON ent_cdt_visa (etablissement_id, id);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_cdt_visa_cours_fk') THEN
        ALTER TABLE ent_cdt_visa ADD CONSTRAINT ent_cdt_visa_cours_fk
            FOREIGN KEY (cours_id, etablissement_id) REFERENCES ent_cours (id, etablissement_id) ON DELETE RESTRICT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_cdt_visa_agent_fk') THEN
        ALTER TABLE ent_cdt_visa ADD CONSTRAINT ent_cdt_visa_agent_fk
            FOREIGN KEY (etablissement_id, agent_id) REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT;
    END IF;
END
$$;

-- Ce que le visa a vu : référence figée (objet + version) ; une modification postérieure est marquée « modifiée après visa ».
CREATE TABLE IF NOT EXISTS ent_cdt_visa_element (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    visa_id UUID NOT NULL,
    objet TEXT NOT NULL CHECK (objet IN ('entree', 'travail')),
    objet_id UUID NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_visa_element_uq UNIQUE (visa_id, objet, objet_id),
    CONSTRAINT ent_cdt_visa_element_visa_fk FOREIGN KEY (etablissement_id, visa_id)
        REFERENCES ent_cdt_visa (etablissement_id, id) ON DELETE RESTRICT
);
DROP TRIGGER IF EXISTS ent_cdt_visa_element_ajout_seul ON ent_cdt_visa_element;
CREATE TRIGGER ent_cdt_visa_element_ajout_seul BEFORE UPDATE OR DELETE ON ent_cdt_visa_element
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_ajout_seul();

CREATE TABLE IF NOT EXISTS ent_cdt_observation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    visa_id UUID NOT NULL,
    texte TEXT NOT NULL CHECK (length(btrim(texte)) > 0 AND length(texte) <= 4000),
    par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_observation_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_observation_visa_fk FOREIGN KEY (etablissement_id, visa_id)
        REFERENCES ent_cdt_visa (etablissement_id, id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS ent_cdt_observation_reponse (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    observation_id UUID NOT NULL,
    texte TEXT NOT NULL CHECK (length(btrim(texte)) > 0 AND length(texte) <= 4000),
    par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_observation_reponse_fk FOREIGN KEY (etablissement_id, observation_id)
        REFERENCES ent_cdt_observation (etablissement_id, id) ON DELETE RESTRICT
);

-- Journal des consultations : ajout seul (lecture d'inspection, lecture de copie sur motif, agenda lu par la vie scolaire).
CREATE TABLE IF NOT EXISTS ent_cdt_consultation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    qui_user_id UUID NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN ('inspection', 'copie_motif', 'agenda_vie_scolaire')),
    objet_id UUID,
    motif TEXT CHECK (motif IS NULL OR length(motif) <= 500),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now()
);
DROP TRIGGER IF EXISTS ent_cdt_consultation_ajout_seul ON ent_cdt_consultation;
CREATE TRIGGER ent_cdt_consultation_ajout_seul BEFORE UPDATE OR DELETE ON ent_cdt_consultation
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_ajout_seul();

-- Alertes de charge ignorées : comptées pour le pilotage, sans auteur exposé (R-C16).
CREATE TABLE IF NOT EXISTS ent_cdt_alerte_charge (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    travail_id UUID NOT NULL,
    repere TEXT NOT NULL CHECK (repere IN ('minutes', 'evaluations', 'travaux')),
    valeur INTEGER NOT NULL,
    seuil INTEGER NOT NULL,
    ignoree BOOLEAN NOT NULL DEFAULT false,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (travail_id, repere),
    CONSTRAINT ent_cdt_alerte_charge_travail_fk FOREIGN KEY (etablissement_id, travail_id)
        REFERENCES ent_cdt_travail (etablissement_id, id) ON DELETE RESTRICT
);

SELECT ent_rls_poser(ARRAY['ent_cdt_visa_element', 'ent_cdt_observation', 'ent_cdt_observation_reponse',
                           'ent_cdt_consultation', 'ent_cdt_alerte_charge']);
