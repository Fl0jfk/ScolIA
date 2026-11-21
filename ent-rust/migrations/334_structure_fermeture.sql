-- Socle 5, lot S3 (`docs/socles/05_STRUCTURE.md` STR-25, STR-26, STR-94, R-S1, §14.3).
--
-- Fermeture exceptionnelle : effets DIFFÉRENCIÉS par service (cours, accueil des élèves,
-- restauration, internat, transports, administration), fermeture conditionnelle avec levée
-- programmée, historique en ajout seul de chaque changement. Additif et rejouable.

-- Contrôles propres à la fermeture, posés sur la table de 333.
ALTER TABLE ent_calendrier_evenement DROP CONSTRAINT IF EXISTS ent_calendrier_evenement_fermeture_chk;
ALTER TABLE ent_calendrier_evenement ADD CONSTRAINT ent_calendrier_evenement_fermeture_chk CHECK (
    services_touches <@ ARRAY['cours', 'accueil_eleves', 'restauration', 'internat', 'transports',
                              'administration']::TEXT[]
    AND cardinality(services_touches) <= 6
    AND (type = 'fermeture_exceptionnelle'
         OR (cardinality(services_touches) = 0 AND NOT conditionnelle
             AND levee_prevue_le IS NULL AND NOT internes_presents))
    AND (type <> 'fermeture_exceptionnelle'
         OR (effet IN ('sans_cours', 'accueil_seulement', 'cours_modifies')
             AND cardinality(services_touches) >= 1
             AND motif IS NOT NULL AND length(btrim(motif)) >= 1))
    AND (levee_prevue_le IS NULL OR conditionnelle));

-- Une fermeture ne se supprime jamais ; une fois levée, elle ne se rouvre pas (on en déclare
-- une nouvelle, ce qui produit un nouvel événement et un nouveau message).
CREATE OR REPLACE FUNCTION ent_trg_fermeture_garde() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF OLD.type = 'fermeture_exceptionnelle' THEN
            RAISE EXCEPTION 'fermeture_non_supprimable' USING ERRCODE = '23514';
        END IF;
        RETURN OLD;
    END IF;
    IF OLD.type = 'fermeture_exceptionnelle' THEN
        IF NEW.type <> OLD.type OR NEW.etablissement_id <> OLD.etablissement_id
           OR NEW.annee_id <> OLD.annee_id THEN
            RAISE EXCEPTION 'fermeture_identite_immuable' USING ERRCODE = '23514';
        END IF;
        IF OLD.annule_le IS NOT NULL AND (
               NEW.annule_le IS DISTINCT FROM OLD.annule_le
            OR NEW.du <> OLD.du OR NEW.au <> OLD.au
            OR NEW.services_touches IS DISTINCT FROM OLD.services_touches) THEN
            RAISE EXCEPTION 'fermeture_levee_non_rouvrable' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_calendrier_fermeture_garde ON ent_calendrier_evenement;
CREATE TRIGGER ent_calendrier_fermeture_garde BEFORE UPDATE OR DELETE ON ent_calendrier_evenement
    FOR EACH ROW EXECUTE FUNCTION ent_trg_fermeture_garde();

-- Historique des changements d'une fermeture, en ajout seul. Chaque ligne = un message au plus
-- (« un seul message par changement » : l'événement `referentiel.fermeture_*` publié dans la
-- même transaction porte l'identifiant de cette ligne).
CREATE TABLE IF NOT EXISTS ent_fermeture_historique (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    evenement_id UUID NOT NULL,
    changement TEXT NOT NULL CHECK (changement IN
        ('declaree', 'prolongee', 'raccourcie', 'services_modifies', 'levee', 'levee_programmee',
         'levee_programmee_modifiee')),
    origine TEXT NOT NULL CHECK (origine IN ('manuel', 'programme')),
    du DATE NOT NULL,
    au DATE NOT NULL,
    services_touches TEXT[] NOT NULL DEFAULT '{}',
    levee_prevue_le TIMESTAMPTZ,
    motif TEXT CHECK (motif IS NULL OR length(motif) <= 500),
    par UUID,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_fermeture_historique_evenement_fk FOREIGN KEY (evenement_id, etablissement_id)
        REFERENCES ent_calendrier_evenement (id, etablissement_id)
);
CREATE INDEX IF NOT EXISTS ent_fermeture_historique_evt_idx
    ON ent_fermeture_historique (etablissement_id, evenement_id, le);

CREATE OR REPLACE FUNCTION ent_fermeture_historique_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'ent_fermeture_historique est en ajout seul' USING ERRCODE = '42501';
END
$$;
DROP TRIGGER IF EXISTS ent_fermeture_historique_immuable ON ent_fermeture_historique;
CREATE TRIGGER ent_fermeture_historique_immuable
    BEFORE UPDATE OR DELETE ON ent_fermeture_historique
    FOR EACH ROW EXECUTE FUNCTION ent_fermeture_historique_ajout_seul();

-- RLS de l'historique (même politique que les autres tables du socle).
DO $$
BEGIN
    EXECUTE 'ALTER TABLE ent_fermeture_historique ENABLE ROW LEVEL SECURITY';
    EXECUTE 'ALTER TABLE ent_fermeture_historique FORCE ROW LEVEL SECURITY';
    EXECUTE 'DROP POLICY IF EXISTS ent_rls_etablissement ON ent_fermeture_historique';
    EXECUTE 'CREATE POLICY ent_rls_etablissement ON ent_fermeture_historique FOR ALL
        USING (etablissement_id = (SELECT ent_rls_etablissement())
            OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
        WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
            OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))';
END
$$;
