-- Messagerie, lot M5 (`docs/socles/16_MESSAGERIE.md` §12, R-M31 à R-M33 ; CM-57 à CM-60, D16) : pièces jointes
-- servies depuis le stockage objet **après** quarantaine et analyse, carte « document » du socle 7, journal des
-- téléchargements. Plage 480–499 (repli de la messagerie) : 488 à 493 sont réservées à M5 et M6. Rejouable.
--
-- Principes (reprise du modèle de D5, `3704_documents_pieces_reelles`) :
--   * `storage_key` / `sha256` = l'**original** reçu : scellé, jamais servi ;
--   * `cle_objet_servi` / `sha256_servi` = le fichier **servi** (PDF désarmé, JPEG sans EXIF…), écrit une seule
--     fois, uniquement à la sortie de quarantaine (CHECK + déclencheur) ;
--   * une pièce créée avant M5 (ou reprise du `BYTEA`) naît « en quarantaine » : elle n'est servie qu'après
--     analyse a posteriori (§26 : « état saine après analyse »), jamais en silence ;
--   * plus de plafond de 256 Kio (D16) : 20 Mio par pièce, comme la conception (§12 point 3).

-- 1. Taille : 20 Mio (le plafond de 256 Kio tenait au stockage en base) -------------------------------------------
ALTER TABLE ent_msg_piece DROP CONSTRAINT IF EXISTS ent_msg_piece_taille_chk;
ALTER TABLE ent_msg_piece ADD CONSTRAINT ent_msg_piece_taille_chk
    CHECK (taille_octets > 0 AND taille_octets <= 20971520);

-- 2. Quarantaine, analyse, fichier servi -------------------------------------------------------------------------
ALTER TABLE ent_msg_piece ADD COLUMN IF NOT EXISTS etat TEXT NOT NULL DEFAULT 'en_quarantaine';
ALTER TABLE ent_msg_piece ADD COLUMN IF NOT EXISTS cle_objet_servi TEXT;
ALTER TABLE ent_msg_piece ADD COLUMN IF NOT EXISTS sha256_servi TEXT;
ALTER TABLE ent_msg_piece ADD COLUMN IF NOT EXISTS mime_servi TEXT;
ALTER TABLE ent_msg_piece ADD COLUMN IF NOT EXISTS taille_servie INTEGER;
-- Motif court et sans donnée de la pièce (jamais son nom) : « antivirus indisponible », « menace : … », forme.
ALTER TABLE ent_msg_piece ADD COLUMN IF NOT EXISTS analyse_motif TEXT;
ALTER TABLE ent_msg_piece ADD COLUMN IF NOT EXISTS analyse_tentatives INTEGER NOT NULL DEFAULT 0;
ALTER TABLE ent_msg_piece ADD COLUMN IF NOT EXISTS derniere_analyse_le TIMESTAMPTZ;
ALTER TABLE ent_msg_piece ADD COLUMN IF NOT EXISTS analysee_le TIMESTAMPTZ;

ALTER TABLE ent_msg_piece DROP CONSTRAINT IF EXISTS ent_msg_piece_etat_chk;
ALTER TABLE ent_msg_piece ADD CONSTRAINT ent_msg_piece_etat_chk
    CHECK (etat IN ('en_quarantaine', 'saine', 'infectee', 'rejetee'));

-- Un fichier servi existe si et seulement si la pièce est saine ; sa clé est celle de l'établissement de la ligne
-- (jamais l'objet d'un autre cloisonnement) ; empreinte SHA-256 hexadécimale.
ALTER TABLE ent_msg_piece DROP CONSTRAINT IF EXISTS ent_msg_piece_servi_chk;
ALTER TABLE ent_msg_piece ADD CONSTRAINT ent_msg_piece_servi_chk CHECK (
    (etat = 'saine') = (cle_objet_servi IS NOT NULL)
    AND (etat = 'saine') = (sha256_servi IS NOT NULL)
    AND (etat = 'saine') = (mime_servi IS NOT NULL)
    AND (etat = 'saine') = (taille_servie IS NOT NULL)
    AND (cle_objet_servi IS NULL OR cle_objet_servi LIKE etablissement_id::text || '/%')
    AND (sha256_servi IS NULL OR sha256_servi ~ '^[0-9a-f]{64}$')
    AND (taille_servie IS NULL OR taille_servie > 0)
    AND (analyse_motif IS NULL OR char_length(analyse_motif) <= 200)
);

-- Le contrôle de transition : l'original et le fichier servi ne se réécrivent pas, une pièce analysée ne change
-- plus d'état (saine, infectée ou rejetée : terminal), l'identité de la ligne est immuable. `fil_id` bouge
-- (fusion et scission de fils, M4) ; `storage_key` se pose une seule fois (reprise du `BYTEA`, 601).
CREATE OR REPLACE FUNCTION ent_msg_piece_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.message_id IS DISTINCT FROM OLD.message_id
       OR NEW.created_by_user_id IS DISTINCT FROM OLD.created_by_user_id
       OR NEW.mime_type IS DISTINCT FROM OLD.mime_type THEN
        RAISE EXCEPTION 'ent_msg_piece : identité de la pièce immuable' USING ERRCODE = 'check_violation';
    END IF;
    IF OLD.storage_key IS NOT NULL AND NEW.storage_key IS DISTINCT FROM OLD.storage_key THEN
        RAISE EXCEPTION 'ent_msg_piece : original scellé' USING ERRCODE = 'check_violation';
    END IF;
    IF OLD.sha256 IS NOT NULL AND NEW.sha256 IS DISTINCT FROM OLD.sha256 THEN
        RAISE EXCEPTION 'ent_msg_piece : empreinte de l''original scellée' USING ERRCODE = 'check_violation';
    END IF;
    IF OLD.cle_objet_servi IS NOT NULL AND (NEW.cle_objet_servi IS DISTINCT FROM OLD.cle_objet_servi
                                            OR NEW.sha256_servi IS DISTINCT FROM OLD.sha256_servi) THEN
        RAISE EXCEPTION 'ent_msg_piece : fichier servi écrit une seule fois' USING ERRCODE = 'check_violation';
    END IF;
    IF OLD.etat <> 'en_quarantaine' AND NEW.etat IS DISTINCT FROM OLD.etat THEN
        RAISE EXCEPTION 'ent_msg_piece : une pièce analysée ne change plus d''état' USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_msg_piece_garde_trg ON ent_msg_piece;
CREATE TRIGGER ent_msg_piece_garde_trg BEFORE UPDATE ON ent_msg_piece
    FOR EACH ROW EXECUTE FUNCTION ent_msg_piece_garde();

-- Pièces à analyser : petit index partiel (la tâche de fond ne lit que la quarantaine).
CREATE INDEX IF NOT EXISTS ent_msg_piece_quarantaine_idx
    ON ent_msg_piece (etablissement_id, created_at, id) WHERE etat = 'en_quarantaine';

-- 3. Journal des téléchargements (« journalise chaque téléchargement de pièce ») ------------------------------------
-- Une ligne par lien émis ou téléchargement direct. Ajout seul. Ni nom de fichier, ni clé d'objet, ni contenu :
-- l'identifiant de la pièce, le compte et la voie. L'élève sujet n'est pas copié (le fil le porte).
CREATE TABLE IF NOT EXISTS ent_msg_piece_acces (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    piece_id UUID NOT NULL,
    fil_id UUID NOT NULL,
    user_id UUID NOT NULL,
    via TEXT NOT NULL CHECK (via IN ('lien', 'telechargement')),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_msg_piece_acces_piece_fk FOREIGN KEY (piece_id) REFERENCES ent_msg_piece (id)
);
CREATE INDEX IF NOT EXISTS ent_msg_piece_acces_piece_idx ON ent_msg_piece_acces (etablissement_id, piece_id, le);

CREATE OR REPLACE FUNCTION ent_msg_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '% : ajout seul (ni modification ni suppression)', TG_TABLE_NAME
        USING ERRCODE = 'check_violation';
END
$$;
DROP TRIGGER IF EXISTS ent_msg_piece_acces_ajout_seul_trg ON ent_msg_piece_acces;
CREATE TRIGGER ent_msg_piece_acces_ajout_seul_trg BEFORE UPDATE OR DELETE ON ent_msg_piece_acces
    FOR EACH ROW EXECUTE FUNCTION ent_msg_ajout_seul();

-- 4. Carte « document » (R-M33) : un document du coffre (socle 7) se joint par carte, jamais par fichier -------------
-- La carte ne porte ni titre, ni contenu, ni nature : seulement le renvoi. Le droit **du lecteur** est relu à
-- chaque lecture auprès du socle 7 (un lecteur qui n'a pas le droit voit « document non disponible »).
CREATE TABLE IF NOT EXISTS ent_msg_carte_document (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    fil_id UUID NOT NULL,
    message_id UUID NOT NULL,
    document_id UUID NOT NULL,
    ajoutee_par_user_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_msg_carte_document_msg_uq UNIQUE (message_id, document_id),
    CONSTRAINT ent_msg_carte_document_msg_fk FOREIGN KEY (etablissement_id, message_id)
        REFERENCES ent_msg (etablissement_id, id),
    CONSTRAINT ent_msg_carte_document_fil_fk FOREIGN KEY (etablissement_id, fil_id)
        REFERENCES ent_msg_fil (etablissement_id, id),
    CONSTRAINT ent_msg_carte_document_doc_fk FOREIGN KEY (etablissement_id, document_id)
        REFERENCES ent_doc_document (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_msg_carte_document_fil_idx ON ent_msg_carte_document (etablissement_id, fil_id);
DROP TRIGGER IF EXISTS ent_msg_carte_document_ajout_seul_trg ON ent_msg_carte_document;
CREATE TRIGGER ent_msg_carte_document_ajout_seul_trg BEFORE UPDATE OR DELETE ON ent_msg_carte_document
    FOR EACH ROW EXECUTE FUNCTION ent_msg_ajout_seul();

-- 5. RLS permissive des nouvelles tables (même politique que 480, 482 et 485) --------------------------------------
DO $$
DECLARE
    t text;
    tables text[] := ARRAY['ent_msg_piece_acces', 'ent_msg_carte_document'];
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
