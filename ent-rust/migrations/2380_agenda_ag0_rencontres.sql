-- 2380 — Socle 19, lot AG0 : correctifs urgents des rencontres parents-professeurs (192).
-- Réf. : 19_AGENDA_RDV.md §27 (RD-01, RD-02, RD-05, RD-10, RD-11), E1–E14 (PRA §18.4), D-036.
--
-- * RD-01 : `ent_rdv_fait`, fil en AJOUT SEUL (qui, quand, quoi) : une annulation libère le
--   créneau mais ne fait plus disparaître qui l'avait réservé, pour quel enfant, ni quand.
-- * RD-10 : un créneau peut être « bloqué » par son enseignant ; présence « honoré / non venu ».
-- * RD-11 : clés étrangères composites (établissement, …), `ON DELETE RESTRICT` : supprimer une
--   rencontre qui a des créneaux est refusé (le CASCADE d'origine effaçait l'historique).
-- * Aucune donnée supprimée ; aucune colonne JSON. Rejouable (contraintes posées par nom).
--   `ent_d036_contrainte` / `ent_d036_retirer_fk_simple` viennent de la migration 661.

-- Cibles des références composites.
SELECT ent_d036_contrainte('ent_rencontre', 'ent_rencontre_etab_id_uq', 'UNIQUE (etablissement_id, id)');
SELECT ent_d036_contrainte('ent_creneau_rdv', 'ent_creneau_rdv_etab_id_uq', 'UNIQUE (etablissement_id, id)');
SELECT ent_d036_contrainte('ent_classe', 'ent_classe_etab_id_uq', 'UNIQUE (etablissement_id, id)');

-- RD-11 : références composites, sans cascade.
SELECT ent_d036_retirer_fk_simple('ent_creneau_rdv', 'rencontre_id');
SELECT ent_d036_contrainte('ent_creneau_rdv', 'ent_creneau_rdv_rencontre_fk',
    'FOREIGN KEY (etablissement_id, rencontre_id) REFERENCES ent_rencontre (etablissement_id, id) ON DELETE RESTRICT');
SELECT ent_d036_contrainte('ent_creneau_rdv', 'ent_creneau_rdv_eleve_fk',
    'FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id)');
SELECT ent_d036_retirer_fk_simple('ent_rencontre', 'classe_id');
SELECT ent_d036_contrainte('ent_rencontre', 'ent_rencontre_classe_fk',
    'FOREIGN KEY (etablissement_id, classe_id) REFERENCES ent_classe (etablissement_id, id)');

-- RD-10 : créneau bloqué par l'enseignant ; présence saisie après le rendez-vous.
ALTER TABLE ent_creneau_rdv ADD COLUMN IF NOT EXISTS presence TEXT;
ALTER TABLE ent_creneau_rdv ADD COLUMN IF NOT EXISTS presence_le TIMESTAMPTZ;
ALTER TABLE ent_creneau_rdv DROP CONSTRAINT IF EXISTS ent_creneau_rdv_statut_chk;
ALTER TABLE ent_creneau_rdv ADD CONSTRAINT ent_creneau_rdv_statut_chk
    CHECK (statut IN ('libre', 'reserve', 'bloque'));
ALTER TABLE ent_creneau_rdv DROP CONSTRAINT IF EXISTS ent_creneau_rdv_reserve_chk;
ALTER TABLE ent_creneau_rdv ADD CONSTRAINT ent_creneau_rdv_reserve_chk CHECK (
    (statut IN ('libre', 'bloque') AND reserve_par_user_id IS NULL AND eleve_id IS NULL)
    OR (statut = 'reserve' AND reserve_par_user_id IS NOT NULL AND eleve_id IS NOT NULL));
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_creneau_rdv_presence_chk') THEN
        ALTER TABLE ent_creneau_rdv ADD CONSTRAINT ent_creneau_rdv_presence_chk CHECK (
            presence IS NULL OR (statut = 'reserve' AND presence IN ('honore', 'non_venu')));
    END IF;
END $$;

-- Fonction commune des tables en ajout seul du socle 19. Seule exception : l'anonymisation d'un
-- élève supprimé (la clé étrangère remet `eleve_id` à NULL, le reste de la ligne ne bouge pas).
CREATE OR REPLACE FUNCTION ent_agenda_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND TG_TABLE_NAME = 'ent_rdv_fait'
       AND OLD.eleve_id IS NOT NULL AND NEW.eleve_id IS NULL
       AND NEW.id = OLD.id AND NEW.creneau_id = OLD.creneau_id AND NEW.fait = OLD.fait
       AND NEW.le = OLD.le AND NEW.par_user_id IS NOT DISTINCT FROM OLD.par_user_id THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION '% est en ajout seul', TG_TABLE_NAME USING ERRCODE = '42501';
END
$$;

-- RD-01 : fil des faits d'un créneau de rendez-vous.
CREATE TABLE IF NOT EXISTS ent_rdv_fait (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    creneau_id UUID NOT NULL,
    rencontre_id UUID NOT NULL,
    enseignant_user_id UUID NOT NULL,
    -- Enfant et responsable concernés AU MOMENT du fait (le créneau, lui, est remis à zéro).
    eleve_id UUID,
    reserve_par_user_id UUID,
    fait TEXT NOT NULL CHECK (fait IN (
        'reserve', 'annule_famille', 'annule_organisateur', 'rappel',
        'honore', 'non_venu', 'bloque', 'debloque', 'a_revoir')),
    par_user_id UUID,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Motif montré à la famille (obligatoire pour une annulation par l'organisateur).
    motif_public TEXT CHECK (motif_public IS NULL OR length(motif_public) <= 300),
    detail TEXT CHECK (detail IS NULL OR length(detail) <= 500),
    CONSTRAINT ent_rdv_fait_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_rdv_fait_creneau_fk FOREIGN KEY (etablissement_id, creneau_id)
        REFERENCES ent_creneau_rdv (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_rdv_fait_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id) ON DELETE SET NULL (eleve_id),
    CONSTRAINT ent_rdv_fait_motif_chk CHECK (
        fait <> 'annule_organisateur' OR length(btrim(coalesce(motif_public, ''))) > 0)
);
CREATE INDEX IF NOT EXISTS ent_rdv_fait_creneau_idx
    ON ent_rdv_fait (etablissement_id, creneau_id, le);
DROP TRIGGER IF EXISTS ent_rdv_fait_immuable ON ent_rdv_fait;
CREATE TRIGGER ent_rdv_fait_immuable
    BEFORE UPDATE OR DELETE ON ent_rdv_fait
    FOR EACH ROW EXECUTE FUNCTION ent_agenda_ajout_seul();

SELECT ent_rls_poser(ARRAY['ent_rencontre', 'ent_creneau_rdv', 'ent_rdv_fait']);
