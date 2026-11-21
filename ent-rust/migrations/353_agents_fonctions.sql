-- Socle 6 (personnels), lot P1, migration 353 : fonctions, disciplines, qualifications, contact
-- d'urgence (`docs/socles/06_PERSONNELS.md` §8.3, §17 ; R-P14).
--
--  * `ent_fonction` : catalogue. `etablissement_id NULL` = fonction fournie par ScolIA (codes du
--    logiciel, libellés modifiables par l'établissement en créant la sienne) ; sinon fonction créée
--    par l'établissement (« animateur en pastorale », « coordinatrice ULIS »…). Informative : elle
--    PROPOSE un profil de base, elle n'accorde rien (D-017). `profil_propose` remplace
--    `modeles_roles_proposes UUID[]` de la conception : les modèles de rôles propres à un
--    établissement n'existent qu'au lot P4.
--  * `ent_agent_fonction`, `ent_agent_discipline`, `ent_agent_qualification` : lignes datées,
--    jamais supprimées (annulées en cas d'erreur).
--  * `ent_agent_contact_urgence` : facultatif, sur décision de l'agent, lisible de la direction
--    seule. Le contact y est porté en clair sur la ligne (nom, lien, téléphone) et NON comme une
--    `ent_personne` : une personne du socle 4 apparaîtrait dans la recherche de personnes du
--    secrétariat, ce que « visible de la direction seule » interdit (écart assumé, à valider).
--  * la reprise donne à chaque agent une fonction principale déduite du profil du compte, et le
--    déclencheur de 352 est complété pour les nouveaux comptes.
--
-- Additive et rejouable. RLS sur toutes les tables.

-- 1. Catalogue des fonctions -----------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_fonction (
    id UUID PRIMARY KEY,
    etablissement_id UUID
        CHECK (etablissement_id IS NULL OR etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    code TEXT NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{1,59}$'),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    famille TEXT NOT NULL CHECK (famille IN (
        'direction', 'enseignement', 'education', 'administration', 'sante_social',
        'accompagnement', 'service_agents', 'technique', 'intervention', 'autre')),
    -- Profil de base (modèle de droits historique) proposé à l'arrivée ; jamais accordé seul.
    profil_propose TEXT CHECK (profil_propose IS NULL OR profil_propose IN (
        'admin', 'direction', 'administratif', 'cpe', 'compta', 'prof', 'infirmerie', 'psy', 'accueil')),
    -- Fonction STS / AAF si elle existe [NV].
    code_national TEXT CHECK (code_national IS NULL OR length(code_national) <= 20),
    archivee_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- Un code est unique dans le catalogue fourni, et unique dans un établissement.
CREATE UNIQUE INDEX IF NOT EXISTS ent_fonction_code_uq
    ON ent_fonction (COALESCE(etablissement_id, '00000000-0000-0000-0000-000000000000'::uuid), code);

INSERT INTO ent_fonction (id, etablissement_id, code, libelle, famille, profil_propose) VALUES
    (gen_random_uuid(), NULL, 'chef_etablissement', 'Chef d''établissement', 'direction', 'direction'),
    (gen_random_uuid(), NULL, 'adjoint', 'Adjoint', 'direction', 'direction'),
    (gen_random_uuid(), NULL, 'directeur_delegue_formations', 'Directeur délégué aux formations', 'direction', 'direction'),
    (gen_random_uuid(), NULL, 'directeur_ecole', 'Directeur d''école', 'direction', 'direction'),
    (gen_random_uuid(), NULL, 'gestionnaire', 'Gestionnaire ou adjoint gestionnaire', 'administration', 'compta'),
    (gen_random_uuid(), NULL, 'secretaire_direction', 'Secrétaire de direction', 'administration', 'administratif'),
    (gen_random_uuid(), NULL, 'secretaire_scolarite', 'Secrétaire de scolarité', 'administration', 'administratif'),
    (gen_random_uuid(), NULL, 'comptable', 'Comptable', 'administration', 'compta'),
    (gen_random_uuid(), NULL, 'enseignant', 'Enseignant', 'enseignement', 'prof'),
    (gen_random_uuid(), NULL, 'documentaliste', 'Documentaliste', 'enseignement', 'prof'),
    (gen_random_uuid(), NULL, 'cpe', 'Conseiller principal d''éducation', 'education', 'cpe'),
    (gen_random_uuid(), NULL, 'aed', 'Assistant d''éducation', 'education', 'cpe'),
    (gen_random_uuid(), NULL, 'surveillant', 'Surveillant', 'education', 'cpe'),
    (gen_random_uuid(), NULL, 'educateur_internat', 'Éducateur d''internat', 'education', 'cpe'),
    (gen_random_uuid(), NULL, 'aesh', 'Accompagnant d''élèves en situation de handicap', 'accompagnement', NULL),
    (gen_random_uuid(), NULL, 'infirmier', 'Infirmier', 'sante_social', 'infirmerie'),
    (gen_random_uuid(), NULL, 'assistant_service_social', 'Assistant de service social', 'sante_social', NULL),
    (gen_random_uuid(), NULL, 'psychologue', 'Psychologue', 'sante_social', 'psy'),
    (gen_random_uuid(), NULL, 'medecin', 'Médecin', 'sante_social', NULL),
    (gen_random_uuid(), NULL, 'agent_accueil', 'Agent d''accueil', 'service_agents', 'accueil'),
    (gen_random_uuid(), NULL, 'agent_entretien', 'Agent d''entretien', 'service_agents', NULL),
    (gen_random_uuid(), NULL, 'agent_restauration', 'Agent de restauration', 'service_agents', NULL),
    (gen_random_uuid(), NULL, 'chef_cuisine', 'Chef de cuisine', 'service_agents', NULL),
    (gen_random_uuid(), NULL, 'agent_maintenance', 'Agent de maintenance', 'technique', NULL),
    (gen_random_uuid(), NULL, 'technicien_informatique', 'Technicien informatique', 'technique', NULL),
    (gen_random_uuid(), NULL, 'technicien_laboratoire', 'Technicien de laboratoire', 'technique', NULL),
    (gen_random_uuid(), NULL, 'animateur', 'Animateur', 'intervention', NULL),
    (gen_random_uuid(), NULL, 'intervenant_exterieur', 'Intervenant extérieur', 'intervention', NULL),
    (gen_random_uuid(), NULL, 'benevole', 'Bénévole', 'intervention', NULL),
    -- Fonction d'attente de la reprise (un compte « admin » ou une nature inconnue) : à préciser.
    (gen_random_uuid(), NULL, 'a_preciser', 'À préciser', 'autre', NULL)
ON CONFLICT DO NOTHING;

-- 2. Fonctions d'un agent -----------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_agent_fonction (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    agent_id UUID NOT NULL,
    fonction_id UUID NOT NULL REFERENCES ent_fonction (id),
    principale BOOLEAN NOT NULL DEFAULT false,
    du DATE NOT NULL,
    au DATE,
    annule_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_agent_fonction_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_agent_fonction_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id),
    -- Une seule fonction principale à une date ; la même fonction ne se recouvre pas.
    CONSTRAINT ent_agent_fonction_une_principale EXCLUDE USING gist (
        etablissement_id WITH =, agent_id WITH =, daterange(du, au, '[]') WITH &&
    ) WHERE (principale AND annule_le IS NULL),
    CONSTRAINT ent_agent_fonction_sans_doublon EXCLUDE USING gist (
        etablissement_id WITH =, agent_id WITH =, fonction_id WITH =, daterange(du, au, '[]') WITH &&
    ) WHERE (annule_le IS NULL)
);
CREATE INDEX IF NOT EXISTS ent_agent_fonction_agent_idx ON ent_agent_fonction (etablissement_id, agent_id, du);

-- Une fonction est fournie (établissement NULL) ou créée par CET établissement.
CREATE OR REPLACE FUNCTION ent_agent_fonction_coherence() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM ent_fonction f
                    WHERE f.id = NEW.fonction_id
                      AND (f.etablissement_id IS NULL OR f.etablissement_id = NEW.etablissement_id)) THEN
        RAISE EXCEPTION 'fonction d''un autre etablissement' USING ERRCODE = '23503';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_agent_fonction_coherence_trg ON ent_agent_fonction;
CREATE TRIGGER ent_agent_fonction_coherence_trg
    BEFORE INSERT OR UPDATE OF fonction_id, etablissement_id ON ent_agent_fonction
    FOR EACH ROW EXECUTE FUNCTION ent_agent_fonction_coherence();

-- 3. Disciplines de poste (enseignants) ----------------------------------------------------------
CREATE UNIQUE INDEX IF NOT EXISTS ent_matiere_etab_id_uq ON ent_matiere (etablissement_id, id);

CREATE TABLE IF NOT EXISTS ent_agent_discipline (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    agent_id UUID NOT NULL,
    matiere_id UUID,
    code_discipline_poste TEXT CHECK (code_discipline_poste IS NULL OR length(code_discipline_poste) <= 20),
    principale BOOLEAN NOT NULL DEFAULT false,
    du DATE NOT NULL,
    au DATE,
    annule_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_agent_discipline_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_agent_discipline_cible_chk CHECK (matiere_id IS NOT NULL OR code_discipline_poste IS NOT NULL),
    CONSTRAINT ent_agent_discipline_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id),
    CONSTRAINT ent_agent_discipline_matiere_fk FOREIGN KEY (etablissement_id, matiere_id)
        REFERENCES ent_matiere (etablissement_id, id),
    CONSTRAINT ent_agent_discipline_une_principale EXCLUDE USING gist (
        etablissement_id WITH =, agent_id WITH =, daterange(du, au, '[]') WITH &&
    ) WHERE (principale AND annule_le IS NULL)
);
CREATE INDEX IF NOT EXISTS ent_agent_discipline_agent_idx ON ent_agent_discipline (etablissement_id, agent_id, du);

-- 4. Qualifications ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_agent_qualification (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    agent_id UUID NOT NULL,
    code TEXT NOT NULL CHECK (code IN (
        'secourisme', 'encadrement_sortie', 'permis_b', 'permis_transport', 'langue',
        'habilitation_electrique', 'surveillance_baignade', 'autre',
        -- R-P14 [NV]
        'sst', 'ssiap', 'caces', 'haccp', 'bafa', 'bnssa')),
    precision TEXT CHECK (precision IS NULL OR length(precision) <= 80),
    obtenue_le DATE,
    valable_jusqu_au DATE,
    -- Pièce justificative facultative (socle 7, sans clé étrangère tant qu'il n'existe pas).
    piece_document_id UUID,
    annule_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_agent_qualification_dates_chk
        CHECK (valable_jusqu_au IS NULL OR obtenue_le IS NULL OR valable_jusqu_au >= obtenue_le),
    CONSTRAINT ent_agent_qualification_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_agent_qualification_agent_idx ON ent_agent_qualification (etablissement_id, agent_id);
CREATE INDEX IF NOT EXISTS ent_agent_qualification_echeance_idx
    ON ent_agent_qualification (etablissement_id, code, valable_jusqu_au) WHERE annule_le IS NULL;

-- 5. Contact d'urgence ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_agent_contact_urgence (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    agent_id UUID NOT NULL,
    ordre SMALLINT NOT NULL CHECK (ordre BETWEEN 1 AND 3),
    nom TEXT NOT NULL CHECK (length(btrim(nom)) BETWEEN 1 AND 120),
    prenom TEXT CHECK (prenom IS NULL OR length(prenom) <= 120),
    lien TEXT CHECK (lien IS NULL OR lien IN ('conjoint', 'parent', 'enfant', 'proche', 'autre')),
    telephone TEXT NOT NULL CHECK (length(telephone) BETWEEN 6 AND 40),
    du DATE NOT NULL DEFAULT CURRENT_DATE,
    au DATE,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_agent_contact_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_agent_contact_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id),
    CONSTRAINT ent_agent_contact_un_par_rang EXCLUDE USING gist (
        etablissement_id WITH =, agent_id WITH =, ordre WITH =, daterange(du, au, '[]') WITH &&
    )
);

-- 6. Fonction principale : reprise et nouveaux comptes ---------------------------------------------
CREATE OR REPLACE FUNCTION ent_fonction_du_profil(p_role TEXT) RETURNS UUID
LANGUAGE sql STABLE
AS $$
    SELECT id FROM ent_fonction
     WHERE etablissement_id IS NULL
       AND code = CASE p_role
            WHEN 'prof' THEN 'enseignant'
            WHEN 'cpe' THEN 'cpe'
            WHEN 'infirmerie' THEN 'infirmier'
            WHEN 'psy' THEN 'psychologue'
            WHEN 'compta' THEN 'comptable'
            WHEN 'administratif' THEN 'secretaire_scolarite'
            WHEN 'direction' THEN 'chef_etablissement'
            WHEN 'accueil' THEN 'agent_accueil'
            ELSE 'a_preciser' END
$$;

-- Complète le déclencheur de 352 : l'agent créé avec le compte reçoit sa fonction principale.
CREATE OR REPLACE FUNCTION ent_membership_agent_sync() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    pid UUID;
    aid UUID;
    cree BOOLEAN := false;
    debut DATE;
BEGIN
    IF NEW.role IN ('parent', 'eleve') THEN
        RETURN NEW;
    END IF;
    SELECT id INTO pid FROM ent_personne
     WHERE etablissement_id = NEW.etablissement_id AND user_id = NEW.user_id;
    IF pid IS NULL THEN
        INSERT INTO ent_personne (id, etablissement_id, nom, email, user_id, source)
        SELECT gen_random_uuid(), NEW.etablissement_id,
               left(COALESCE(NULLIF(btrim(u.display_name), ''), u.email), 120),
               lower(u.email), u.id, 'compte'
          FROM ent_user u WHERE u.id = NEW.user_id
        ON CONFLICT (etablissement_id, user_id) WHERE user_id IS NOT NULL DO NOTHING;
        SELECT id INTO pid FROM ent_personne
         WHERE etablissement_id = NEW.etablissement_id AND user_id = NEW.user_id;
        cree := true;
    END IF;
    IF pid IS NULL THEN
        RETURN NEW;
    END IF;
    SELECT id INTO aid FROM ent_agent WHERE etablissement_id = NEW.etablissement_id AND personne_id = pid;
    IF aid IS NULL THEN
        aid := gen_random_uuid();
        INSERT INTO ent_agent (id, etablissement_id, personne_id, source, identite_a_separer)
        VALUES (aid, NEW.etablissement_id, pid, 'saisie', cree);
        debut := ent_aujourdhui_etab(NEW.etablissement_id, COALESCE(NEW.created_at, now()));
        INSERT INTO ent_agent_engagement (id, etablissement_id, agent_id, du, au, nature, employeur_type)
        VALUES (gen_random_uuid(), NEW.etablissement_id, aid, debut,
                CASE WHEN NEW.date_fin IS NULL THEN NULL ELSE GREATEST(debut, NEW.date_fin) END,
                'a_preciser', 'inconnu')
        ON CONFLICT DO NOTHING;
        INSERT INTO ent_agent_fonction (id, etablissement_id, agent_id, fonction_id, principale, du)
        SELECT gen_random_uuid(), NEW.etablissement_id, aid, ent_fonction_du_profil(NEW.role), true, debut
         WHERE ent_fonction_du_profil(NEW.role) IS NOT NULL
        ON CONFLICT DO NOTHING;
    END IF;
    RETURN NEW;
END
$$;

-- Reprise : aucun agent du personnel sans fonction principale. La fonction est déduite du profil
-- du compte (prof → enseignant, cpe → CPE, infirmerie → infirmier, psy → psychologue, compta →
-- comptable, administratif → secrétaire de scolarité, direction → chef d'établissement,
-- accueil → agent d'accueil, admin → à préciser). Un agent sans compte reçoit « à préciser ».
INSERT INTO ent_agent_fonction (id, etablissement_id, agent_id, fonction_id, principale, du)
SELECT gen_random_uuid(), a.etablissement_id, a.id,
       COALESCE(ent_fonction_du_profil(m.role), ent_fonction_du_profil('admin')),
       true,
       COALESCE((SELECT min(e.du) FROM ent_agent_engagement e
                  WHERE e.etablissement_id = a.etablissement_id AND e.agent_id = a.id AND e.annule_le IS NULL),
                ent_aujourdhui_etab(a.etablissement_id))
  FROM ent_agent a
  LEFT JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
  LEFT JOIN ent_membership m ON m.user_id = p.user_id AND m.etablissement_id = a.etablissement_id
 WHERE NOT EXISTS (SELECT 1 FROM ent_agent_fonction f
                    WHERE f.etablissement_id = a.etablissement_id AND f.agent_id = a.id
                      AND f.principale AND f.annule_le IS NULL)
ON CONFLICT DO NOTHING;

-- 7. RLS --------------------------------------------------------------------------------------------
DO $$
DECLARE
    t text;
    tables text[] := ARRAY['ent_agent_fonction', 'ent_agent_discipline', 'ent_agent_qualification',
                           'ent_agent_contact_urgence'];
BEGIN
    FOREACH t IN ARRAY tables LOOP
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
    END LOOP;
END
$$;

-- Catalogue : les fonctions fournies (établissement NULL) sont lisibles de tous ; seules les
-- fonctions de l'établissement courant s'écrivent.
ALTER TABLE ent_fonction ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_fonction FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_fonction;
CREATE POLICY ent_rls_etablissement ON ent_fonction FOR ALL
    USING (etablissement_id IS NULL
           OR etablissement_id = (SELECT ent_rls_etablissement())
           OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
                OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));
