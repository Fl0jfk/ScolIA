-- 443 — Socle 10 (emploi du temps), lot T2 : séance enrichie (docs/socles/10_EMPLOI_DU_TEMPS.md §8, §21.1).
--
-- `ent_edt_seance` (110) est LA séance datée. Elle gagne : le cours (socle 5) et le placement et la
-- version d'où elle vient (identité stable : une séance reste la même quand une nouvelle version
-- reprend le même placement), le code de créneau de grille, les instants UTC calculés dans le fuseau de
-- l'établissement, l'état du socle 10 (huit états), un motif PUBLIC (jamais le motif interne, qui reste
-- dans l'exception ou la modification), l'origine et les traces de calcul.
-- Les colonnes historiques (classe texte, prof_user_id, salle, statut…) sont conservées comme
-- projections jusqu'au lot T11 : les lecteurs actuels (appel, cahier, notes) ne changent pas.
-- `etat` et `origine` sont DÉRIVÉS par déclencheur de `statut` et `source` (les écrivains historiques
-- n'ont pas à changer) ; les états `deplacee`, `neutralisee` et `non_assuree` sont posés par les
-- lots T3 et T4 (modifications, événements des autres socles).
-- Additive, rejouable (E8). Aucune donnée n'est supprimée.

ALTER TABLE ent_edt_seance ADD COLUMN IF NOT EXISTS cours_id UUID;
ALTER TABLE ent_edt_seance ADD COLUMN IF NOT EXISTS placement_id UUID;
ALTER TABLE ent_edt_seance ADD COLUMN IF NOT EXISTS version_id UUID;
ALTER TABLE ent_edt_seance ADD COLUMN IF NOT EXISTS creneau_code TEXT;
ALTER TABLE ent_edt_seance ADD COLUMN IF NOT EXISTS debut_utc TIMESTAMPTZ;
ALTER TABLE ent_edt_seance ADD COLUMN IF NOT EXISTS fin_utc TIMESTAMPTZ;
ALTER TABLE ent_edt_seance ADD COLUMN IF NOT EXISTS etat TEXT NOT NULL DEFAULT 'prevue';
ALTER TABLE ent_edt_seance ADD COLUMN IF NOT EXISTS motif_public_code TEXT;
ALTER TABLE ent_edt_seance ADD COLUMN IF NOT EXISTS motif_public TEXT;
ALTER TABLE ent_edt_seance ADD COLUMN IF NOT EXISTS origine TEXT NOT NULL DEFAULT 'plan';
ALTER TABLE ent_edt_seance ADD COLUMN IF NOT EXISTS remplacee_par_id UUID;
ALTER TABLE ent_edt_seance ADD COLUMN IF NOT EXISTS calcule_le TIMESTAMPTZ;
ALTER TABLE ent_edt_seance ADD COLUMN IF NOT EXISTS version_calcul INTEGER NOT NULL DEFAULT 1;

CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_seance_id_etab_uq ON ent_edt_seance (id, etablissement_id);
-- Identité stable : une séance par placement et par jour.
CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_seance_placement_jour_uq
    ON ent_edt_seance (etablissement_id, placement_id, jour) WHERE placement_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_edt_seance_cours_idx ON ent_edt_seance (etablissement_id, cours_id, jour) WHERE cours_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_edt_seance_debut_idx ON ent_edt_seance (etablissement_id, debut_utc) WHERE debut_utc IS NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_seance_etat_chk') THEN
        ALTER TABLE ent_edt_seance ADD CONSTRAINT ent_edt_seance_etat_chk
            CHECK (etat IN ('prevue', 'modifiee', 'annulee', 'deplacee', 'neutralisee', 'non_assuree', 'retiree', 'ajoutee'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_seance_origine_chk') THEN
        ALTER TABLE ent_edt_seance ADD CONSTRAINT ent_edt_seance_origine_chk
            CHECK (origine IN ('plan', 'ajout', 'deplacement', 'fusion', 'import'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_seance_motif_public_chk') THEN
        ALTER TABLE ent_edt_seance ADD CONSTRAINT ent_edt_seance_motif_public_chk
            CHECK (motif_public IS NULL OR char_length(motif_public) <= 200);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_seance_cours_fk') THEN
        ALTER TABLE ent_edt_seance ADD CONSTRAINT ent_edt_seance_cours_fk
            FOREIGN KEY (cours_id, etablissement_id) REFERENCES ent_cours (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_seance_version_fk') THEN
        ALTER TABLE ent_edt_seance ADD CONSTRAINT ent_edt_seance_version_fk
            FOREIGN KEY (version_id, etablissement_id) REFERENCES ent_edt_version (id, etablissement_id);
    END IF;
    -- Un placement retiré d'un brouillon ne peut pas avoir de séance ; la colonne seule est remise à NULL.
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_seance_placement_fk') THEN
        ALTER TABLE ent_edt_seance ADD CONSTRAINT ent_edt_seance_placement_fk
            FOREIGN KEY (placement_id, etablissement_id) REFERENCES ent_edt_placement (id, etablissement_id)
            ON DELETE SET NULL (placement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_seance_remplacee_par_fk') THEN
        ALTER TABLE ent_edt_seance ADD CONSTRAINT ent_edt_seance_remplacee_par_fk
            FOREIGN KEY (remplacee_par_id) REFERENCES ent_edt_seance (id) ON DELETE SET NULL;
    END IF;
END
$$;

-- Dérivation de l'état, de l'origine et des instants (fuseau de l'établissement, Europe/Paris par
-- défaut) à chaque écriture : tous les écrivains, historiques compris, restent cohérents.
CREATE OR REPLACE FUNCTION ent_edt_seance_derive() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    fuseau TEXT;
BEGIN
    NEW.etat := CASE
        WHEN NEW.statut = 'retiree' THEN 'retiree'
        WHEN NEW.statut = 'annulee' THEN 'annulee'
        WHEN NEW.statut = 'remplacee' THEN 'modifiee'
        WHEN NEW.source IN ('deplacement', 'manuel') THEN 'ajoutee'
        ELSE 'prevue'
    END;
    NEW.origine := CASE NEW.source
        WHEN 'deplacement' THEN 'deplacement'
        WHEN 'manuel' THEN 'ajout'
        ELSE 'plan'
    END;
    IF TG_OP = 'INSERT' OR NEW.jour IS DISTINCT FROM OLD.jour
       OR NEW.heure_debut IS DISTINCT FROM OLD.heure_debut OR NEW.heure_fin IS DISTINCT FROM OLD.heure_fin
       OR NEW.debut_utc IS NULL THEN
        SELECT p.fuseau INTO fuseau FROM ent_etablissement_parametre p
         WHERE p.etablissement_id = NEW.etablissement_id;
        fuseau := COALESCE(fuseau, 'Europe/Paris');
        NEW.debut_utc := (NEW.jour + NEW.heure_debut) AT TIME ZONE fuseau;
        NEW.fin_utc := (NEW.jour + NEW.heure_fin) AT TIME ZONE fuseau;
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS ent_edt_seance_derive_tg ON ent_edt_seance;
CREATE TRIGGER ent_edt_seance_derive_tg BEFORE INSERT OR UPDATE ON ent_edt_seance
    FOR EACH ROW EXECUTE FUNCTION ent_edt_seance_derive();

-- Reprise des séances existantes : le cours vient du créneau ; état, origine et instants sont dérivés
-- en réécrivant les lignes concernées (le déclencheur s'applique). Rejouable : ne touche que ce qui manque.
UPDATE ent_edt_seance s SET cours_id = c.cours_id
  FROM ent_edt_creneau c
 WHERE c.id = s.creneau_id AND c.etablissement_id = s.etablissement_id
   AND s.cours_id IS NULL AND c.cours_id IS NOT NULL;
UPDATE ent_edt_seance SET statut = statut WHERE debut_utc IS NULL;
