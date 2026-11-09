-- Socle 6 (personnels), lot P2, migration 357 : casquettes et invitations
-- (`docs/socles/06_PERSONNELS.md` §8.4, §10.2, §13.2, §17).
--
-- Avant : un compte n'avait qu'UN profil par établissement (`ent_membership.role`). Un parent
-- d'élève devenu professeur ne pouvait pas être les deux, et le mot de passe temporaire avait été
-- remplacé (lot P0) par un lien de réinitialisation qui ne distingue pas « première activation »
-- d'« oubli », ne se rattache à aucun agent et n'a pas de QR code.
--
-- Après :
--  * `ent_membership_casquette` : les casquettes d'un compte dans un établissement (`personnel`,
--    `parent`…). Une session = une casquette (`ent_session.casquette`) ; les droits de la session
--    sont ceux de la casquette, JAMAIS l'union. `passkey_requise` : l'enrôlement d'une passkey est
--    exigé pour la casquette personnel, quelle que soit la politique de l'établissement (D-009) ;
--  * `ent_agent_invitation` : invitation d'un agent (lien par e-mail, 7 jours, ou QR code, 10
--    minutes). Le jeton n'est JAMAIS stocké (empreinte SHA-256) ni relu après l'émission ;
--  * `ent_agent_avant_arrivee` : l'agent n'a pas encore d'engagement courant mais en a un à venir
--    (accès réduit « préparer mon arrivée », PRS-25) ;
--  * `ent_casquette_ouverte` : une casquette permet d'agir aujourd'hui.
--
-- Additive et rejouable. RLS (même politique que les migrations 100 et 215) sur les deux tables.
-- Aucune casquette n'est créée par la migration : un accès existant garde son profil unique
-- (casquette `principale`), le comportement est inchangé tant qu'aucune invitation n'est acceptée.

-- 1. Casquettes -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_membership_casquette (
    user_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    casquette TEXT NOT NULL CHECK (casquette IN ('personnel', 'parent', 'eleve', 'intervenant')),
    -- Pour `personnel` : profil de base (celui de `ent_membership.role`).
    profil_base TEXT CHECK (profil_base IS NULL OR profil_base IN (
        'admin', 'direction', 'administratif', 'cpe', 'compta', 'prof', 'infirmerie', 'psy', 'accueil')),
    -- Agent (personnel), personne (parent) ou élève selon la casquette.
    source_ref UUID,
    du DATE NOT NULL,
    au DATE,
    etat TEXT NOT NULL DEFAULT 'ouvert' CHECK (etat IN ('ouvert', 'suspendu', 'ferme')),
    -- Casquette personnel : passkey exigée même si la politique de l'établissement ne l'exige pas.
    passkey_requise BOOLEAN NOT NULL DEFAULT false,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    PRIMARY KEY (user_id, etablissement_id, casquette),
    CONSTRAINT ent_casquette_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_casquette_profil_chk CHECK (casquette = 'personnel' OR profil_base IS NULL),
    CONSTRAINT ent_casquette_membership_fk FOREIGN KEY (user_id, etablissement_id)
        REFERENCES ent_membership (user_id, etablissement_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ent_casquette_etab_idx ON ent_membership_casquette (etablissement_id);

-- Casquette de la session : `principale` = le profil de l'accès (`ent_membership.role`) ;
-- `parent` = la casquette parent d'un compte dont le profil est celui d'un personnel.
ALTER TABLE ent_session ADD COLUMN IF NOT EXISTS casquette TEXT NOT NULL DEFAULT 'principale';
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint
                    WHERE conrelid = 'ent_session'::regclass AND conname = 'ent_session_casquette_chk') THEN
        ALTER TABLE ent_session
            ADD CONSTRAINT ent_session_casquette_chk CHECK (casquette IN ('principale', 'parent'));
    END IF;
END $$;

-- Une casquette permet d'agir aujourd'hui (jour de l'établissement) : ouverte et dans ses dates.
CREATE OR REPLACE FUNCTION ent_casquette_ouverte(p_user UUID, p_etab UUID, p_casquette TEXT)
RETURNS BOOLEAN
LANGUAGE sql STABLE
AS $$
    SELECT EXISTS (
        SELECT 1 FROM ent_membership_casquette c
         WHERE c.user_id = p_user AND c.etablissement_id = p_etab AND c.casquette = p_casquette
           AND c.etat = 'ouvert'
           AND c.du <= ent_aujourdhui_etab(p_etab)
           AND (c.au IS NULL OR c.au >= ent_aujourdhui_etab(p_etab)))
$$;

-- 2. Invitations ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_agent_invitation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    agent_id UUID NOT NULL,
    canal TEXT NOT NULL CHECK (canal IN ('email', 'qr')),
    -- Copie de la coordonnée au moment de l'envoi (canal e-mail seulement). Jamais journalisée.
    email_cible TEXT CHECK (email_cible IS NULL OR (length(email_cible) BETWEEN 3 AND 254 AND email_cible = lower(email_cible))),
    -- Profil que l'accès recevra à l'acceptation.
    profil_base TEXT NOT NULL CHECK (profil_base IN (
        'admin', 'direction', 'administratif', 'cpe', 'compta', 'prof', 'infirmerie', 'psy', 'accueil')),
    -- SHA-256 du jeton : le jeton lui-même n'est stocké nulle part et ne peut pas être relu.
    jeton_hash TEXT NOT NULL CHECK (length(jeton_hash) = 64),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID NOT NULL,
    expire_le TIMESTAMPTZ NOT NULL,
    utilisee_le TIMESTAMPTZ,
    user_id_rattache UUID,
    -- L'invitation a rattaché une identité qui existait déjà (après confirmation explicite).
    rattachement_identite_existante BOOLEAN NOT NULL DEFAULT false,
    annulee_le TIMESTAMPTZ,
    annulee_par_user_id UUID,
    -- Nombre de relances (PRS-23) : informatif.
    relance_de UUID,
    arrivee_signalee_le TIMESTAMPTZ,
    CONSTRAINT ent_invitation_jeton_uq UNIQUE (jeton_hash),
    CONSTRAINT ent_invitation_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_invitation_expire_chk CHECK (expire_le > cree_le),
    CONSTRAINT ent_invitation_canal_email_chk CHECK ((canal = 'email') = (email_cible IS NOT NULL)),
    CONSTRAINT ent_invitation_issue_chk CHECK (NOT (utilisee_le IS NOT NULL AND annulee_le IS NOT NULL)),
    CONSTRAINT ent_invitation_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id)
);
-- Au plus UNE invitation en attente (ni utilisée ni annulée) par agent : relancer = annuler + émettre.
CREATE UNIQUE INDEX IF NOT EXISTS ent_invitation_en_attente_uq
    ON ent_agent_invitation (etablissement_id, agent_id)
    WHERE utilisee_le IS NULL AND annulee_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_invitation_agent_idx ON ent_agent_invitation (etablissement_id, agent_id);

-- 3. Accès réduit avant la date d'arrivée (PRS-25) -------------------------------------------
-- Vrai quand l'agent du compte n'a AUCUN engagement courant (non annulé) mais en a un qui commence
-- après aujourd'hui. Un agent sans aucun engagement, ou dont tous les engagements sont passés, n'est
-- pas « avant arrivée » : la fermeture éventuelle relève de l'état de l'accès (migration 350).
CREATE OR REPLACE FUNCTION ent_agent_avant_arrivee(p_user UUID, p_etab UUID)
RETURNS BOOLEAN
LANGUAGE sql STABLE
AS $$
    SELECT EXISTS (
        SELECT 1
          FROM ent_personne p
          JOIN ent_agent a ON a.etablissement_id = p.etablissement_id AND a.personne_id = p.id
         WHERE p.etablissement_id = p_etab AND p.user_id = p_user
           AND EXISTS (SELECT 1 FROM ent_agent_engagement e
                        WHERE e.etablissement_id = a.etablissement_id AND e.agent_id = a.id
                          AND e.annule_le IS NULL AND e.du > ent_aujourdhui_etab(p_etab))
           AND NOT EXISTS (SELECT 1 FROM ent_agent_engagement e
                            WHERE e.etablissement_id = a.etablissement_id AND e.agent_id = a.id
                              AND e.annule_le IS NULL AND e.du <= ent_aujourdhui_etab(p_etab)
                              AND (e.au IS NULL OR e.au >= ent_aujourdhui_etab(p_etab))))
$$;

-- 4. RLS (politique des migrations 100 et 215) -----------------------------------------------
DO $$
DECLARE
    t text;
    tables text[] := ARRAY['ent_membership_casquette', 'ent_agent_invitation'];
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
