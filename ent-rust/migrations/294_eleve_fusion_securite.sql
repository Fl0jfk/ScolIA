-- Socle 2 (l'élève), lot E6 : réservée à la sécurité de la fusion.
--
-- (Les tables des migrations 290 à 292 portent déjà `etablissement_id` et leur politique
-- d'isolation par établissement en mode permissif, comme les tables des lots E2 à E5 ; le mode
-- strict suit le calendrier de `docs/RLS_ISOLATION.md`.)
--
-- Garde-fou de dernier recours : un élève ne peut pas se désigner lui-même comme absorbé et une
-- chaîne d'absorption ne peut pas former de boucle (A absorbé dans B alors que B est absorbé dans A).
CREATE OR REPLACE FUNCTION ent_eleve_absorption_sans_boucle() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
DECLARE
    cur UUID := NEW.fusionne_dans;
    n INT := 0;
BEGIN
    WHILE cur IS NOT NULL AND n < 1000 LOOP
        IF cur = NEW.id THEN
            RAISE EXCEPTION 'fusion : une absorption ne peut pas former de boucle' USING ERRCODE = '23514';
        END IF;
        SELECT fusionne_dans INTO cur FROM ent_eleve WHERE etablissement_id = NEW.etablissement_id AND id = cur;
        n := n + 1;
    END LOOP;
    RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS ent_eleve_absorption_sans_boucle ON ent_eleve;
CREATE TRIGGER ent_eleve_absorption_sans_boucle
    BEFORE INSERT OR UPDATE OF fusionne_dans ON ent_eleve
    FOR EACH ROW WHEN (NEW.fusionne_dans IS NOT NULL)
    EXECUTE FUNCTION ent_eleve_absorption_sans_boucle();

-- L'élève conservé existe, dans le même établissement (clé composée, §10.1).
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_eleve_fusionne_dans_fk') THEN
        ALTER TABLE ent_eleve ADD CONSTRAINT ent_eleve_fusionne_dans_fk
            FOREIGN KEY (etablissement_id, fusionne_dans) REFERENCES ent_eleve (etablissement_id, id);
    END IF;
END $$;
