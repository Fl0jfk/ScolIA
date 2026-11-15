-- Personnels, lot P6 (06_PERSONNELS.md §8.9, PRS-80 à 84) : DÉLÉGATIONS par agent et POUVOIRS DE SIGNATURE.
-- `ent_delegation` (172, 560) existe : délégation de droits datée entre deux comptes (fin obligatoire, anti-escalade,
-- expiration au jour de l'établissement). P6 l'ENRICHIT sans la dupliquer : objet (rôle, mission, signature, intérim de
-- direction), agents délégant et délégataire (projection des comptes), motif codé, avertissement avant l'expiration.
-- `ent_habilitation_signature` porte les pouvoirs de signature (socle 7 : `peut_signer`). Migration additive,
-- rejouable, sans JSON ni jour UTC (le jour est celui de l'établissement : `ent_aujourdhui_etab`).

ALTER TABLE ent_delegation ADD COLUMN IF NOT EXISTS delegant_agent_id UUID;
ALTER TABLE ent_delegation ADD COLUMN IF NOT EXISTS delegataire_agent_id UUID;
ALTER TABLE ent_delegation ADD COLUMN IF NOT EXISTS objet TEXT NOT NULL DEFAULT 'role';
-- Mission copiée (objet `mission`) ou aucune ; l'habilitation dérivée pointe vers la délégation, pas l'inverse.
ALTER TABLE ent_delegation ADD COLUMN IF NOT EXISTS objet_ref UUID;
ALTER TABLE ent_delegation ADD COLUMN IF NOT EXISTS motif_code TEXT NOT NULL DEFAULT 'autre';
-- Avertissement « elle expire dans trois jours » : publié une seule fois, horodaté.
ALTER TABLE ent_delegation ADD COLUMN IF NOT EXISTS prevenue_le TIMESTAMPTZ;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_delegation_objet_chk') THEN
        ALTER TABLE ent_delegation ADD CONSTRAINT ent_delegation_objet_chk
            CHECK (objet IN ('role', 'mission', 'signature', 'interim_direction'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_delegation_motif_chk') THEN
        ALTER TABLE ent_delegation ADD CONSTRAINT ent_delegation_motif_chk
            CHECK (motif_code IN ('absence', 'surcharge', 'organisation', 'autre'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_delegation_delegant_agent_fk') THEN
        ALTER TABLE ent_delegation ADD CONSTRAINT ent_delegation_delegant_agent_fk
            FOREIGN KEY (etablissement_id, delegant_agent_id) REFERENCES ent_agent (etablissement_id, id) NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_delegation_delegataire_agent_fk') THEN
        ALTER TABLE ent_delegation ADD CONSTRAINT ent_delegation_delegataire_agent_fk
            FOREIGN KEY (etablissement_id, delegataire_agent_id) REFERENCES ent_agent (etablissement_id, id) NOT VALID;
    END IF;
END
$$;

-- Intérim de direction : ses droits ne sont PAS recopiés en lignes (la limite de 20 droits de 560 vaut pour les
-- délégations saisies) : ils sont CALCULÉS à chaque requête (modèle du chef d'établissement, hors exclusions), donc
-- ils s'éteignent à la date de fin comme les autres, sans tâche de fond.

-- Reprise : agents des comptes (projection) pour les délégations existantes.
UPDATE ent_delegation d
   SET delegant_agent_id = (SELECT a.id FROM ent_agent a
                              JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
                             WHERE a.etablissement_id = d.etablissement_id AND p.user_id = d.delegant_user_id
                               AND a.fusionne_dans_agent_id IS NULL LIMIT 1)
 WHERE d.delegant_agent_id IS NULL;
UPDATE ent_delegation d
   SET delegataire_agent_id = (SELECT a.id FROM ent_agent a
                                 JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
                                WHERE a.etablissement_id = d.etablissement_id AND p.user_id = d.delegue_user_id
                                  AND a.fusionne_dans_agent_id IS NULL LIMIT 1)
 WHERE d.delegataire_agent_id IS NULL;

CREATE OR REPLACE FUNCTION ent_trg_delegation_agents() RETURNS trigger AS $$
BEGIN
    IF NEW.delegant_agent_id IS NULL THEN
        NEW.delegant_agent_id := (SELECT a.id FROM ent_agent a
                                    JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
                                   WHERE a.etablissement_id = NEW.etablissement_id AND p.user_id = NEW.delegant_user_id
                                     AND a.fusionne_dans_agent_id IS NULL LIMIT 1);
    END IF;
    IF NEW.delegataire_agent_id IS NULL THEN
        NEW.delegataire_agent_id := (SELECT a.id FROM ent_agent a
                                       JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
                                      WHERE a.etablissement_id = NEW.etablissement_id AND p.user_id = NEW.delegue_user_id
                                        AND a.fusionne_dans_agent_id IS NULL LIMIT 1);
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS ent_delegation_agents ON ent_delegation;
CREATE TRIGGER ent_delegation_agents BEFORE INSERT ON ent_delegation
    FOR EACH ROW EXECUTE FUNCTION ent_trg_delegation_agents();

-- Pouvoirs de signature (§8.9). Le chef d'établissement signe tout PAR SA FONCTION (aucune ligne) ; les autres agents
-- par habilitation permanente ou par délégation (fin obligatoire). `documents` : natures de documents du socle 7
-- (`certificat_scolarite`, `convention_stage`, `ordre_mission`, `bulletin`, `attestation`, `courrier_famille`…) ;
-- `*` = toutes les natures (intérim de direction).
CREATE TABLE IF NOT EXISTS ent_habilitation_signature (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    agent_id UUID NOT NULL,
    documents TEXT[] NOT NULL CHECK (cardinality(documents) BETWEEN 1 AND 40),
    du DATE NOT NULL,
    au DATE,
    origine TEXT NOT NULL CHECK (origine IN ('habilitation', 'delegation', 'interim')),
    -- Délégation qui l'a produite (origine `delegation` ou `interim`).
    origine_ref UUID,
    -- Formule imprimée sous la signature ; NULL = signature en son nom propre.
    formule TEXT CHECK (formule IS NULL OR length(btrim(formule)) BETWEEN 1 AND 160),
    revoquee_le TIMESTAMPTZ,
    revoquee_par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_habilitation_dates_chk CHECK (au IS NULL OR au >= du),
    -- « Les délégations expirent seules » : une délégation de signature a toujours une fin.
    CONSTRAINT ent_habilitation_fin_chk CHECK (origine = 'habilitation' OR au IS NOT NULL),
    CONSTRAINT ent_habilitation_origine_chk CHECK ((origine = 'habilitation') = (origine_ref IS NULL)),
    CONSTRAINT ent_habilitation_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_habilitation_agent_idx
    ON ent_habilitation_signature (etablissement_id, agent_id, du) WHERE revoquee_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_habilitation_origine_idx
    ON ent_habilitation_signature (etablissement_id, origine_ref) WHERE origine_ref IS NOT NULL;

DO $$
DECLARE
    t text := 'ent_habilitation_signature';
BEGIN
    EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
    EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
    EXECUTE format('DROP POLICY IF EXISTS ent_rls_etablissement ON %I', t);
    EXECUTE format(
        'CREATE POLICY ent_rls_etablissement ON %I FOR ALL '
        'USING (etablissement_id = (SELECT ent_rls_etablissement()) '
        '  OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict()))) '
        'WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement()) '
        '  OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))',
        t);
END
$$;
