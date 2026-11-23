-- Socle 21, suites V5 (passe 3), plage d'extension 700–719 : jeton de version des moyennes externes (D-069, M2).
--
-- Un compteur `version` (1 à la saisie, +1 à CHAQUE écriture effective) remplace le triplet (statut, valeur, barème) comme
-- jeton de compare-et-écrit de « publier » et de « retirer la publication » : une correction A → B → A, une correction de
-- la pièce ou de l'appréciation transcrite sont désormais détectées (le triplet, lui, revenait à l'identique).
-- Additive et rejouable ; aucune migration versée n'est modifiée (règle 25) ; posée APRÈS 690 (règle 38).
--
-- Le compteur ne se pose que dans le déclencheur : une valeur envoyée par un client est ignorée. Le déclencheur
-- `eva_moyenne_externe_garde` (477/478) renvoie NULL quand rien ne change : l'écriture n'a pas lieu et la version non plus.
-- Le nom `eva_moyenne_externe_version` passe APRÈS `eva_moyenne_externe_garde` dans l'ordre alphabétique des déclencheurs.

ALTER TABLE eva_moyenne_externe ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE eva_moyenne_externe DROP CONSTRAINT IF EXISTS eva_moyenne_externe_version_chk;
ALTER TABLE eva_moyenne_externe ADD CONSTRAINT eva_moyenne_externe_version_chk CHECK (version >= 1);

CREATE OR REPLACE FUNCTION ent_trg_eva_moyenne_externe_version() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp
AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        NEW.version := 1;
    ELSE
        NEW.version := OLD.version + 1;
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS eva_moyenne_externe_version ON eva_moyenne_externe;
CREATE TRIGGER eva_moyenne_externe_version BEFORE INSERT OR UPDATE ON eva_moyenne_externe
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_moyenne_externe_version();
