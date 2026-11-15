-- Socle 5, lot S6 : groupes et compositions (05_STRUCTURE.md §9.6, M8, STR-50 à 55, 60 à 68, R-S5, R-S6).
--
-- Additif et rejouable ; aucune colonne à structure libre, aucune date UTC (le jour est celui de l'ÉTABLISSEMENT :
-- `ent_aujourdhui_etab`). Appliquée APRÈS 343 (ordre de `migrate.rs`) : elle redéfinit la liste des objets
-- du journal de structure, que 341 repose (NOT VALID) ; la dernière redéfinition fait foi.
--
-- * `ent_groupe` : nature (enseignement, dispositif, activité, liste), capacité (bloquante ou non),
--   ouverture et fermeture datées, droits (`ouvre_droits` : une « liste » n'en donne aucun) ;
-- * `ent_groupe_classe` : 0..n classes d'origine d'un groupe (groupe inter-classes) ; `ent_groupe.classe_id`
--   n'est plus qu'une PROJECTION tenue par la base (la classe unique, sinon NULL) ;
-- * `ent_affectation` : motif et décideur ; une affectation erronée est ANNULÉE (déplacée dans l'archive
--   `ent_affectation_supprimee` avec son motif, jamais effacée sans trace) ; une même personne n'est pas
--   deux fois dans le même groupe aux mêmes dates ; une seule classe à une date ; un groupe ne reçoit
--   d'élèves que pendant qu'il est ouvert ;
-- * fonctions SQL de lecture (membres d'un groupe à une date) : la seule implémentation de la composition.
-- Les affectations restent la source de vérité de l'appartenance (aucun lecteur existant n'est modifié).

-- ───────────────────────── Groupe : nature, capacité, dates, droits ─────────────────────────
ALTER TABLE ent_groupe ADD COLUMN IF NOT EXISTS nature TEXT;
ALTER TABLE ent_groupe ADD COLUMN IF NOT EXISTS nature_a_verifier BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_groupe ADD COLUMN IF NOT EXISTS capacite INTEGER;
ALTER TABLE ent_groupe ADD COLUMN IF NOT EXISTS capacite_bloquante BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_groupe ADD COLUMN IF NOT EXISTS ouvre_droits BOOLEAN;
ALTER TABLE ent_groupe ADD COLUMN IF NOT EXISTS ouvert_le DATE;
ALTER TABLE ent_groupe ADD COLUMN IF NOT EXISTS ferme_le DATE;
ALTER TABLE ent_groupe ADD COLUMN IF NOT EXISTS modifie_le TIMESTAMPTZ NOT NULL DEFAULT now();

-- Reprise (M8), une seule fois (les lignes sans nature) : `enseignement` si un service, un cours ou une
-- offre vise le groupe ; `activite` s'il a des responsables sans service ; sinon `liste`, à revoir.
UPDATE ent_groupe g
   SET nature = x.n, nature_a_verifier = (x.n = 'liste')
  FROM (SELECT g2.id,
               CASE
                 WHEN EXISTS (SELECT 1 FROM ent_service_enseignement s
                               WHERE s.etablissement_id = g2.etablissement_id AND s.groupe_id = g2.id)
                   OR EXISTS (SELECT 1 FROM ent_offre_groupe og
                               WHERE og.etablissement_id = g2.etablissement_id AND og.groupe_id = g2.id)
                   OR EXISTS (SELECT 1 FROM ent_cours c
                               WHERE c.etablissement_id = g2.etablissement_id AND c.groupe_id = g2.id)
                   THEN 'enseignement'
                 WHEN EXISTS (SELECT 1 FROM ent_groupe_responsable r
                               WHERE r.etablissement_id = g2.etablissement_id AND r.groupe_id = g2.id)
                   THEN 'activite'
                 ELSE 'liste'
               END AS n
          FROM ent_groupe g2 WHERE g2.nature IS NULL) x
 WHERE g.id = x.id;

UPDATE ent_groupe SET ouvre_droits = (nature <> 'liste') WHERE ouvre_droits IS NULL;
UPDATE ent_groupe g
   SET ouvert_le = y.date_debut
  FROM ent_annee_scolaire y
 WHERE g.ouvert_le IS NULL AND y.id = g.annee_id AND y.etablissement_id = g.etablissement_id;

ALTER TABLE ent_groupe ALTER COLUMN nature SET DEFAULT 'enseignement';
ALTER TABLE ent_groupe ALTER COLUMN nature SET NOT NULL;
ALTER TABLE ent_groupe ALTER COLUMN ouvre_droits SET DEFAULT true;
ALTER TABLE ent_groupe ALTER COLUMN ouvre_droits SET NOT NULL;
ALTER TABLE ent_groupe ALTER COLUMN ouvert_le SET NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_groupe_nature_chk') THEN
        ALTER TABLE ent_groupe ADD CONSTRAINT ent_groupe_nature_chk
            CHECK (nature IN ('enseignement', 'dispositif', 'activite', 'liste'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_groupe_liste_sans_droits_chk') THEN
        ALTER TABLE ent_groupe ADD CONSTRAINT ent_groupe_liste_sans_droits_chk
            CHECK (nature <> 'liste' OR NOT ouvre_droits);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_groupe_capacite_chk') THEN
        ALTER TABLE ent_groupe ADD CONSTRAINT ent_groupe_capacite_chk
            CHECK (capacite IS NULL OR capacite > 0);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_groupe_dates_chk') THEN
        ALTER TABLE ent_groupe ADD CONSTRAINT ent_groupe_dates_chk
            CHECK (ferme_le IS NULL OR ferme_le >= ouvert_le);
    END IF;
END $$;

-- Garde-fous du groupe : valeurs par défaut des écrivains historiques, dates dans l'année, public et
-- année immuables, fermeture définitive, année clôturée = gel.
CREATE OR REPLACE FUNCTION ent_trg_groupe() RETURNS trigger AS $$
DECLARE
    a_debut DATE; a_fin DATE; a_close TIMESTAMPTZ;
BEGIN
    SELECT y.date_debut, y.date_fin, y.cloturee_le INTO a_debut, a_fin, a_close
      FROM ent_annee_scolaire y WHERE y.id = NEW.annee_id AND y.etablissement_id = NEW.etablissement_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'groupe_incoherent: annee' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'INSERT' THEN
        IF NEW.nature IS NULL THEN NEW.nature := 'enseignement'; END IF;
        IF NEW.ouvre_droits IS NULL THEN NEW.ouvre_droits := (NEW.nature <> 'liste'); END IF;
        IF NEW.ouvert_le IS NULL THEN NEW.ouvert_le := a_debut; END IF;
        IF a_close IS NOT NULL THEN
            RAISE EXCEPTION 'groupe_annee_cloturee' USING ERRCODE = '23514';
        END IF;
        IF NEW.ouvert_le < a_debut OR NEW.ouvert_le > a_fin THEN
            RAISE EXCEPTION 'groupe_incoherent: ouverture hors annee' USING ERRCODE = '23514';
        END IF;
    ELSE
        IF (NEW.etablissement_id, NEW.annee_id, NEW.ouvert_le)
               IS DISTINCT FROM (OLD.etablissement_id, OLD.annee_id, OLD.ouvert_le) THEN
            RAISE EXCEPTION 'groupe_annee_immuable' USING ERRCODE = '23514';
        END IF;
        IF OLD.ferme_le IS NOT NULL AND NEW.ferme_le IS DISTINCT FROM OLD.ferme_le THEN
            RAISE EXCEPTION 'groupe_ferme_definitif' USING ERRCODE = '23514';
        END IF;
        IF a_close IS NOT NULL
           AND (NEW.nature, NEW.libelle, NEW.capacite, NEW.capacite_bloquante, NEW.ouvre_droits, NEW.ferme_le, NEW.code)
               IS DISTINCT FROM (OLD.nature, OLD.libelle, OLD.capacite, OLD.capacite_bloquante, OLD.ouvre_droits, OLD.ferme_le, OLD.code) THEN
            RAISE EXCEPTION 'groupe_annee_cloturee' USING ERRCODE = '23514';
        END IF;
        NEW.modifie_le := now();
    END IF;
    IF NEW.ferme_le IS NOT NULL AND (NEW.ferme_le > a_fin OR NEW.ferme_le < NEW.ouvert_le) THEN
        RAISE EXCEPTION 'groupe_incoherent: fermeture hors annee' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_groupe_garde ON ent_groupe;
CREATE TRIGGER ent_groupe_garde BEFORE INSERT OR UPDATE ON ent_groupe
    FOR EACH ROW EXECUTE FUNCTION ent_trg_groupe();

-- Un groupe « liste » ne donne aucun droit : il ne peut pas être le public d'un cours (le public d'un
-- cours ouvre la visibilité du professeur, D-022).
CREATE OR REPLACE FUNCTION ent_trg_cours_groupe_droits() RETURNS trigger AS $$
BEGIN
    IF NEW.groupe_id IS NOT NULL AND EXISTS (
        SELECT 1 FROM ent_groupe g
         WHERE g.id = NEW.groupe_id AND g.etablissement_id = NEW.etablissement_id AND NOT g.ouvre_droits) THEN
        RAISE EXCEPTION 'groupe_sans_droits: un groupe de nature liste ne peut pas être le public d''un cours'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_cours_groupe_droits ON ent_cours;
CREATE TRIGGER ent_cours_groupe_droits BEFORE INSERT ON ent_cours
    FOR EACH ROW EXECUTE FUNCTION ent_trg_cours_groupe_droits();

-- ───────────────────────── Classes d'origine d'un groupe (0..n) ─────────────────────────
CREATE UNIQUE INDEX IF NOT EXISTS ent_groupe_id_etab_uq ON ent_groupe (id, etablissement_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_classe_id_etab_uq ON ent_classe (id, etablissement_id);

CREATE TABLE IF NOT EXISTS ent_groupe_classe (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    groupe_id UUID NOT NULL,
    classe_id UUID NOT NULL,
    ajoute_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (groupe_id, classe_id),
    CONSTRAINT ent_groupe_classe_groupe_fk FOREIGN KEY (groupe_id, etablissement_id)
        REFERENCES ent_groupe (id, etablissement_id),
    CONSTRAINT ent_groupe_classe_classe_fk FOREIGN KEY (classe_id, etablissement_id)
        REFERENCES ent_classe (id, etablissement_id)
);
CREATE INDEX IF NOT EXISTS ent_groupe_classe_classe_idx ON ent_groupe_classe (etablissement_id, classe_id);

-- La classe d'origine est de la même année que le groupe.
CREATE OR REPLACE FUNCTION ent_trg_groupe_classe_garde() RETURNS trigger AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ent_groupe g JOIN ent_classe c ON c.etablissement_id = g.etablissement_id AND c.annee_id = g.annee_id
         WHERE g.id = NEW.groupe_id AND c.id = NEW.classe_id AND g.etablissement_id = NEW.etablissement_id) THEN
        RAISE EXCEPTION 'structure_incoherente: classe d''origine d''une autre année' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_groupe_classe_garde ON ent_groupe_classe;
CREATE TRIGGER ent_groupe_classe_garde BEFORE INSERT ON ent_groupe_classe
    FOR EACH ROW EXECUTE FUNCTION ent_trg_groupe_classe_garde();

-- `ent_groupe.classe_id` = projection : la classe d'origine si elle est unique, sinon NULL.
CREATE OR REPLACE FUNCTION ent_trg_groupe_classe_projection() RETURNS trigger AS $$
DECLARE g UUID; e UUID;
BEGIN
    g := COALESCE(NEW.groupe_id, OLD.groupe_id);
    e := COALESCE(NEW.etablissement_id, OLD.etablissement_id);
    UPDATE ent_groupe x
       SET classe_id = (SELECT CASE WHEN count(*) = 1 THEN (array_agg(gc.classe_id))[1] END
                          FROM ent_groupe_classe gc WHERE gc.groupe_id = g AND gc.etablissement_id = e)
     WHERE x.id = g AND x.etablissement_id = e
       AND x.classe_id IS DISTINCT FROM (SELECT CASE WHEN count(*) = 1 THEN (array_agg(gc.classe_id))[1] END
                                           FROM ent_groupe_classe gc WHERE gc.groupe_id = g AND gc.etablissement_id = e);
    RETURN NULL;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_groupe_classe_projection ON ent_groupe_classe;
CREATE TRIGGER ent_groupe_classe_projection AFTER INSERT OR DELETE ON ent_groupe_classe
    FOR EACH ROW EXECUTE FUNCTION ent_trg_groupe_classe_projection();

-- Les écrivains historiques posent encore `classe_id` : la classe devient une classe d'origine.
CREATE OR REPLACE FUNCTION ent_trg_groupe_classe_historique() RETURNS trigger AS $$
BEGIN
    IF NEW.classe_id IS NOT NULL THEN
        INSERT INTO ent_groupe_classe (etablissement_id, groupe_id, classe_id)
        VALUES (NEW.etablissement_id, NEW.id, NEW.classe_id) ON CONFLICT DO NOTHING;
    END IF;
    RETURN NULL;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_groupe_classe_historique ON ent_groupe;
CREATE TRIGGER ent_groupe_classe_historique AFTER INSERT ON ent_groupe
    FOR EACH ROW EXECUTE FUNCTION ent_trg_groupe_classe_historique();

-- Reprise de l'existant : une ligne par groupe déjà rattaché à une classe.
INSERT INTO ent_groupe_classe (etablissement_id, groupe_id, classe_id)
SELECT g.etablissement_id, g.id, g.classe_id
  FROM ent_groupe g
 WHERE g.classe_id IS NOT NULL
ON CONFLICT DO NOTHING;

ALTER TABLE ent_groupe_classe ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_groupe_classe FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_groupe_classe;
CREATE POLICY ent_rls_etablissement ON ent_groupe_classe FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));

-- ───────────────────────── Affectations : motif, annulation, cohérence ─────────────────────────
ALTER TABLE ent_affectation ADD COLUMN IF NOT EXISTS motif_code TEXT;
ALTER TABLE ent_affectation ADD COLUMN IF NOT EXISTS decide_par UUID;

-- La provenance d'une affectation du lot S6 (règle, changement de classe, dédoublement, correction…) est le CODE
-- DE MOTIF ; `source` garde ses valeurs historiques (110 la repose à chaque rejeu complet des migrations, et refuserait
-- toute valeur nouvelle déjà présente en base).
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_affectation_motif_chk') THEN
        ALTER TABLE ent_affectation ADD CONSTRAINT ent_affectation_motif_chk
            CHECK (motif_code IS NULL OR motif_code ~ '^[a-z0-9_]{1,40}$') NOT VALID;
    END IF;
END $$;

-- Annulation : l'archive garde le motif et l'auteur, transmis par la transaction
-- (`ent.annulation_motif`, `ent.annulation_par`). Une suppression sans motif reste tracée (comme en S0).
ALTER TABLE ent_affectation_supprimee ADD COLUMN IF NOT EXISTS motif_annulation TEXT;
ALTER TABLE ent_affectation_supprimee ADD COLUMN IF NOT EXISTS annulee_par UUID;
ALTER TABLE ent_affectation_supprimee ADD COLUMN IF NOT EXISTS motif_code TEXT;

CREATE OR REPLACE FUNCTION ent_trg_affectation_archive() RETURNS trigger AS $$
BEGIN
    INSERT INTO ent_affectation_supprimee
        (id, etablissement_id, annee_id, eleve_id, classe_id, groupe_id, date_debut, date_fin, source, created_at,
         motif_annulation, annulee_par, motif_code)
    VALUES (OLD.id, OLD.etablissement_id, OLD.annee_id, OLD.eleve_id, OLD.classe_id, OLD.groupe_id,
            OLD.date_debut, OLD.date_fin, OLD.source, OLD.created_at,
            left(nullif(current_setting('ent.annulation_motif', true), ''), 300),
            nullif(current_setting('ent.annulation_par', true), '')::uuid,
            OLD.motif_code);
    RETURN OLD;
END
$$ LANGUAGE plpgsql;

-- Cohérence d'une affectation : à l'INSERTION d'une ligne qui porte un code de motif (toute écriture du lot S6 ; les
-- anciens écrivains, dont la reprise des options, n'en posent pas et peuvent avoir des états que cette migration ne
-- doit pas rendre illégaux), pas de chevauchement (même groupe ; deux classes) et dates dans l'année ;
-- à toute écriture, un groupe fermé ne garde pas d'élève au-delà de sa fermeture. Les mises à jour (clôture, correction
-- de dates) ne sont pas recontrôlées pour le chevauchement : les anciens écrivains y posent des états que cette
-- migration ne doit pas rendre illégaux ; les commandes du lot S6 ne font que raccourcir.
CREATE OR REPLACE FUNCTION ent_trg_affectation_dates() RETURNS trigger AS $$
DECLARE
    a_debut DATE; a_fin DATE; g_ouvert DATE; g_ferme DATE;
BEGIN
    SELECT y.date_debut, y.date_fin INTO a_debut, a_fin
      FROM ent_annee_scolaire y WHERE y.id = NEW.annee_id AND y.etablissement_id = NEW.etablissement_id;
    IF TG_OP = 'INSERT' AND (NEW.date_debut < a_debut OR NEW.date_debut > a_fin) THEN
        RAISE EXCEPTION 'affectation_hors_annee' USING ERRCODE = '23514';
    END IF;
    IF NEW.groupe_id IS NOT NULL THEN
        SELECT g.ouvert_le, g.ferme_le INTO g_ouvert, g_ferme
          FROM ent_groupe g WHERE g.id = NEW.groupe_id AND g.etablissement_id = NEW.etablissement_id;
        IF TG_OP = 'INSERT' AND (NEW.date_debut < g_ouvert OR (g_ferme IS NOT NULL AND NEW.date_debut > g_ferme)) THEN
            RAISE EXCEPTION 'affectation_groupe_ferme' USING ERRCODE = '23514';
        END IF;
        IF g_ferme IS NOT NULL AND (NEW.date_fin IS NULL OR NEW.date_fin > g_ferme)
           AND NEW.date_debut <= g_ferme THEN
            RAISE EXCEPTION 'affectation_groupe_ferme' USING ERRCODE = '23514';
        END IF;
        IF TG_OP = 'INSERT' AND NEW.motif_code IS NOT NULL AND EXISTS (SELECT 1 FROM ent_affectation o
                    WHERE o.etablissement_id = NEW.etablissement_id AND o.eleve_id = NEW.eleve_id
                      AND o.groupe_id = NEW.groupe_id AND o.id <> NEW.id
                      AND o.date_debut <= COALESCE(NEW.date_fin, 'infinity'::date)
                      AND COALESCE(o.date_fin, 'infinity'::date) >= NEW.date_debut) THEN
            RAISE EXCEPTION 'affectation_chevauchement' USING ERRCODE = '23514';
        END IF;
    ELSIF NEW.classe_id IS NOT NULL THEN
        IF TG_OP = 'INSERT' AND NEW.motif_code IS NOT NULL AND EXISTS (SELECT 1 FROM ent_affectation o
                    WHERE o.etablissement_id = NEW.etablissement_id AND o.eleve_id = NEW.eleve_id
                      AND o.annee_id = NEW.annee_id AND o.classe_id IS NOT NULL AND o.id <> NEW.id
                      AND o.date_debut <= COALESCE(NEW.date_fin, 'infinity'::date)
                      AND COALESCE(o.date_fin, 'infinity'::date) >= NEW.date_debut) THEN
            RAISE EXCEPTION 'affectation_chevauchement' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_affectation_dates ON ent_affectation;
CREATE TRIGGER ent_affectation_dates BEFORE INSERT OR UPDATE OF date_debut, date_fin, groupe_id, classe_id, eleve_id
    ON ent_affectation FOR EACH ROW EXECUTE FUNCTION ent_trg_affectation_dates();

-- ───────────────────────── Lectures : la composition à une date ─────────────────────────
-- Membres d'un groupe à `p_jour` (élèves scolarisés ce jour-là, hors élèves fusionnés). Une seule
-- implémentation : le port Rust `groupes::membres` et les contrôles la lisent.
CREATE OR REPLACE FUNCTION ent_groupe_membres_jour(p_etab UUID, p_jour DATE, p_groupe UUID DEFAULT NULL)
RETURNS TABLE (groupe_id UUID, eleve_id UUID) LANGUAGE sql STABLE AS $$
    SELECT DISTINCT a.groupe_id, a.eleve_id
      FROM ent_affectation a
      JOIN ent_eleve e ON e.id = a.eleve_id AND e.etablissement_id = a.etablissement_id
     WHERE a.etablissement_id = p_etab
       AND a.groupe_id IS NOT NULL
       AND (p_groupe IS NULL OR a.groupe_id = p_groupe)
       AND a.date_debut <= p_jour AND (a.date_fin IS NULL OR a.date_fin >= p_jour)
       AND (e.date_sortie IS NULL OR e.date_sortie > p_jour)
       AND e.fusionne_dans IS NULL
$$;

-- Groupes d'un élève à `p_jour`.
CREATE OR REPLACE FUNCTION ent_groupes_eleve_jour(p_etab UUID, p_jour DATE, p_eleve UUID)
RETURNS TABLE (groupe_id UUID) LANGUAGE sql STABLE AS $$
    SELECT DISTINCT a.groupe_id
      FROM ent_affectation a
      JOIN ent_eleve e ON e.id = a.eleve_id AND e.etablissement_id = a.etablissement_id
     WHERE a.etablissement_id = p_etab AND a.eleve_id = p_eleve AND a.groupe_id IS NOT NULL
       AND a.date_debut <= p_jour AND (a.date_fin IS NULL OR a.date_fin >= p_jour)
       AND (e.date_sortie IS NULL OR e.date_sortie > p_jour)
$$;

-- ───────────────────────── Journal de structure : objets du lot ─────────────────────────
-- Union de TOUS les lots (S2, S5, S3, S7, S4 + S6). NOT VALID : la liste ne fait que s'allonger.
ALTER TABLE ent_structure_journal DROP CONSTRAINT IF EXISTS ent_structure_journal_objet_check;
ALTER TABLE ent_structure_journal ADD CONSTRAINT ent_structure_journal_objet_check CHECK (objet IN
    ('profil', 'parametre', 'fuseau', 'site', 'batiment', 'salle', 'lieu', 'disponibilite',
     'indisponibilite', 'import_salles', 'prereglage',
     'matiere', 'offre', 'offre_groupe', 'regle_choix', 'incompatibilite', 'prerequis',
     'enseignements_suivis', 'enseignement_suivi',
     'calendrier_evenement', 'calendrier_adoption', 'semaines', 'semaine', 'fermeture',
     'periodisation', 'periode', 'grille',
     'cours', 'cours_membre', 'cours_service',
     'groupe', 'groupe_classe', 'affectation', 'composition')) NOT VALID;
