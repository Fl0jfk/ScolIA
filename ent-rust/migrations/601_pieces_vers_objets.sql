-- Sortie des fichiers de la base : pièces de messagerie et de PAI (PRA_STOCKAGE_OBJETS §5, E5, E8).
-- Étape 1 du plan : migration ADDITIVE et rejouable. Rien n'est supprimé ici.
--   * `storage_key` : clé d'objet opaque (`{etab}/messages|pai/{uuid}.{ext}`), jamais un nom de fichier ;
--   * `sha256`      : empreinte du contenu, calculée avant l'envoi et relue après (même valeur que `ent_objet`) ;
--   * `repris_le`   : date de la reprise d'une pièce historique (NULL = écrite directement dans le magasin).
-- La taille reste dans `taille_octets` (déjà présente, contrôlée par CHECK).
-- `contenu` (BYTEA) n'est PLUS ÉCRIT par l'application : il devient nullable, vide par défaut, et ne
-- garde d'octets que pour l'historique non encore repris. Sa suppression est l'étape 4 (migration
-- ultérieure, destructive, sur décision et après sauvegarde vérifiée).
-- Plage 601 : 600 est prise par `ent-securite-rls-stricte` ; aucune des deux ne modifie l'autre.

ALTER TABLE ent_msg_piece ADD COLUMN IF NOT EXISTS storage_key TEXT;
ALTER TABLE ent_msg_piece ADD COLUMN IF NOT EXISTS sha256 TEXT;
ALTER TABLE ent_msg_piece ADD COLUMN IF NOT EXISTS repris_le TIMESTAMPTZ;
ALTER TABLE ent_msg_piece ALTER COLUMN contenu DROP NOT NULL;
ALTER TABLE ent_msg_piece ALTER COLUMN contenu SET DEFAULT '\x'::bytea;

ALTER TABLE ent_pai_piece ADD COLUMN IF NOT EXISTS sha256 TEXT;
ALTER TABLE ent_pai_piece ADD COLUMN IF NOT EXISTS repris_le TIMESTAMPTZ;

-- Cohérence : une empreinte est un SHA-256 hexadécimal minuscule ; une clé d'objet commence par
-- l'établissement de la ligne (jamais l'objet d'un autre cloisonnement).
ALTER TABLE ent_msg_piece DROP CONSTRAINT IF EXISTS ent_msg_piece_objet_chk;
ALTER TABLE ent_msg_piece ADD CONSTRAINT ent_msg_piece_objet_chk CHECK (
    (sha256 IS NULL OR sha256 ~ '^[0-9a-f]{64}$')
    AND (storage_key IS NULL OR storage_key LIKE etablissement_id::text || '/%')
);
ALTER TABLE ent_pai_piece DROP CONSTRAINT IF EXISTS ent_pai_piece_objet_chk;
ALTER TABLE ent_pai_piece ADD CONSTRAINT ent_pai_piece_objet_chk CHECK (
    (sha256 IS NULL OR sha256 ~ '^[0-9a-f]{64}$')
    AND (storage_key IS NULL OR storage_key LIKE etablissement_id::text || '/%')
);

-- Candidates à la reprise : lignes dont le BYTEA est encore rempli (index partiel, petit).
CREATE INDEX IF NOT EXISTS ent_msg_piece_a_reprendre_idx
    ON ent_msg_piece (etablissement_id, id) WHERE contenu IS NOT NULL AND octet_length(contenu) > 0;
CREATE INDEX IF NOT EXISTS ent_pai_piece_a_reprendre_idx
    ON ent_pai_piece (etablissement_id, id) WHERE contenu IS NOT NULL AND octet_length(contenu) > 0;
