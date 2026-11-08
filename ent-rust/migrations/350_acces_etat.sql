-- Socle 6 (personnels), lot P0, migration 350 : état et fin d'un accès (`docs/socles/06_PERSONNELS.md`
-- §2.2 n° 1-2, §8.4, §17). Table du socle 1, pilotée par le socle 6.
--
-- Avant : `ent_membership` n'avait ni état ni date de fin. Un contractuel parti, un AED en fin de
-- contrat, un professeur muté gardaient leur accès indéfiniment (on pouvait révoquer leurs
-- sessions, ils se reconnectaient), et rien n'empêchait un établissement de perdre son dernier
-- administrateur.
--
-- Après :
--  * `etat` ('ouvert' | 'suspendu' | 'ferme') + trace (depuis quand, pourquoi, par qui) ;
--  * `date_fin` : l'accès se ferme le soir de cette date (jour de l'établissement, fuseau de la
--    migration 265) ; une connexion, une session ou un jeton après cette date sont refusés ;
--  * `ent_acces_ouvert(user, etab)` : définition unique de « cet accès permet d'agir aujourd'hui » ;
--  * garde-fou du dernier administrateur **en base** (déclencheur), quel que soit le chemin
--    (commande, SQL de maintenance, suppression en cascade).
--
-- Additive et rejouable. Tous les accès existants restent `ouvert`, sans date de fin.

ALTER TABLE ent_membership
    ADD COLUMN IF NOT EXISTS etat TEXT NOT NULL DEFAULT 'ouvert',
    ADD COLUMN IF NOT EXISTS etat_depuis TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS etat_motif TEXT,
    ADD COLUMN IF NOT EXISTS etat_par_user_id UUID,
    ADD COLUMN IF NOT EXISTS date_fin DATE;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint
                    WHERE conrelid = 'ent_membership'::regclass AND conname = 'ent_membership_etat_chk') THEN
        ALTER TABLE ent_membership
            ADD CONSTRAINT ent_membership_etat_chk CHECK (etat IN ('ouvert', 'suspendu', 'ferme'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint
                    WHERE conrelid = 'ent_membership'::regclass AND conname = 'ent_membership_motif_chk') THEN
        ALTER TABLE ent_membership
            ADD CONSTRAINT ent_membership_motif_chk CHECK (etat_motif IS NULL OR etat_motif IN
                ('depart', 'fin_contrat', 'mutation', 'conservatoire', 'conge', 'erreur', 'autre'));
    END IF;
END $$;

-- Index partiel : les accès non ouverts sont rares, la revue des accès les retrouve vite.
CREATE INDEX IF NOT EXISTS ent_membership_non_ouvert_idx
    ON ent_membership (etablissement_id, etat) WHERE etat <> 'ouvert';

-- Un accès permet d'agir si et seulement si : état ouvert ET date de fin non dépassée dans le
-- fuseau de l'établissement (`ent_aujourdhui_etab`, migration 265 : Europe/Paris par défaut).
CREATE OR REPLACE FUNCTION ent_acces_ouvert(p_user UUID, p_etab UUID)
RETURNS BOOLEAN
LANGUAGE sql STABLE
AS $$
    SELECT EXISTS (
        SELECT 1 FROM ent_membership m
         WHERE m.user_id = p_user AND m.etablissement_id = p_etab
           AND m.etat = 'ouvert'
           AND (m.date_fin IS NULL OR m.date_fin >= ent_aujourdhui_etab(p_etab)))
$$;

-- Garde-fou du dernier administrateur. Un administrateur « durable » est un accès de profil
-- `admin`, ouvert et sans date de fin. Toute écriture qui lui ferait perdre l'une de ces trois
-- qualités (fermeture, suspension, date de fin, changement de profil, suppression) est refusée si
-- aucun autre administrateur durable n'existe dans l'établissement. Le verrou consultatif
-- sérialise deux retraits simultanés (sans lui, deux transactions pourraient chacune voir l'autre
-- administrateur et retirer les deux). Purge volontaire d'un établissement entier :
-- `SET LOCAL ent.purge_etablissement = 'on'` (jamais posé par l'application).
CREATE OR REPLACE FUNCTION ent_membership_garde_dernier_admin()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    restant BIGINT;
BEGIN
    IF OLD.role IS DISTINCT FROM 'admin' OR OLD.etat <> 'ouvert' OR OLD.date_fin IS NOT NULL THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;
    IF TG_OP = 'UPDATE'
       AND NEW.user_id = OLD.user_id AND NEW.etablissement_id = OLD.etablissement_id
       AND NEW.role = 'admin' AND NEW.etat = 'ouvert' AND NEW.date_fin IS NULL THEN
        RETURN NEW;
    END IF;
    IF current_setting('ent.purge_etablissement', true) = 'on' THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;

    PERFORM pg_advisory_xact_lock(hashtextextended('ent_dernier_admin:' || OLD.etablissement_id::text, 0));
    SELECT count(*) INTO restant
      FROM ent_membership m
     WHERE m.etablissement_id = OLD.etablissement_id
       AND m.user_id <> OLD.user_id
       AND m.role = 'admin' AND m.etat = 'ouvert' AND m.date_fin IS NULL;
    IF restant = 0 THEN
        RAISE EXCEPTION 'dernier_administrateur' USING ERRCODE = '23514',
            HINT = 'Un établissement ne peut pas perdre son dernier administrateur.';
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END
$$;

DROP TRIGGER IF EXISTS ent_membership_dernier_admin ON ent_membership;
CREATE TRIGGER ent_membership_dernier_admin
    BEFORE UPDATE OR DELETE ON ent_membership
    FOR EACH ROW EXECUTE FUNCTION ent_membership_garde_dernier_admin();
