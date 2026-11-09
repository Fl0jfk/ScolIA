-- Socle 5, lot S4 (`05_STRUCTURE.md` §9.7, §17 M7, STR-56 à STR-58, Q-S3, Q-S20, R-S5, D-022) :
-- l'objet « cours » = CE QUI EST ENSEIGNÉ À QUI (une matière, à une classe entière OU à un groupe,
-- pour une année), distinct du SERVICE = QUI l'enseigne (`ent_service_enseignement`, socle 6).
--
--   ent_cours ──< ent_service_enseignement   (un cours peut avoir 0, 1 ou plusieurs services)
--   ent_cours ──< ent_cours_membre           (inclusions et exclusions individuelles datées, R-S5)
--   ent_cours ──< ent_edt_creneau.cours_id   (contrat avec le socle 10)
--
-- * POSTE VACANT = cours ouvert sans aucun service. Rien n'est inventé : il suffit qu'aucune ligne
--   de service ne le vise (fonction `ent_cours_vacants`).
-- * PUBLIC d'un cours à la date d : une seule fonction, `ent_public_cours_jour` (redéfinie ici sur
--   `ent_cours`) = (classe ou groupe à d) + inclusions actives - exclusions actives, hors sortis.
-- * REPRISE AUTOMATIQUE de l'existant (M7) : chaque `ent_service_enseignement` reçoit un `cours_id` ;
--   deux professeurs sur la même cible et la même matière = UN cours, DEUX services (dédoublonnage).
--   Aucune ligne n'est supprimée ni modifiée dans ses colonnes d'origine ; les membres individuels
--   attachés à une ligne de service sont rattachés au cours (la limite n° 1 du lot S1 disparaît).
--   Rejouable : exécutée à chaque démarrage, ne fait rien la deuxième fois.
-- * Les écrivains historiques de services (`INSERT INTO ent_service_enseignement`) n'ont pas à changer :
--   un déclencheur trouve ou crée le cours.
-- * Un cours ne se supprime jamais : il se ferme. Son public (classe ou groupe), sa matière et son
--   année ne changent jamais : changer de public = fermer et ouvrir (STR-57, l'historique des notes
--   reste sur l'ancien cours).
-- Additif et rejouable.

-- ───────────────────────── Cibles des clés composites ─────────────────────────
CREATE UNIQUE INDEX IF NOT EXISTS ent_classe_id_etab_s4_uq ON ent_classe (id, etablissement_id);

-- ───────────────────────── Le cours ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_cours (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    -- Libellé affiché (reprend celui de la matière quand elle existe et que rien n'est saisi).
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 160),
    matiere_id UUID,
    -- Offre d'enseignements de l'année (socle 5, lot S5) dont le cours est la mise en oeuvre.
    offre_id UUID,
    -- Public : exactement un des deux.
    classe_id UUID,
    groupe_id UUID,
    heures_hebdo NUMERIC(4, 2) CHECK (heures_hebdo IS NULL OR (heures_hebdo >= 0 AND heures_hebdo <= 40)),
    semaines TEXT CHECK (semaines IS NULL OR semaines IN ('toutes', 'A', 'B')),
    ouvert_le DATE NOT NULL,
    ferme_le DATE,
    reconduit_de_id UUID,
    -- 'saisie' (commande STR-57), 'reprise' (M7), 'service' (déclencheur d'un écrivain historique),
    -- 'generation' (STR-56, lot S10).
    source TEXT NOT NULL DEFAULT 'saisie' CHECK (source IN ('saisie', 'reprise', 'service', 'generation')),
    -- Posé à la reprise quand le service n'avait ni matière ni libellé : à revoir par la gestion.
    a_verifier BOOLEAN NOT NULL DEFAULT false,
    cree_par UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cours_public_chk CHECK ((classe_id IS NULL) <> (groupe_id IS NULL)),
    CONSTRAINT ent_cours_dates_chk CHECK (ferme_le IS NULL OR ferme_le >= ouvert_le)
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_cours_id_etab_uq ON ent_cours (id, etablissement_id);

-- Un seul cours OUVERT par (année, public, matière) ; sans matière du référentiel, par libellé.
CREATE UNIQUE INDEX IF NOT EXISTS ent_cours_ouvert_uq ON ent_cours (
    etablissement_id, annee_id, coalesce(classe_id, groupe_id),
    coalesce(matiere_id, '00000000-0000-0000-0000-000000000000'::uuid),
    (CASE WHEN matiere_id IS NULL THEN lower(btrim(libelle)) ELSE '' END)
) WHERE ferme_le IS NULL;

CREATE INDEX IF NOT EXISTS ent_cours_classe_idx ON ent_cours (etablissement_id, annee_id, classe_id) WHERE classe_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_cours_groupe_idx ON ent_cours (etablissement_id, annee_id, groupe_id) WHERE groupe_id IS NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_cours_annee_fk') THEN
        ALTER TABLE ent_cours ADD CONSTRAINT ent_cours_annee_fk FOREIGN KEY (annee_id, etablissement_id)
            REFERENCES ent_annee_scolaire (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_cours_matiere_fk') THEN
        ALTER TABLE ent_cours ADD CONSTRAINT ent_cours_matiere_fk FOREIGN KEY (matiere_id, etablissement_id)
            REFERENCES ent_matiere (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_cours_offre_fk') THEN
        ALTER TABLE ent_cours ADD CONSTRAINT ent_cours_offre_fk FOREIGN KEY (offre_id, etablissement_id)
            REFERENCES ent_offre_enseignement (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_cours_classe_fk') THEN
        ALTER TABLE ent_cours ADD CONSTRAINT ent_cours_classe_fk FOREIGN KEY (classe_id, etablissement_id)
            REFERENCES ent_classe (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_cours_groupe_fk') THEN
        ALTER TABLE ent_cours ADD CONSTRAINT ent_cours_groupe_fk FOREIGN KEY (groupe_id, etablissement_id)
            REFERENCES ent_groupe (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_cours_reconduit_fk') THEN
        ALTER TABLE ent_cours ADD CONSTRAINT ent_cours_reconduit_fk FOREIGN KEY (reconduit_de_id, etablissement_id)
            REFERENCES ent_cours (id, etablissement_id);
    END IF;
END $$;

-- Garde-fous du cours.
CREATE OR REPLACE FUNCTION ent_trg_cours() RETURNS trigger AS $$
DECLARE
    a_debut DATE; a_fin DATE; a_close TIMESTAMPTZ; o_annee UUID;
    reprise BOOLEAN := coalesce(current_setting('ent.cours_reprise', true), '') = 'on';
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'cours_non_supprimable' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'UPDATE' THEN
        IF (NEW.etablissement_id, NEW.annee_id, NEW.classe_id, NEW.groupe_id, NEW.matiere_id, NEW.ouvert_le, NEW.source)
               IS DISTINCT FROM (OLD.etablissement_id, OLD.annee_id, OLD.classe_id, OLD.groupe_id, OLD.matiere_id, OLD.ouvert_le, OLD.source) THEN
            RAISE EXCEPTION 'cours_public_immuable' USING ERRCODE = '23514';
        END IF;
        IF OLD.ferme_le IS NOT NULL AND NEW.ferme_le IS DISTINCT FROM OLD.ferme_le THEN
            RAISE EXCEPTION 'cours_ferme_definitif' USING ERRCODE = '23514';
        END IF;
    END IF;
    SELECT y.date_debut, y.date_fin, y.cloturee_le INTO a_debut, a_fin, a_close
      FROM ent_annee_scolaire y WHERE y.id = NEW.annee_id AND y.etablissement_id = NEW.etablissement_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'cours_incoherent: annee' USING ERRCODE = '23514';
    END IF;
    IF a_close IS NOT NULL AND NOT reprise THEN
        RAISE EXCEPTION 'cours_annee_cloturee' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'INSERT' THEN
        IF NEW.classe_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM ent_classe c WHERE c.id = NEW.classe_id AND c.etablissement_id = NEW.etablissement_id AND c.annee_id = NEW.annee_id) THEN
            RAISE EXCEPTION 'cours_incoherent: classe' USING ERRCODE = '23514';
        END IF;
        IF NEW.groupe_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM ent_groupe g WHERE g.id = NEW.groupe_id AND g.etablissement_id = NEW.etablissement_id AND g.annee_id = NEW.annee_id) THEN
            RAISE EXCEPTION 'cours_incoherent: groupe' USING ERRCODE = '23514';
        END IF;
        IF NEW.ouvert_le < a_debut OR NEW.ouvert_le > a_fin THEN
            RAISE EXCEPTION 'cours_incoherent: ouverture hors annee' USING ERRCODE = '23514';
        END IF;
    END IF;
    IF NEW.ferme_le IS NOT NULL AND NEW.ferme_le > a_fin THEN
        RAISE EXCEPTION 'cours_incoherent: fermeture hors annee' USING ERRCODE = '23514';
    END IF;
    IF NEW.offre_id IS NOT NULL THEN
        SELECT o.annee_id INTO o_annee FROM ent_offre_enseignement o
         WHERE o.id = NEW.offre_id AND o.etablissement_id = NEW.etablissement_id;
        IF o_annee IS DISTINCT FROM NEW.annee_id THEN
            RAISE EXCEPTION 'cours_incoherent: offre' USING ERRCODE = '23514';
        END IF;
    END IF;
    IF TG_OP = 'UPDATE' THEN
        NEW.modifie_le := now();
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_cours_garde ON ent_cours;
CREATE TRIGGER ent_cours_garde BEFORE INSERT OR UPDATE OR DELETE ON ent_cours
    FOR EACH ROW EXECUTE FUNCTION ent_trg_cours();

-- Trouve ou crée le cours OUVERT d'un public et d'une matière (clé de dédoublonnage de M7).
CREATE OR REPLACE FUNCTION ent_cours_assurer(
    p_etab UUID, p_annee UUID, p_classe UUID, p_groupe UUID, p_matiere UUID, p_libelle TEXT, p_source TEXT)
RETURNS UUID LANGUAGE plpgsql AS $$
DECLARE v_id UUID; v_lib TEXT; v_debut DATE; v_verif BOOLEAN := false;
BEGIN
    v_lib := btrim(coalesce(p_libelle, ''));
    IF p_matiere IS NOT NULL AND v_lib = '' THEN
        SELECT m.libelle INTO v_lib FROM ent_matiere m WHERE m.id = p_matiere AND m.etablissement_id = p_etab;
    END IF;
    v_lib := btrim(coalesce(v_lib, ''));
    IF v_lib = '' THEN
        v_lib := 'Cours à préciser';
        v_verif := true;
    END IF;
    FOR i IN 1..2 LOOP
        SELECT c.id INTO v_id FROM ent_cours c
         WHERE c.etablissement_id = p_etab AND c.annee_id = p_annee AND c.ferme_le IS NULL
           AND coalesce(c.classe_id, c.groupe_id) = coalesce(p_classe, p_groupe)
           AND coalesce(c.matiere_id, '00000000-0000-0000-0000-000000000000'::uuid)
               = coalesce(p_matiere, '00000000-0000-0000-0000-000000000000'::uuid)
           AND (p_matiere IS NOT NULL OR lower(btrim(c.libelle)) = lower(v_lib));
        IF FOUND THEN
            RETURN v_id;
        END IF;
        SELECT y.date_debut INTO v_debut FROM ent_annee_scolaire y WHERE y.id = p_annee AND y.etablissement_id = p_etab;
        INSERT INTO ent_cours (id, etablissement_id, annee_id, libelle, matiere_id, classe_id, groupe_id,
                               ouvert_le, source, a_verifier)
        VALUES (gen_random_uuid(), p_etab, p_annee, v_lib, p_matiere, p_classe, p_groupe, v_debut,
                p_source, v_verif)
        ON CONFLICT DO NOTHING;
    END LOOP;
    RAISE EXCEPTION 'cours_assurer_impossible' USING ERRCODE = '23505';
END
$$;

-- ───────────────────────── Le service pointe vers son cours ─────────────────────────
ALTER TABLE ent_service_enseignement ADD COLUMN IF NOT EXISTS cours_id UUID;
CREATE INDEX IF NOT EXISTS ent_service_cours_idx ON ent_service_enseignement (etablissement_id, cours_id);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_service_cours_fk') THEN
        ALTER TABLE ent_service_enseignement ADD CONSTRAINT ent_service_cours_fk FOREIGN KEY (cours_id, etablissement_id)
            REFERENCES ent_cours (id, etablissement_id);
    END IF;
END $$;

-- INSERT sans cours_id : le cours est trouvé ou créé (les écrivains historiques ne changent pas).
-- INSERT avec cours_id : la cible du service est recopiée du cours (le socle 6 pose un service SUR un cours).
-- UPDATE qui change la cible ou la matière : le service est rattaché au cours qui convient.
-- UPDATE de cours_id seul : la cohérence avec la cible du service est vérifiée, rien n'est recopié.
CREATE OR REPLACE FUNCTION ent_trg_service_cours() RETURNS trigger AS $$
DECLARE c ent_cours%ROWTYPE;
    reprise BOOLEAN := coalesce(current_setting('ent.cours_reprise', true), '') = 'on';
BEGIN
    IF NEW.cours_id IS NOT NULL AND (TG_OP = 'INSERT' OR NEW.cours_id IS DISTINCT FROM OLD.cours_id) THEN
        SELECT * INTO c FROM ent_cours WHERE id = NEW.cours_id AND etablissement_id = NEW.etablissement_id;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'service_cours_incoherent: cours' USING ERRCODE = '23514';
        END IF;
        IF TG_OP = 'INSERT' THEN
            NEW.annee_id := c.annee_id;
            NEW.classe_id := c.classe_id;
            NEW.groupe_id := c.groupe_id;
            NEW.matiere_id := c.matiere_id;
            NEW.matiere_libelle := c.libelle;
        ELSIF (NEW.annee_id, NEW.classe_id, NEW.groupe_id, NEW.matiere_id)
                IS DISTINCT FROM (c.annee_id, c.classe_id, c.groupe_id, c.matiere_id) THEN
            RAISE EXCEPTION 'service_cours_incoherent: cible' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.cours_id IS NULL
       OR (TG_OP = 'UPDATE' AND (NEW.annee_id, NEW.classe_id, NEW.groupe_id, NEW.matiere_id, NEW.matiere_libelle)
              IS DISTINCT FROM (OLD.annee_id, OLD.classe_id, OLD.groupe_id, OLD.matiere_id, OLD.matiere_libelle)) THEN
        NEW.cours_id := ent_cours_assurer(NEW.etablissement_id, NEW.annee_id, NEW.classe_id, NEW.groupe_id,
                                          NEW.matiere_id, NEW.matiere_libelle,
                                          CASE WHEN reprise THEN 'reprise' ELSE 'service' END);
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

-- Si un service change de cours (sa cible ou sa matière change), ses créneaux le suivent.
CREATE OR REPLACE FUNCTION ent_trg_service_cours_propage() RETURNS trigger AS $$
BEGIN
    IF NEW.cours_id IS DISTINCT FROM OLD.cours_id THEN
        UPDATE ent_edt_creneau SET cours_id = NEW.cours_id
         WHERE etablissement_id = NEW.etablissement_id AND service_id = NEW.id;
    END IF;
    RETURN NULL;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_service_cours ON ent_service_enseignement;
CREATE TRIGGER ent_service_cours BEFORE INSERT OR UPDATE OF annee_id, classe_id, groupe_id, matiere_id, matiere_libelle, cours_id
    ON ent_service_enseignement FOR EACH ROW EXECUTE FUNCTION ent_trg_service_cours();

DROP TRIGGER IF EXISTS ent_service_cours_propage ON ent_service_enseignement;
CREATE TRIGGER ent_service_cours_propage AFTER UPDATE OF cours_id
    ON ent_service_enseignement FOR EACH ROW EXECUTE FUNCTION ent_trg_service_cours_propage();

-- ───────────────────────── Créneaux d'emploi du temps : contrat avec le socle 10 ─────────────────────────
ALTER TABLE ent_edt_creneau ADD COLUMN IF NOT EXISTS cours_id UUID;
CREATE INDEX IF NOT EXISTS ent_edt_creneau_cours_idx ON ent_edt_creneau (etablissement_id, cours_id) WHERE cours_id IS NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_creneau_cours_fk') THEN
        ALTER TABLE ent_edt_creneau ADD CONSTRAINT ent_edt_creneau_cours_fk FOREIGN KEY (cours_id, etablissement_id)
            REFERENCES ent_cours (id, etablissement_id);
    END IF;
END $$;

-- Un créneau qui vise un service hérite du cours de ce service ; un créneau peut aussi viser un cours
-- sans service (poste vacant). Incohérence service / cours = refus.
CREATE OR REPLACE FUNCTION ent_trg_creneau_cours() RETURNS trigger AS $$
DECLARE v UUID;
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.service_id IS DISTINCT FROM OLD.service_id
       AND NEW.cours_id IS NOT DISTINCT FROM OLD.cours_id THEN
        NEW.cours_id := NULL;
    END IF;
    IF NEW.service_id IS NOT NULL THEN
        SELECT s.cours_id INTO v FROM ent_service_enseignement s
         WHERE s.id = NEW.service_id AND s.etablissement_id = NEW.etablissement_id;
        IF NEW.cours_id IS NULL THEN
            NEW.cours_id := v;
        ELSIF v IS NOT NULL AND v <> NEW.cours_id THEN
            RAISE EXCEPTION 'creneau_cours_incoherent' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_edt_creneau_cours ON ent_edt_creneau;
CREATE TRIGGER ent_edt_creneau_cours BEFORE INSERT OR UPDATE OF service_id, cours_id
    ON ent_edt_creneau FOR EACH ROW EXECUTE FUNCTION ent_trg_creneau_cours();

-- ───────────────────────── Membres individuels : rattachés au cours (R-S5) ─────────────────────────
-- `cours_id` désignait une ligne de service (lot S1) ; il désigne désormais le cours. La reprise
-- ci-dessous repointe les lignes existantes (et garde l'ancienne ligne de service dans `service_origine_id`).
ALTER TABLE ent_cours_membre DROP CONSTRAINT IF EXISTS ent_cours_membre_cours_id_fkey;
ALTER TABLE ent_cours_membre ADD COLUMN IF NOT EXISTS service_origine_id UUID;

CREATE OR REPLACE FUNCTION ent_trg_cours_membre() RETURNS trigger AS $$
DECLARE a_debut DATE; a_fin DATE; a_close TIMESTAMPTZ;
    reprise BOOLEAN := coalesce(current_setting('ent.cours_reprise', true), '') = 'on';
BEGIN
    -- Un membre ne se modifie pas : seule l'annulation (annulee_le NULL -> date) est permise.
    -- Exception : la reprise du lot S4 repointe `cours_id` (de la ligne de service vers le cours).
    IF TG_OP = 'UPDATE' AND NOT reprise THEN
        IF (NEW.etablissement_id, NEW.cours_id, NEW.eleve_id, NEW.nature, NEW.motif_code, NEW.du, NEW.au, NEW.service_origine_id)
               IS DISTINCT FROM (OLD.etablissement_id, OLD.cours_id, OLD.eleve_id, OLD.nature, OLD.motif_code, OLD.du, OLD.au, OLD.service_origine_id)
           OR (OLD.annulee_le IS NOT NULL AND NEW.annulee_le IS DISTINCT FROM OLD.annulee_le)
           OR (OLD.annulee_par IS NOT NULL AND NEW.annulee_par IS DISTINCT FROM OLD.annulee_par) THEN
            RAISE EXCEPTION 'cours_membre_immuable' USING ERRCODE = '23514';
        END IF;
    END IF;
    IF TG_OP = 'UPDATE' AND reprise
       AND (NEW.etablissement_id, NEW.eleve_id, NEW.nature, NEW.motif_code, NEW.du, NEW.au)
           IS DISTINCT FROM (OLD.etablissement_id, OLD.eleve_id, OLD.nature, OLD.motif_code, OLD.du, OLD.au) THEN
        RAISE EXCEPTION 'cours_membre_immuable' USING ERRCODE = '23514';
    END IF;
    SELECT y.date_debut, y.date_fin, y.cloturee_le INTO a_debut, a_fin, a_close
      FROM ent_cours c
      JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
     WHERE c.id = NEW.cours_id AND c.etablissement_id = NEW.etablissement_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'cours_membre_incoherent: cours' USING ERRCODE = '23514';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM ent_eleve e WHERE e.id = NEW.eleve_id AND e.etablissement_id = NEW.etablissement_id) THEN
        RAISE EXCEPTION 'cours_membre_incoherent: eleve' USING ERRCODE = '23514';
    END IF;
    IF a_close IS NOT NULL AND NOT reprise THEN
        RAISE EXCEPTION 'cours_membre_annee_cloturee' USING ERRCODE = '23514';
    END IF;
    IF NOT reprise AND (NEW.du < a_debut OR NEW.du > a_fin OR (NEW.au IS NOT NULL AND NEW.au > a_fin)) THEN
        RAISE EXCEPTION 'cours_membre_incoherent: hors annee' USING ERRCODE = '23514';
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(NEW.cours_id::text, 0));
    -- Pas deux périodes actives qui se chevauchent pour (cours, élève, nature). La reprise garde
    -- telles quelles les lignes venues de plusieurs services du même cours (rien n'est perdu).
    IF NOT reprise AND NEW.annulee_le IS NULL AND EXISTS (
        SELECT 1 FROM ent_cours_membre m
         WHERE m.cours_id = NEW.cours_id AND m.eleve_id = NEW.eleve_id AND m.nature = NEW.nature
           AND m.annulee_le IS NULL AND m.id <> NEW.id
           AND m.du <= coalesce(NEW.au, 'infinity'::date) AND coalesce(m.au, 'infinity'::date) >= NEW.du) THEN
        RAISE EXCEPTION 'cours_membre_chevauchement' USING ERRCODE = '23505';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_cours_membre_cours_fk') THEN
        -- NOT VALID : la reprise (plus bas) repointe d'abord les lignes ; la validation vient après.
        ALTER TABLE ent_cours_membre ADD CONSTRAINT ent_cours_membre_cours_fk FOREIGN KEY (cours_id, etablissement_id)
            REFERENCES ent_cours (id, etablissement_id) NOT VALID;
    END IF;
END $$;

-- ───────────────────────── LA règle de visibilité, sur le cours ─────────────────────────
-- Même signature qu'au lot S1 (migration 331). `p_cours` est désormais un cours ; `p_prof` filtre
-- sur les cours qui ont un service de ce professeur (titulaire, co-intervenant, remplaçant : tout
-- service). Un cours sans service (poste vacant) n'est vu d'aucun professeur.
CREATE OR REPLACE FUNCTION ent_public_cours_jour(
    p_etab UUID, p_jour DATE,
    p_cours UUID DEFAULT NULL, p_prof UUID DEFAULT NULL, p_eleve UUID DEFAULT NULL)
RETURNS TABLE (cours_id UUID, eleve_id UUID) LANGUAGE sql STABLE AS $$
    SELECT DISTINCT x.cours_id, x.eleve_id
      FROM (
        SELECT c.id AS cours_id, a.eleve_id
          FROM ent_cours c
          JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
          JOIN ent_affectation a
            ON a.etablissement_id = c.etablissement_id AND a.annee_id = c.annee_id
           AND ((c.classe_id IS NOT NULL AND a.classe_id = c.classe_id)
             OR (c.groupe_id IS NOT NULL AND a.groupe_id = c.groupe_id))
         WHERE c.etablissement_id = p_etab
           AND (p_cours IS NULL OR c.id = p_cours)
           AND (p_prof IS NULL OR EXISTS (
                 SELECT 1 FROM ent_service_enseignement s
                  WHERE s.etablissement_id = c.etablissement_id AND s.cours_id = c.id AND s.prof_user_id = p_prof))
           AND (p_eleve IS NULL OR a.eleve_id = p_eleve)
           AND y.date_debut <= p_jour AND y.date_fin >= p_jour
           AND c.ouvert_le <= p_jour AND (c.ferme_le IS NULL OR c.ferme_le >= p_jour)
           AND a.date_debut <= p_jour AND (a.date_fin IS NULL OR a.date_fin >= p_jour)
        UNION
        SELECT c.id, m.eleve_id
          FROM ent_cours_membre m
          JOIN ent_cours c ON c.id = m.cours_id AND c.etablissement_id = m.etablissement_id
          JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
         WHERE m.etablissement_id = p_etab AND m.nature = 'inclusion' AND m.annulee_le IS NULL
           AND (p_cours IS NULL OR c.id = p_cours)
           AND (p_prof IS NULL OR EXISTS (
                 SELECT 1 FROM ent_service_enseignement s
                  WHERE s.etablissement_id = c.etablissement_id AND s.cours_id = c.id AND s.prof_user_id = p_prof))
           AND (p_eleve IS NULL OR m.eleve_id = p_eleve)
           AND y.date_debut <= p_jour AND y.date_fin >= p_jour
           AND c.ouvert_le <= p_jour AND (c.ferme_le IS NULL OR c.ferme_le >= p_jour)
           AND m.du <= p_jour AND (m.au IS NULL OR m.au >= p_jour)
      ) x
      JOIN ent_eleve e ON e.id = x.eleve_id AND e.etablissement_id = p_etab
     WHERE (e.date_sortie IS NULL OR e.date_sortie > p_jour)
       AND NOT EXISTS (
            SELECT 1 FROM ent_cours_membre ex
             WHERE ex.etablissement_id = p_etab AND ex.cours_id = x.cours_id AND ex.eleve_id = x.eleve_id
               AND ex.nature = 'exclusion' AND ex.annulee_le IS NULL
               AND ex.du <= p_jour AND (ex.au IS NULL OR ex.au >= p_jour))
$$;

-- Postes vacants : cours ouverts à la date, sans aucun service.
CREATE OR REPLACE FUNCTION ent_cours_vacants(p_etab UUID, p_jour DATE)
RETURNS TABLE (cours_id UUID) LANGUAGE sql STABLE AS $$
    SELECT c.id
      FROM ent_cours c
      JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
     WHERE c.etablissement_id = p_etab
       AND y.date_debut <= p_jour AND y.date_fin >= p_jour
       AND c.ouvert_le <= p_jour AND (c.ferme_le IS NULL OR c.ferme_le >= p_jour)
       AND NOT EXISTS (SELECT 1 FROM ent_service_enseignement s
                        WHERE s.etablissement_id = c.etablissement_id AND s.cours_id = c.id)
$$;

-- ───────────────────────── Reprise de l'existant (M7) ─────────────────────────
-- Ce qui ne peut pas être repris est rangé ici, sans faire échouer le démarrage.
CREATE TABLE IF NOT EXISTS ent_cours_a_traiter (
    service_id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    raison TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_cours_a_traiter_etab_idx ON ent_cours_a_traiter (etablissement_id);

CREATE OR REPLACE FUNCTION ent_cours_reprendre() RETURNS BIGINT LANGUAGE plpgsql AS $$
DECLARE r_s RECORD; r_m RECORD; n BIGINT := 0; v UUID; cr UUID;
BEGIN
    PERFORM set_config('ent.cours_reprise', 'on', true);
    -- 1. Un cours par (année, public, matière) ; un service par professeur sur ce cours.
    FOR r_s IN SELECT id, etablissement_id, annee_id, classe_id, groupe_id, matiere_id, matiere_libelle
               FROM ent_service_enseignement WHERE cours_id IS NULL ORDER BY created_at, id
    LOOP
        BEGIN
            v := ent_cours_assurer(r_s.etablissement_id, r_s.annee_id, r_s.classe_id, r_s.groupe_id,
                                   r_s.matiere_id, r_s.matiere_libelle, 'reprise');
            UPDATE ent_service_enseignement SET cours_id = v WHERE id = r_s.id;
            DELETE FROM ent_cours_a_traiter WHERE service_id = r_s.id;
            n := n + 1;
        EXCEPTION WHEN OTHERS THEN
            INSERT INTO ent_cours_a_traiter (service_id, etablissement_id, raison)
            VALUES (r_s.id, r_s.etablissement_id, left('erreur_reprise: ' || SQLERRM, 300))
            ON CONFLICT (service_id) DO UPDATE SET raison = EXCLUDED.raison;
        END;
    END LOOP;
    -- 2. Créneaux : le cours de leur service.
    BEGIN
        UPDATE ent_edt_creneau c SET cours_id = s.cours_id
          FROM ent_service_enseignement s
         WHERE c.service_id = s.id AND c.etablissement_id = s.etablissement_id
           AND c.cours_id IS NULL AND s.cours_id IS NOT NULL;
    EXCEPTION WHEN OTHERS THEN
        RAISE WARNING 'ent_cours_reprendre : créneaux non repris (%)', SQLERRM;
    END;
    -- 3. Membres individuels : de la ligne de service vers le cours (une seule fois : après, le
    --    cours_id n'est plus un identifiant de service).
    FOR r_m IN SELECT mb.id, s.cours_id AS cours_cible
               FROM ent_cours_membre mb
               JOIN ent_service_enseignement s ON s.id = mb.cours_id AND s.etablissement_id = mb.etablissement_id
              WHERE s.cours_id IS NOT NULL
              ORDER BY mb.created_at, mb.id
    LOOP
        BEGIN
            UPDATE ent_cours_membre SET service_origine_id = cours_id, cours_id = r_m.cours_cible WHERE id = r_m.id;
        EXCEPTION WHEN OTHERS THEN
            RAISE WARNING 'ent_cours_reprendre : membre % non repris (%)', r_m.id, SQLERRM;
        END;
    END LOOP;
    PERFORM set_config('ent.cours_reprise', '', true);
    RETURN n;
END
$$;

SELECT ent_cours_reprendre();

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_cours_membre_cours_fk' AND NOT convalidated)
       AND NOT EXISTS (SELECT 1 FROM ent_cours_membre mb WHERE NOT EXISTS (
            SELECT 1 FROM ent_cours c WHERE c.id = mb.cours_id AND c.etablissement_id = mb.etablissement_id)) THEN
        ALTER TABLE ent_cours_membre VALIDATE CONSTRAINT ent_cours_membre_cours_fk;
    END IF;
END $$;

-- L'archive des affectations supprimées (330) n'avait pas de clé primaire : le registre de fusion
-- des élèves en exige une (id de l'affectation d'origine, unique).
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_index i WHERE i.indrelid = 'ent_affectation_supprimee'::regclass AND i.indisprimary)
       AND NOT EXISTS (SELECT 1 FROM ent_affectation_supprimee GROUP BY id HAVING count(*) > 1) THEN
        ALTER TABLE ent_affectation_supprimee ADD PRIMARY KEY (id);
    END IF;
END $$;

-- ───────────────────────── RLS ─────────────────────────
DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_cours', 'ent_cours_a_traiter']
    LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('DROP POLICY IF EXISTS ent_rls_etablissement ON %I', t);
        EXECUTE format($p$CREATE POLICY ent_rls_etablissement ON %I FOR ALL
            USING (etablissement_id = (SELECT ent_rls_etablissement())
                OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
            WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
                OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))$p$, t);
    END LOOP;
END $$;

-- ───────────────────────── Journal de structure : nouveaux objets ─────────────────────────
-- NOT VALID : la liste ne fait que s'allonger, les lignes existantes la respectent ; ne pas parcourir
-- tout le journal sous verrou à chaque démarrage.
ALTER TABLE ent_structure_journal DROP CONSTRAINT IF EXISTS ent_structure_journal_objet_check;
ALTER TABLE ent_structure_journal ADD CONSTRAINT ent_structure_journal_objet_check CHECK (objet IN
    ('profil', 'parametre', 'fuseau', 'site', 'batiment', 'salle', 'lieu', 'disponibilite',
     'indisponibilite', 'import_salles', 'prereglage',
     'matiere', 'offre', 'offre_groupe', 'regle_choix', 'incompatibilite', 'prerequis',
     'enseignements_suivis', 'enseignement_suivi',
     'cours', 'cours_membre', 'cours_service')) NOT VALID;
