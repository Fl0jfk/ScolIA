-- Socle 2 (l'élève), lot E4 / étape M5 (1/2) : scolarisations datées et statut DÉRIVÉ.
--
-- Une scolarisation = un séjour continu dans l'établissement (entrée, origine, sortie,
-- destination). Un élève qui part et revient a DEUX scolarisations sur la MÊME fiche.
-- Le statut (« à venir », « scolarisé », « sorti ») se DÉDUIT de ces périodes : il ne se
-- saisit jamais. Les colonnes héritées d'`ent_eleve` (`status`, `date_entree`,
-- `date_sortie`, `motif_sortie`) deviennent des PROJECTIONS maintenues par la base ;
-- toute écriture héritée de ces colonnes est convertie en changement de la scolarisation
-- (pont ci-dessous, explicite : les commandes ELV-08/11/12 sont le chemin cible).
--
-- Convention de date (identique à l'existant, cf. `eleve_repo`) : `date_sortie` est le
-- PREMIER jour où l'élève n'est plus dans l'établissement (période [entrée, sortie[).
--
-- « Aujourd'hui » est le jour de l'ÉTABLISSEMENT (R-E15) : fuseau par établissement
-- (Europe/Paris par défaut). La table ci-dessous est provisoire : le paramètre définitif
-- sera fourni par le socle 5 (STR-03).
--
-- Additif, idempotent, rejoué à chaque démarrage. Numéros 281-289 : lots E3 et E4.

CREATE EXTENSION IF NOT EXISTS btree_gist;

-- ───────────────────────── Fuseau de l'établissement (R-E15) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_param_eleve_fuseau (
    etablissement_id UUID PRIMARY KEY,
    fuseau           TEXT NOT NULL DEFAULT 'Europe/Paris',
    modifie_le       TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_par      UUID
);
ALTER TABLE ent_param_eleve_fuseau ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_param_eleve_fuseau FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_param_eleve_fuseau;
CREATE POLICY ent_rls_etablissement ON ent_param_eleve_fuseau FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));

-- Un fuseau doit exister dans la base (nom IANA).
CREATE OR REPLACE FUNCTION ent_param_eleve_fuseau_valide() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_timezone_names WHERE name = NEW.fuseau) THEN
        RAISE EXCEPTION 'fuseau horaire inconnu' USING ERRCODE = '22023';
    END IF;
    RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS ent_param_eleve_fuseau_valide ON ent_param_eleve_fuseau;
CREATE TRIGGER ent_param_eleve_fuseau_valide
    BEFORE INSERT OR UPDATE ON ent_param_eleve_fuseau
    FOR EACH ROW EXECUTE FUNCTION ent_param_eleve_fuseau_valide();

-- Jour courant dans le fuseau de l'établissement.
CREATE OR REPLACE FUNCTION ent_jour_etablissement(p_etab UUID) RETURNS DATE
LANGUAGE sql STABLE AS $$
    SELECT (now() AT TIME ZONE coalesce(
        (SELECT f.fuseau FROM ent_param_eleve_fuseau f WHERE f.etablissement_id = p_etab),
        'Europe/Paris'))::date
$$;

-- ───────────────────────── Scolarisations ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_eleve_scolarisation (
    id                   UUID PRIMARY KEY,
    etablissement_id     UUID NOT NULL,
    eleve_id             UUID NOT NULL,
    date_entree          DATE NOT NULL,
    motif_entree         TEXT NOT NULL,
    origine_uai          TEXT,
    origine_libelle      TEXT,
    origine_type         TEXT,
    date_sortie          DATE,
    motif_sortie         TEXT,
    motif_sortie_libre   TEXT,
    destination_uai      TEXT,
    destination_libelle  TEXT,
    destination_type     TEXT,
    exeat_document_id    UUID,
    source               TEXT NOT NULL DEFAULT 'saisie',
    source_ref           UUID,
    decide_par           UUID,
    cree_le              TIMESTAMPTZ NOT NULL DEFAULT now(),
    cloture_le           TIMESTAMPTZ,
    cloture_par          UUID,
    CONSTRAINT ent_eleve_scolarisation_eleve_fk
        FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_eleve_scolarisation_regles_chk CHECK (
        motif_entree IN ('premiere_inscription', 'changement_etablissement', 'retour', 'passage_interne', 'reprise')
        AND (origine_type IS NULL OR origine_type IN
             ('public', 'prive_sous_contrat', 'prive_hors_contrat', 'instruction_famille', 'etranger', 'inconnu'))
        AND (destination_type IS NULL OR destination_type IN
             ('public', 'prive_sous_contrat', 'prive_hors_contrat', 'instruction_famille', 'etranger', 'inconnu'))
        AND (motif_sortie IS NULL OR motif_sortie IN
             ('fin_de_cycle', 'changement_etablissement', 'demenagement', 'exclusion_definitive', 'abandon',
              'fin_obligation', 'instruction_famille', 'etranger', 'deces', 'autre'))
        AND (date_sortie IS NULL OR date_sortie >= date_entree)
        AND (date_sortie IS NOT NULL OR (motif_sortie IS NULL AND destination_uai IS NULL
                                         AND destination_libelle IS NULL AND destination_type IS NULL))
        AND (origine_uai IS NULL OR origine_uai ~ '^[0-9A-Za-z]{1,20}$')
        AND (destination_uai IS NULL OR destination_uai ~ '^[0-9A-Za-z]{1,20}$')
        AND (origine_libelle IS NULL OR char_length(origine_libelle) <= 200)
        AND (destination_libelle IS NULL OR char_length(destination_libelle) <= 200)
        AND (motif_sortie_libre IS NULL OR char_length(motif_sortie_libre) <= 500)
        AND source IN ('saisie', 'admission', 'reinscription', 'montee', 'import', 'reprise', 'labo', 'heritage')
    ) NOT VALID
);

-- Pas de chevauchement : un élève n'est jamais scolarisé deux fois en même temps.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_eleve_scolarisation_chevauchement') THEN
        ALTER TABLE ent_eleve_scolarisation ADD CONSTRAINT ent_eleve_scolarisation_chevauchement
            EXCLUDE USING gist (eleve_id WITH =, daterange(date_entree, date_sortie, '[)') WITH &&);
    END IF;
END $$;

-- Une seule scolarisation ouverte par élève.
CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_scolarisation_ouverte_uniq
    ON ent_eleve_scolarisation (eleve_id) WHERE date_sortie IS NULL;
CREATE INDEX IF NOT EXISTS ent_eleve_scolarisation_eleve_idx
    ON ent_eleve_scolarisation (etablissement_id, eleve_id, date_entree DESC);

-- Statut dérivé de l'élève à un jour : `scolarise` si une scolarisation couvre le jour ;
-- sinon `a_venir` si une scolarisation commence après (ou si l'élève n'en a aucune : fiche
-- créée avant l'entrée) ; sinon `sorti`. `archive` (durée de conservation dépassée) n'est
-- PAS dérivé : les durées relèvent du DPO et ne sont pas inventées.
CREATE OR REPLACE FUNCTION ent_eleve_statut_a(p_etab UUID, p_eleve UUID, p_jour DATE) RETURNS TEXT
LANGUAGE sql STABLE AS $$
    SELECT CASE
        WHEN EXISTS (SELECT 1 FROM ent_eleve_scolarisation s
                     WHERE s.etablissement_id = p_etab AND s.eleve_id = p_eleve
                       AND s.date_entree <= p_jour AND (s.date_sortie IS NULL OR s.date_sortie > p_jour))
            THEN 'scolarise'
        WHEN EXISTS (SELECT 1 FROM ent_eleve_scolarisation s
                     WHERE s.etablissement_id = p_etab AND s.eleve_id = p_eleve AND s.date_entree > p_jour)
            THEN 'a_venir'
        WHEN EXISTS (SELECT 1 FROM ent_eleve_scolarisation s
                     WHERE s.etablissement_id = p_etab AND s.eleve_id = p_eleve)
            THEN 'sorti'
        ELSE 'a_venir'
    END
$$;

-- ───────────────────────── Projections sur ent_eleve ─────────────────────────
ALTER TABLE ent_eleve ADD COLUMN IF NOT EXISTS statut TEXT;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_eleve_statut_chk') THEN
        ALTER TABLE ent_eleve ADD CONSTRAINT ent_eleve_statut_chk
            CHECK (statut IS NULL OR statut IN ('a_venir', 'scolarise', 'sorti', 'archive')) NOT VALID;
    END IF;
END $$;

-- Scolarisation de référence d'un élève au jour : celle qui couvre le jour, sinon la
-- prochaine, sinon la dernière.
CREATE OR REPLACE FUNCTION ent_eleve_scolarisation_reference(p_etab UUID, p_eleve UUID, p_jour DATE)
RETURNS ent_eleve_scolarisation LANGUAGE sql STABLE AS $$
    SELECT s.* FROM ent_eleve_scolarisation s
    WHERE s.etablissement_id = p_etab AND s.eleve_id = p_eleve
    ORDER BY (s.date_entree <= p_jour AND (s.date_sortie IS NULL OR s.date_sortie > p_jour)) DESC,
             (s.date_entree > p_jour) DESC,
             CASE WHEN s.date_entree > p_jour THEN s.date_entree END ASC,
             s.date_entree DESC
    LIMIT 1
$$;

-- Recalcule les projections héritées d'UN élève et renvoie (ancien statut, nouveau statut).
-- Le statut texte hérité (`actif` / `sorti` / `preinscrit`) en est dérivé ; sans aucune
-- scolarisation (fiche créée avant l'entrée), il reste tel quel.
CREATE OR REPLACE FUNCTION ent_eleve_scolarite_projeter(p_etab UUID, p_eleve UUID, p_jour DATE DEFAULT NULL)
RETURNS TABLE (ancien TEXT, nouveau TEXT) LANGUAGE plpgsql AS $$
DECLARE
    jour DATE := coalesce(p_jour, ent_jour_etablissement(p_etab));
    ancien_statut TEXT;
    nouveau_statut TEXT;
    ref ent_eleve_scolarisation;
    legacy TEXT;
BEGIN
    SELECT e.statut INTO ancien_statut FROM ent_eleve e WHERE e.etablissement_id = p_etab AND e.id = p_eleve;
    IF NOT FOUND THEN RETURN; END IF;
    nouveau_statut := ent_eleve_statut_a(p_etab, p_eleve, jour);
    ref := ent_eleve_scolarisation_reference(p_etab, p_eleve, jour);
    legacy := CASE nouveau_statut WHEN 'scolarise' THEN 'actif' WHEN 'sorti' THEN 'sorti' ELSE NULL END;
    PERFORM set_config('ent.scolarite_projection', 'on', true);
    IF ref.id IS NULL THEN
        UPDATE ent_eleve SET statut = nouveau_statut
        WHERE etablissement_id = p_etab AND id = p_eleve AND statut IS DISTINCT FROM nouveau_statut;
    ELSE
        UPDATE ent_eleve SET
            statut = nouveau_statut,
            status = coalesce(legacy, CASE WHEN status = 'sorti' THEN 'preinscrit' ELSE status END),
            date_entree = ref.date_entree,
            date_sortie = ref.date_sortie,
            motif_sortie = coalesce(ref.motif_sortie_libre, ref.motif_sortie)
        WHERE etablissement_id = p_etab AND id = p_eleve
          AND (statut IS DISTINCT FROM nouveau_statut
               OR status IS DISTINCT FROM coalesce(legacy, CASE WHEN status = 'sorti' THEN 'preinscrit' ELSE status END)
               OR date_entree IS DISTINCT FROM ref.date_entree
               OR date_sortie IS DISTINCT FROM ref.date_sortie
               OR motif_sortie IS DISTINCT FROM coalesce(ref.motif_sortie_libre, ref.motif_sortie));
    END IF;
    PERFORM set_config('ent.scolarite_projection', 'off', true);
    RETURN QUERY SELECT ancien_statut, nouveau_statut;
END $$;

-- Après toute écriture d'une scolarisation : projections recalculées.
CREATE OR REPLACE FUNCTION ent_eleve_scolarisation_apres_ecriture() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM ent_eleve_scolarite_projeter(NEW.etablissement_id, NEW.eleve_id);
    RETURN NULL;
END $$;
DROP TRIGGER IF EXISTS ent_eleve_scolarisation_projection ON ent_eleve_scolarisation;
CREATE TRIGGER ent_eleve_scolarisation_projection
    AFTER INSERT OR UPDATE ON ent_eleve_scolarisation
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_scolarisation_apres_ecriture();

-- Pas de suppression d'une scolarisation (on la clôt) ; l'identité de la ligne ne change pas.
CREATE OR REPLACE FUNCTION ent_eleve_scolarisation_garde() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'scolarisation : ne se supprime pas (la clôturer)' USING ERRCODE = '42501';
    END IF;
    IF (NEW.id, NEW.etablissement_id, NEW.eleve_id, NEW.cree_le)
       IS DISTINCT FROM (OLD.id, OLD.etablissement_id, OLD.eleve_id, OLD.cree_le) THEN
        RAISE EXCEPTION 'scolarisation : élève et identifiant immuables' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS ent_eleve_scolarisation_garde ON ent_eleve_scolarisation;
CREATE TRIGGER ent_eleve_scolarisation_garde
    BEFORE UPDATE OR DELETE ON ent_eleve_scolarisation
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_scolarisation_garde();

-- ───────────────────────── Pont des écritures héritées ─────────────────────────
-- Un ancien chemin (création, import, `sortir`, `change_status`, labo) qui écrit
-- `status`, `date_entree`, `date_sortie` ou `motif_sortie` d'`ent_eleve` est converti en
-- changement de la scolarisation la plus récente (pas d'historique par ce chemin : c'est le
-- sens des commandes ELV-08, ELV-11 et ELV-12). Les projections écrivent sous le drapeau
-- `ent.scolarite_projection`, qui coupe le pont.
CREATE OR REPLACE FUNCTION ent_eleve_scolarite_pont() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
DECLARE
    jour DATE := ent_jour_etablissement(NEW.etablissement_id);
    derniere ent_eleve_scolarisation;
    code_sortie TEXT;
BEGIN
    IF coalesce(current_setting('ent.scolarite_projection', true), '') = 'on' THEN
        RETURN NULL;
    END IF;
    SELECT * INTO derniere FROM ent_eleve_scolarisation s
    WHERE s.etablissement_id = NEW.etablissement_id AND s.eleve_id = NEW.id
    ORDER BY s.date_entree DESC LIMIT 1;
    IF TG_OP = 'INSERT' THEN
        IF NEW.status = 'preinscrit' AND NEW.date_entree IS NULL THEN
            PERFORM ent_eleve_scolarite_projeter(NEW.etablissement_id, NEW.id);
            RETURN NULL;
        END IF;
        INSERT INTO ent_eleve_scolarisation (id, etablissement_id, eleve_id, date_entree, motif_entree,
            date_sortie, motif_sortie, motif_sortie_libre, source, decide_par)
        VALUES (gen_random_uuid(), NEW.etablissement_id, NEW.id,
            coalesce(NEW.date_entree, NEW.created_at::date, jour),
            'premiere_inscription',
            CASE WHEN NEW.status = 'sorti' OR NEW.date_sortie IS NOT NULL
                 THEN greatest(coalesce(NEW.date_sortie, jour), coalesce(NEW.date_entree, NEW.created_at::date, jour)) END,
            CASE WHEN NEW.status = 'sorti' OR NEW.date_sortie IS NOT NULL THEN 'autre' END,
            CASE WHEN NEW.status = 'sorti' OR NEW.date_sortie IS NOT NULL THEN left(NEW.motif_sortie, 500) END,
            'heritage', nullif(current_setting('ent.acteur_id', true), '')::uuid);
        RETURN NULL;
    END IF;
    -- UPDATE
    IF derniere.id IS NULL THEN
        IF NEW.status <> 'preinscrit' OR NEW.date_entree IS NOT NULL THEN
            INSERT INTO ent_eleve_scolarisation (id, etablissement_id, eleve_id, date_entree, motif_entree, source)
            VALUES (gen_random_uuid(), NEW.etablissement_id, NEW.id,
                    coalesce(NEW.date_entree, jour), 'premiere_inscription', 'heritage');
        END IF;
        RETURN NULL;
    END IF;
    IF NEW.date_entree IS NOT NULL AND NEW.date_entree IS DISTINCT FROM OLD.date_entree
       AND NEW.date_entree IS DISTINCT FROM derniere.date_entree THEN
        UPDATE ent_eleve_scolarisation SET date_entree = NEW.date_entree
        WHERE id = derniere.id AND etablissement_id = NEW.etablissement_id;
    END IF;
    IF NEW.date_sortie IS DISTINCT FROM OLD.date_sortie THEN
        IF NEW.date_sortie IS NULL THEN
            -- Sortie annulée par l'ancien chemin : la scolarisation est rouverte.
            UPDATE ent_eleve_scolarisation SET date_sortie = NULL, motif_sortie = NULL, motif_sortie_libre = NULL,
                destination_uai = NULL, destination_libelle = NULL, destination_type = NULL
            WHERE id = derniere.id AND etablissement_id = NEW.etablissement_id;
        ELSE
            code_sortie := CASE WHEN NEW.motif_sortie IN ('fin_de_cycle', 'changement_etablissement', 'demenagement',
                'exclusion_definitive', 'abandon', 'fin_obligation', 'instruction_famille', 'etranger', 'deces')
                THEN NEW.motif_sortie ELSE 'autre' END;
            UPDATE ent_eleve_scolarisation SET
                date_sortie = greatest(NEW.date_sortie, date_entree),
                motif_sortie = coalesce(code_sortie, 'autre'),
                motif_sortie_libre = CASE WHEN code_sortie = 'autre' THEN left(NEW.motif_sortie, 500) END
            WHERE id = derniere.id AND etablissement_id = NEW.etablissement_id;
        END IF;
    ELSIF NEW.motif_sortie IS DISTINCT FROM OLD.motif_sortie AND derniere.date_sortie IS NOT NULL THEN
        code_sortie := CASE WHEN NEW.motif_sortie IN ('fin_de_cycle', 'changement_etablissement', 'demenagement',
            'exclusion_definitive', 'abandon', 'fin_obligation', 'instruction_famille', 'etranger', 'deces')
            THEN NEW.motif_sortie ELSE 'autre' END;
        UPDATE ent_eleve_scolarisation SET motif_sortie = coalesce(code_sortie, 'autre'),
            motif_sortie_libre = CASE WHEN code_sortie = 'autre' THEN left(NEW.motif_sortie, 500) END
        WHERE id = derniere.id AND etablissement_id = NEW.etablissement_id;
    END IF;
    IF NEW.status IS DISTINCT FROM OLD.status THEN
        IF NEW.status = 'sorti' AND derniere.date_sortie IS NULL AND NEW.date_sortie IS NOT DISTINCT FROM OLD.date_sortie THEN
            -- Bascule de statut vers « sorti » sans date : sortie du jour.
            UPDATE ent_eleve_scolarisation SET date_sortie = greatest(jour, date_entree),
                motif_sortie = coalesce(motif_sortie, 'autre')
            WHERE id = derniere.id AND etablissement_id = NEW.etablissement_id;
        ELSIF NEW.status <> 'sorti' AND OLD.status = 'sorti' AND derniere.date_sortie IS NOT NULL
              AND NEW.date_sortie IS NOT DISTINCT FROM OLD.date_sortie THEN
            -- Réactivation par l'ancien chemin : la dernière scolarisation est rouverte.
            UPDATE ent_eleve_scolarisation SET date_sortie = NULL, motif_sortie = NULL, motif_sortie_libre = NULL,
                destination_uai = NULL, destination_libelle = NULL, destination_type = NULL
            WHERE id = derniere.id AND etablissement_id = NEW.etablissement_id;
        END IF;
    END IF;
    PERFORM ent_eleve_scolarite_projeter(NEW.etablissement_id, NEW.id);
    RETURN NULL;
END $$;

DROP TRIGGER IF EXISTS ent_eleve_scolarite_pont ON ent_eleve;
CREATE TRIGGER ent_eleve_scolarite_pont
    AFTER INSERT OR UPDATE OF status, date_entree, date_sortie, motif_sortie ON ent_eleve
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_scolarite_pont();

-- ───────────────────────── Reprise de l'existant (idempotente) ─────────────────────────
-- Une scolarisation par élève sans scolarisation : entrée = `date_entree`, sinon la date de
-- création ; sortie = `date_sortie` (ou, si le statut hérité est « sorti » sans date, le jour
-- de dernière modification) ; motif repris tel quel s'il est un code du logiciel, sinon
-- `autre` avec le texte d'origine conservé. Un élève « préinscrit » sans date d'entrée n'a pas
-- encore de scolarisation (statut « à venir »). Les écarts entre le statut hérité et le
-- statut dérivé sont rangés dans `ent_eleve_scolarite_ecart` (rien n'est supprimé).
CREATE TABLE IF NOT EXISTS ent_eleve_scolarite_ecart (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    etablissement_id UUID NOT NULL,
    eleve_id         UUID NOT NULL,
    cause            TEXT NOT NULL,
    statut_herite    TEXT,
    statut_derive    TEXT,
    detecte_le       TIMESTAMPTZ NOT NULL DEFAULT now(),
    traite_le        TIMESTAMPTZ,
    UNIQUE (eleve_id, cause)
);
ALTER TABLE ent_eleve_scolarite_ecart ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_eleve_scolarite_ecart FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_eleve_scolarite_ecart;
CREATE POLICY ent_rls_etablissement ON ent_eleve_scolarite_ecart FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));

CREATE OR REPLACE FUNCTION ent_eleve_scolarisation_reprendre() RETURNS BIGINT
LANGUAGE plpgsql AS $$
DECLARE n BIGINT := 0; e RECORD; entree DATE; sortie DATE; code TEXT; jour DATE;
BEGIN
    FOR e IN SELECT * FROM ent_eleve x
             WHERE NOT EXISTS (SELECT 1 FROM ent_eleve_scolarisation s
                               WHERE s.etablissement_id = x.etablissement_id AND s.eleve_id = x.id)
               AND NOT (x.status = 'preinscrit' AND x.date_entree IS NULL AND x.date_sortie IS NULL)
             ORDER BY x.etablissement_id, x.created_at, x.id
    LOOP
        jour := ent_jour_etablissement(e.etablissement_id);
        entree := coalesce(e.date_entree, e.created_at::date);
        sortie := e.date_sortie;
        IF sortie IS NULL AND e.status = 'sorti' THEN
            sortie := greatest(e.updated_at::date, entree);
            INSERT INTO ent_eleve_scolarite_ecart (etablissement_id, eleve_id, cause, statut_herite, statut_derive)
            VALUES (e.etablissement_id, e.id, 'sortie_sans_date', e.status, 'sorti') ON CONFLICT DO NOTHING;
        END IF;
        IF sortie IS NOT NULL AND sortie < entree THEN
            INSERT INTO ent_eleve_scolarite_ecart (etablissement_id, eleve_id, cause, statut_herite, statut_derive)
            VALUES (e.etablissement_id, e.id, 'sortie_avant_entree', e.status, NULL) ON CONFLICT DO NOTHING;
            sortie := entree;
        END IF;
        code := CASE WHEN e.motif_sortie IN ('fin_de_cycle', 'changement_etablissement', 'demenagement',
            'exclusion_definitive', 'abandon', 'fin_obligation', 'instruction_famille', 'etranger', 'deces')
            THEN e.motif_sortie ELSE 'autre' END;
        INSERT INTO ent_eleve_scolarisation (id, etablissement_id, eleve_id, date_entree, motif_entree,
            date_sortie, motif_sortie, motif_sortie_libre, source, cree_le)
        VALUES (gen_random_uuid(), e.etablissement_id, e.id, entree, 'reprise', sortie,
            CASE WHEN sortie IS NOT NULL THEN code END,
            CASE WHEN sortie IS NOT NULL AND code = 'autre' THEN left(e.motif_sortie, 500) END,
            'reprise', now());
        n := n + 1;
    END LOOP;
    -- Écarts entre statut hérité et statut dérivé (après la reprise, au jour de l'établissement).
    INSERT INTO ent_eleve_scolarite_ecart (etablissement_id, eleve_id, cause, statut_herite, statut_derive)
    SELECT x.etablissement_id, x.id, 'statut_herite_different', x.status,
           ent_eleve_statut_a(x.etablissement_id, x.id, ent_jour_etablissement(x.etablissement_id))
    FROM ent_eleve x
    WHERE x.statut IS NULL
      AND x.status IS DISTINCT FROM CASE ent_eleve_statut_a(x.etablissement_id, x.id, ent_jour_etablissement(x.etablissement_id))
              WHEN 'scolarise' THEN 'actif' WHEN 'sorti' THEN 'sorti' ELSE 'preinscrit' END
    ON CONFLICT DO NOTHING;
    -- Projections de tous les élèves repris (statut dérivé mis en cache).
    PERFORM ent_eleve_scolarite_projeter(x.etablissement_id, x.id)
    FROM ent_eleve x WHERE x.statut IS NULL;
    RETURN n;
END $$;

SELECT ent_eleve_scolarisation_reprendre();

-- Contraintes de valeur validées si les données existantes les respectent.
DO $$
BEGIN
    BEGIN
        ALTER TABLE ent_eleve_scolarisation VALIDATE CONSTRAINT ent_eleve_scolarisation_regles_chk;
    EXCEPTION WHEN check_violation THEN NULL;
    END;
    BEGIN
        ALTER TABLE ent_eleve VALIDATE CONSTRAINT ent_eleve_statut_chk;
    EXCEPTION WHEN check_violation THEN NULL;
    END;
END $$;

-- Recalcul quotidien (tâche nocturne) : élèves dont le statut dérivé a changé avec la
-- date (entrée atteinte, sortie atteinte). Renvoie (établissement, élève, ancien, nouveau)
-- pour que l'application publie `eleve.status_changed`. Idempotente.
CREATE OR REPLACE FUNCTION ent_eleve_statut_recalculer() RETURNS TABLE (
    etablissement_id UUID, eleve_id UUID, ancien TEXT, nouveau TEXT)
LANGUAGE plpgsql AS $$
DECLARE r RECORD; p RECORD;
BEGIN
    FOR r IN SELECT e.etablissement_id AS etab, e.id AS eleve FROM ent_eleve e
             WHERE e.statut IS DISTINCT FROM
                   ent_eleve_statut_a(e.etablissement_id, e.id, ent_jour_etablissement(e.etablissement_id))
    LOOP
        SELECT * INTO p FROM ent_eleve_scolarite_projeter(r.etab, r.eleve);
        etablissement_id := r.etab; eleve_id := r.eleve; ancien := p.ancien; nouveau := p.nouveau;
        RETURN NEXT;
    END LOOP;
END $$;

-- ───────────────────────── Isolation par établissement (RLS, mode permissif) ─────────────────────────
ALTER TABLE ent_eleve_scolarisation ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_eleve_scolarisation FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_eleve_scolarisation;
CREATE POLICY ent_rls_etablissement ON ent_eleve_scolarisation FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));
