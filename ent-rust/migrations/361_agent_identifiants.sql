-- Socle 6 (personnels), lot P1, migration 361 : identifiants externes d'un agent
-- (`docs/socles/06_PERSONNELS.md` §8.11, §17).
--
-- Rattache un agent à son identifiant dans un système externe (STS-web, jointure AAF, guichet
-- Agents, GAR, logiciel d'emploi du temps, ancien logiciel, annuaire d'entreprise). Le NUMEN n'y
-- figure JAMAIS (Q-P15). Le `sub` d'une connexion fédérée reste au socle 1 (`ent_oidc_identity`).
-- Un identifiant actif désigne un seul agent par établissement et par système.
--
-- Additive et rejouable. RLS (politique des migrations 100 et 215).

CREATE TABLE IF NOT EXISTS ent_agent_identifiant (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    agent_id UUID NOT NULL,
    systeme TEXT NOT NULL CHECK (systeme IN (
        'sts', 'aaf_jointure', 'guichet_agents', 'gar', 'edt_logiciel', 'ancien_logiciel', 'annuaire_entreprise')),
    valeur TEXT NOT NULL CHECK (length(btrim(valeur)) BETWEEN 1 AND 200),
    du DATE NOT NULL,
    au DATE,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_agent_identifiant_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_agent_identifiant_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id)
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_agent_identifiant_actif_uq
    ON ent_agent_identifiant (etablissement_id, systeme, valeur) WHERE au IS NULL;
CREATE INDEX IF NOT EXISTS ent_agent_identifiant_agent_idx
    ON ent_agent_identifiant (etablissement_id, agent_id);

ALTER TABLE ent_agent_identifiant ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_agent_identifiant FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_agent_identifiant;
CREATE POLICY ent_rls_etablissement ON ent_agent_identifiant FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
           OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
                OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));
