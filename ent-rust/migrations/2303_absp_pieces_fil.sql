-- 2303 — Socle 15 (absences des personnels), lot A2 : pièces, fil, conservation
-- (`docs/socles/15_ABSENCES_PERSONNELS.md` §18.2, §18.4, §26, R-AP21, R-AP22, R-AP40, ABP-140 à 149 et 160 à 164).
-- Additive et rejouable (E8) ; aucune colonne JSON (D-036) ; RLS posée sur les nouvelles tables.
--
-- 1. `ent_absp_piece` : une pièce (arrêt, justificatif, convocation) d'une absence. Le fichier est dans
--    `ent-object` (objet immuable, registre `ent_objet`, migration 500) ; ici seulement la clé, l'empreinte et
--    l'état. AUCUN nom de fichier n'est conservé (minimisation : un nom d'arrêt peut dire la maladie).
--    Une pièce n'est jamais supprimée de la table : elle passe `deposee` → `retiree` → `purgee`.
-- 2. `ent_absp_piece_lecture` : qui a lu quelle pièce, quand (ajout seul, conservé après la purge : « qui a vu
--    mon arrêt ? », CAP-60).
-- 3. `ent_absp_message` : fil de l'absence entre l'agent, le valideur et le traitement administratif ; ajout
--    seul, 4 000 caractères, jamais de pièce jointe. Seule la purge programmée peut supprimer des lignes.
-- 4. `ent_absp_gel` : gel de conservation (contentieux) ; suspend la purge de l'absence.
-- 5. Paramètres de conservation (S-1502 plafond de relances ; durées de §18.4, à valider par le DPO, Q-AP12).

-- 1. Pièces ---------------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_absp_piece (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    absence_id UUID NOT NULL,
    type_piece TEXT NOT NULL CHECK (type_piece IN ('arret', 'justificatif', 'convocation', 'autre')),
    -- Clé d'objet opaque `{etablissement}/{espace}/{id}.{ext}` (E14) : préfixée par l'établissement.
    cle_objet TEXT NOT NULL,
    sha256 TEXT NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    taille_octets BIGINT NOT NULL CHECK (taille_octets > 0),
    mime TEXT NOT NULL CHECK (char_length(mime) BETWEEN 3 AND 100),
    etat TEXT NOT NULL DEFAULT 'deposee' CHECK (etat IN ('deposee', 'retiree', 'purgee')),
    deposee_par_user_id UUID NOT NULL,
    deposee_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    retiree_par_user_id UUID,
    retiree_le TIMESTAMPTZ,
    -- Date de purge calculée (R-AP21) : NULL tant qu'elle n'est pas connue (agent de l'État : 30 jours après
    -- la transmission, qui n'a peut-être pas encore eu lieu).
    purge_prevue_le DATE,
    purgee_le TIMESTAMPTZ,
    purgee_cause TEXT CHECK (purgee_cause IS NULL OR purgee_cause IN ('echeance', 'demande')),
    purgee_par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_piece_cle_chk CHECK (starts_with(cle_objet, etablissement_id::text || '/')),
    CONSTRAINT ent_absp_piece_retiree_chk CHECK (etat <> 'retiree' OR (retiree_le IS NOT NULL AND retiree_par_user_id IS NOT NULL)),
    CONSTRAINT ent_absp_piece_purgee_chk CHECK (
        (etat = 'purgee') = (purgee_le IS NOT NULL AND purgee_cause IS NOT NULL)),
    CONSTRAINT ent_absp_piece_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_piece_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_absp_absence (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_absp_piece_absence_idx ON ent_absp_piece (etablissement_id, absence_id, deposee_le);
CREATE INDEX IF NOT EXISTS ent_absp_piece_purge_idx
    ON ent_absp_piece (etablissement_id, purge_prevue_le) WHERE etat <> 'purgee';

-- Une pièce ne se supprime pas ; elle ne change que d'état, vers l'avant, et ses champs d'origine sont figés.
CREATE OR REPLACE FUNCTION ent_trg_absp_piece_garde() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_absp_piece ne se supprime pas (état purgee à la place)' USING ERRCODE = '23514';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.absence_id IS DISTINCT FROM OLD.absence_id OR NEW.type_piece IS DISTINCT FROM OLD.type_piece
       OR NEW.cle_objet IS DISTINCT FROM OLD.cle_objet OR NEW.sha256 IS DISTINCT FROM OLD.sha256
       OR NEW.taille_octets IS DISTINCT FROM OLD.taille_octets OR NEW.mime IS DISTINCT FROM OLD.mime
       OR NEW.deposee_par_user_id IS DISTINCT FROM OLD.deposee_par_user_id
       OR NEW.deposee_le IS DISTINCT FROM OLD.deposee_le THEN
        RAISE EXCEPTION 'ent_absp_piece : champs d''origine figés' USING ERRCODE = '23514';
    END IF;
    IF OLD.etat = 'purgee' AND NEW.etat IS DISTINCT FROM OLD.etat THEN
        RAISE EXCEPTION 'ent_absp_piece : une pièce purgée ne revient pas' USING ERRCODE = '23514';
    END IF;
    IF OLD.etat = 'retiree' AND NEW.etat = 'deposee' THEN
        RAISE EXCEPTION 'ent_absp_piece : une pièce retirée ne revient pas' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absp_piece_garde ON ent_absp_piece;
CREATE TRIGGER ent_absp_piece_garde BEFORE UPDATE OR DELETE ON ent_absp_piece
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_piece_garde();

-- 2. Lectures de pièces (ajout seul) -----------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_absp_piece_lecture (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    piece_id UUID NOT NULL,
    lecteur_user_id UUID NOT NULL,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_piece_lecture_piece_fk FOREIGN KEY (etablissement_id, piece_id)
        REFERENCES ent_absp_piece (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_absp_piece_lecture_idx ON ent_absp_piece_lecture (etablissement_id, piece_id, le);

CREATE OR REPLACE FUNCTION ent_trg_absp_ajout_seul() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION '% est en ajout seul (% interdit)', TG_TABLE_NAME, TG_OP USING ERRCODE = '23514';
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absp_piece_lecture_ajout_seul ON ent_absp_piece_lecture;
CREATE TRIGGER ent_absp_piece_lecture_ajout_seul BEFORE UPDATE OR DELETE ON ent_absp_piece_lecture
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_ajout_seul();

-- 3. Fil de messages (ajout seul ; la purge programmée seule peut supprimer) ----------------------------
CREATE TABLE IF NOT EXISTS ent_absp_message (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    absence_id UUID NOT NULL,
    auteur_user_id UUID NOT NULL,
    -- Rôle tel qu'il s'affiche dans le fil : jamais un nom saisi par le client.
    role_affiche TEXT NOT NULL CHECK (role_affiche IN ('agent', 'valideur', 'instructeur', 'direction')),
    texte VARCHAR(4000) NOT NULL CHECK (char_length(btrim(texte)) >= 1),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_message_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_absp_absence (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_absp_message_idx ON ent_absp_message (etablissement_id, absence_id, le);

CREATE OR REPLACE FUNCTION ent_trg_absp_message_garde() RETURNS trigger AS $$
BEGIN
    -- La seule suppression permise est celle de la purge de conservation (§18.4), qui pose ce réglage de
    -- transaction avant de supprimer et journalise la purge.
    IF TG_OP = 'DELETE' AND coalesce(current_setting('ent.absp_purge', true), '') = 'on' THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION '% est en ajout seul (% interdit)', TG_TABLE_NAME, TG_OP USING ERRCODE = '23514';
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absp_message_garde ON ent_absp_message;
CREATE TRIGGER ent_absp_message_garde BEFORE UPDATE OR DELETE ON ent_absp_message
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_message_garde();

-- 4. Gel de conservation ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ent_absp_gel (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    absence_id UUID NOT NULL,
    motif VARCHAR(500) NOT NULL CHECK (char_length(btrim(motif)) >= 3),
    du DATE NOT NULL,
    au DATE,
    pose_par_user_id UUID NOT NULL,
    pose_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    leve_le TIMESTAMPTZ,
    leve_par_user_id UUID,
    CONSTRAINT ent_absp_gel_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_absp_gel_leve_chk CHECK ((leve_le IS NULL) = (leve_par_user_id IS NULL)),
    CONSTRAINT ent_absp_gel_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_gel_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_absp_absence (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_absp_gel_idx ON ent_absp_gel (etablissement_id, absence_id) WHERE leve_le IS NULL;

-- Un gel ne se modifie pas ni ne se supprime ; il se LÈVE (une fois), ce qui est tracé.
CREATE OR REPLACE FUNCTION ent_trg_absp_gel_garde() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND OLD.leve_le IS NULL AND NEW.leve_le IS NOT NULL
       AND NEW.id = OLD.id AND NEW.etablissement_id = OLD.etablissement_id AND NEW.absence_id = OLD.absence_id
       AND NEW.motif = OLD.motif AND NEW.du = OLD.du AND NEW.au IS NOT DISTINCT FROM OLD.au
       AND NEW.pose_par_user_id = OLD.pose_par_user_id AND NEW.pose_le = OLD.pose_le THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'ent_absp_gel : un gel se lève, il ne se modifie ni ne se supprime' USING ERRCODE = '23514';
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absp_gel_garde ON ent_absp_gel;
CREATE TRIGGER ent_absp_gel_garde BEFORE UPDATE OR DELETE ON ent_absp_gel
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_gel_garde();

-- 5. Paramètres de conservation -------------------------------------------------------------------------
-- S-1502 : plafond de relances d'une pièce manquante. §18.4 (à valider par le DPO, Q-AP12) : pièce d'un agent
-- de l'État supprimée 30 jours après la transmission ; pièce d'un salarié de l'OGEC 3 ans après la fin de
-- l'année scolaire de l'absence.
ALTER TABLE ent_absp_param ADD COLUMN IF NOT EXISTS relances_piece_max SMALLINT NOT NULL DEFAULT 3
    CHECK (relances_piece_max BETWEEN 0 AND 10);
ALTER TABLE ent_absp_param ADD COLUMN IF NOT EXISTS purge_piece_etat_jours SMALLINT NOT NULL DEFAULT 30
    CHECK (purge_piece_etat_jours BETWEEN 1 AND 365);
ALTER TABLE ent_absp_param ADD COLUMN IF NOT EXISTS purge_piece_ogec_annees SMALLINT NOT NULL DEFAULT 3
    CHECK (purge_piece_ogec_annees BETWEEN 1 AND 10);

SELECT ent_rls_poser(ARRAY['ent_absp_piece', 'ent_absp_piece_lecture', 'ent_absp_message', 'ent_absp_gel']);
