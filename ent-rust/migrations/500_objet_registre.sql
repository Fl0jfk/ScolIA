-- Registre des objets stockés hors base (PRA §18.4, E5/E6/E13 ; erreur F-013).
-- Chaque fichier écrit dans le magasin d'objets (S3 en production) y laisse UNE ligne :
-- clé opaque, taille et empreinte SHA-256 calculées avant l'envoi, type, dates.
-- Les objets sont IMMUABLES : la ligne ne change jamais, sauf `supprime_le` (suppression
-- logique datée) ; jamais de DELETE. Un document modifié = nouvel objet, nouvelle ligne.
-- Additif et rejouable (E8). Plage 500–519 (480–499 : messagerie, 520 et + : autres lots).

CREATE TABLE IF NOT EXISTS ent_objet (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    cle_objet TEXT NOT NULL,
    taille BIGINT NOT NULL,
    sha256 TEXT NOT NULL,
    mime TEXT,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    supprime_le TIMESTAMPTZ,
    CONSTRAINT ent_objet_cle_uq UNIQUE (etablissement_id, cle_objet),
    CONSTRAINT ent_objet_taille_chk CHECK (taille >= 0),
    CONSTRAINT ent_objet_sha256_chk CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    -- La clé commence par l'établissement : jamais l'objet d'un autre cloisonnement.
    CONSTRAINT ent_objet_cle_etab_chk CHECK (cle_objet LIKE etablissement_id::text || '/%')
);

CREATE INDEX IF NOT EXISTS ent_objet_cree_idx ON ent_objet (etablissement_id, cree_le);

-- Immuabilité : seule `supprime_le` peut passer de NULL à une date ; pas de DELETE.
CREATE OR REPLACE FUNCTION ent_objet_immuable() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_objet : suppression interdite (utiliser supprime_le)';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.cle_objet IS DISTINCT FROM OLD.cle_objet
       OR NEW.taille IS DISTINCT FROM OLD.taille
       OR NEW.sha256 IS DISTINCT FROM OLD.sha256
       OR NEW.mime IS DISTINCT FROM OLD.mime
       OR NEW.cree_le IS DISTINCT FROM OLD.cree_le
       OR (OLD.supprime_le IS NOT NULL AND NEW.supprime_le IS DISTINCT FROM OLD.supprime_le) THEN
        RAISE EXCEPTION 'ent_objet : objet immuable';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_objet_immuable_trg ON ent_objet;
CREATE TRIGGER ent_objet_immuable_trg
    BEFORE UPDATE OR DELETE ON ent_objet
    FOR EACH ROW EXECUTE FUNCTION ent_objet_immuable();

-- RLS (politique standard des migrations 100 et 215, E13).
ALTER TABLE ent_objet ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_objet FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_objet;
CREATE POLICY ent_rls_etablissement ON ent_objet FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));
