-- Socle 2 (l'élève), lot E4 : capacité juridique, majorité, mobilité, modalités de
-- scolarisation, dates autorisées de changement de régime.
--
-- Additif, idempotent, rejoué à chaque démarrage.

-- ───────────────────────── Capacité juridique (§7.5, ELV-13) ─────────────────────────
-- Déclarée avec une pièce OBLIGATOIRE (jugement, acte). La majorité n'est jamais saisie : elle
-- se calcule depuis la date de naissance (fonction ci-dessous).
CREATE TABLE IF NOT EXISTS ent_eleve_capacite (
    id                 UUID PRIMARY KEY,
    etablissement_id   UUID NOT NULL,
    eleve_id           UUID NOT NULL,
    nature             TEXT NOT NULL,
    du                 DATE NOT NULL,
    au                 DATE,
    piece_document_id  UUID NOT NULL,
    motif              TEXT,
    saisi_par          UUID,
    saisi_le           TIMESTAMPTZ NOT NULL DEFAULT now(),
    levee_par          UUID,
    levee_le           TIMESTAMPTZ,
    CONSTRAINT ent_eleve_capacite_eleve_fk
        FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_eleve_capacite_regles_chk CHECK (
        nature IN ('emancipation', 'tutelle', 'curatelle', 'sauvegarde_justice', 'habilitation_familiale')
        AND (au IS NULL OR au >= du)
        AND (motif IS NULL OR char_length(motif) <= 500)
    )
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_capacite_active_uniq
    ON ent_eleve_capacite (eleve_id, nature) WHERE levee_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_eleve_capacite_eleve_idx ON ent_eleve_capacite (etablissement_id, eleve_id);

-- Une mesure ne se supprime pas et ne se réécrit pas : seule sa levée se pose, une fois.
CREATE OR REPLACE FUNCTION ent_eleve_capacite_garde() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'capacité : une mesure ne se supprime pas (la lever)' USING ERRCODE = '42501';
    END IF;
    IF OLD.levee_le IS NULL AND NEW.levee_le IS NOT NULL
       AND (to_jsonb(NEW) - 'levee_le' - 'levee_par' - 'au') = (to_jsonb(OLD) - 'levee_le' - 'levee_par' - 'au') THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'capacité : une mesure est immuable (déclarer ou lever)' USING ERRCODE = '42501';
END $$;
DROP TRIGGER IF EXISTS ent_eleve_capacite_garde ON ent_eleve_capacite;
CREATE TRIGGER ent_eleve_capacite_garde
    BEFORE UPDATE OR DELETE ON ent_eleve_capacite
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_capacite_garde();

-- ───────────────────────── Majorité (ELV-14, ELV-39) ─────────────────────────
-- Jour des 18 ans d'après la date de naissance RETENUE : calculé seulement si la précision est
-- « jour » et si la date n'est pas contestée (R-E1). Un 29 février donne le 28 février des
-- années non bissextiles [H : à valider juridiquement].
CREATE OR REPLACE FUNCTION ent_eleve_majorite_le(p_naissance DATE, p_precision TEXT, p_statut TEXT) RETURNS DATE
LANGUAGE sql IMMUTABLE AS $$
    SELECT CASE
        WHEN p_naissance IS NULL OR coalesce(p_precision, 'jour') <> 'jour' OR coalesce(p_statut, 'declaree') = 'contestee'
            THEN NULL
        ELSE (p_naissance + interval '18 years')::date
    END
$$;

-- Marqueur d'un événement de majorité déjà traité (un seul par élève et par sorte).
CREATE TABLE IF NOT EXISTS ent_eleve_majorite_evenement (
    etablissement_id UUID NOT NULL,
    eleve_id         UUID NOT NULL,
    sorte            TEXT NOT NULL CHECK (sorte IN ('proche', 'atteinte')),
    majorite_le      DATE NOT NULL,
    traite_le        TIMESTAMPTZ NOT NULL DEFAULT now(),
    publie           BOOLEAN NOT NULL,
    PRIMARY KEY (eleve_id, sorte),
    CONSTRAINT ent_eleve_majorite_evenement_eleve_fk
        FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id)
);
ALTER TABLE ent_eleve_majorite_evenement ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_eleve_majorite_evenement FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_eleve_majorite_evenement;
CREATE POLICY ent_rls_etablissement ON ent_eleve_majorite_evenement FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));

-- Élèves scolarisés pour lesquels un événement de majorité reste à traiter, au jour de leur
-- établissement : `atteinte` (le jour des 18 ans est atteint) et `proche` (dans les 30 jours).
-- Lecture seule : l'application pose le marqueur et publie dans UNE transaction par élève.
CREATE OR REPLACE FUNCTION ent_eleve_majorites_a_traiter()
RETURNS TABLE (etablissement_id UUID, eleve_id UUID, sorte TEXT, majorite_le DATE, jour DATE, presumee BOOLEAN)
LANGUAGE sql STABLE AS $$
    SELECT e.etablissement_id, e.id,
           CASE WHEN m.majorite <= ent_jour_etablissement(e.etablissement_id) THEN 'atteinte' ELSE 'proche' END,
           m.majorite, ent_jour_etablissement(e.etablissement_id),
           coalesce(e.date_naissance_statut, 'declaree') = 'presumee'
    FROM ent_eleve e
    CROSS JOIN LATERAL (SELECT ent_eleve_majorite_le(e.date_naissance, e.date_naissance_precision, e.date_naissance_statut) AS majorite) m
    WHERE e.statut = 'scolarise' AND m.majorite IS NOT NULL
      AND m.majorite - 30 <= ent_jour_etablissement(e.etablissement_id)
      AND NOT EXISTS (
          SELECT 1 FROM ent_eleve_majorite_evenement v
          WHERE v.eleve_id = e.id
            AND v.sorte = CASE WHEN m.majorite <= ent_jour_etablissement(e.etablissement_id) THEN 'atteinte' ELSE 'proche' END)
$$;

-- ───────────────────────── Mobilité (R-E13, ELV-35) ─────────────────────────
-- Année ou trimestre à l'étranger, soins longs, CNED temporaire : la scolarisation n'est
-- PAS close, le statut reste « scolarisé ».
CREATE TABLE IF NOT EXISTS ent_eleve_mobilite (
    id                UUID PRIMARY KEY,
    etablissement_id  UUID NOT NULL,
    eleve_id          UUID NOT NULL,
    du                DATE NOT NULL,
    au                DATE,
    type              TEXT NOT NULL,
    retour_prevu      DATE,
    cree_par          UUID,
    cree_le           TIMESTAMPTZ NOT NULL DEFAULT now(),
    ferme_par         UUID,
    ferme_le          TIMESTAMPTZ,
    CONSTRAINT ent_eleve_mobilite_eleve_fk
        FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_eleve_mobilite_regles_chk CHECK (
        type IN ('etranger', 'soins', 'cned', 'autre')
        AND (au IS NULL OR au >= du)
        AND (retour_prevu IS NULL OR retour_prevu >= du)
    )
);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_eleve_mobilite_chevauchement') THEN
        ALTER TABLE ent_eleve_mobilite ADD CONSTRAINT ent_eleve_mobilite_chevauchement
            EXCLUDE USING gist (eleve_id WITH =, daterange(du, au, '[)') WITH &&);
    END IF;
END $$;
CREATE INDEX IF NOT EXISTS ent_eleve_mobilite_eleve_idx ON ent_eleve_mobilite (etablissement_id, eleve_id, du DESC);

-- ───────────────────────── Modalités de scolarisation (R-E14, ELV-36) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_eleve_modalite (
    id                 UUID PRIMARY KEY,
    etablissement_id   UUID NOT NULL,
    eleve_id           UUID NOT NULL,
    code               TEXT NOT NULL,
    du                 DATE NOT NULL,
    au                 DATE,
    source             TEXT NOT NULL DEFAULT 'saisie',
    piece_document_id  UUID,
    cree_par           UUID,
    cree_le            TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_modalite_eleve_fk
        FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_eleve_modalite_regles_chk CHECK (
        code IN ('temps_partiel', 'double_inscription_cned', 'scolarisation_partagee', 'upe2a', 'ulis', 'autre')
        AND (au IS NULL OR au >= du)
        AND source IN ('saisie', 'admission', 'import', 'reprise')
    )
);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_eleve_modalite_chevauchement') THEN
        ALTER TABLE ent_eleve_modalite ADD CONSTRAINT ent_eleve_modalite_chevauchement
            EXCLUDE USING gist (eleve_id WITH =, code WITH =, daterange(du, au, '[)') WITH &&);
    END IF;
END $$;
CREATE INDEX IF NOT EXISTS ent_eleve_modalite_eleve_idx ON ent_eleve_modalite (etablissement_id, eleve_id, du DESC);

-- ───────────────────────── Dates de changement de régime (R-E5) ─────────────────────────
-- libre (défaut) | fin_de_periode | fin_de_trimestre | liste. Un changement hors de ces dates
-- exige une dérogation motivée (motif codé), tracée. Réglage par l'administrateur : lot E5.
CREATE TABLE IF NOT EXISTS ent_param_eleve_regime (
    etablissement_id  UUID PRIMARY KEY,
    dates_changement  TEXT NOT NULL DEFAULT 'libre',
    dates             DATE[] NOT NULL DEFAULT '{}',
    modifie_le        TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_par       UUID,
    CONSTRAINT ent_param_eleve_regime_chk CHECK (
        dates_changement IN ('libre', 'fin_de_periode', 'fin_de_trimestre', 'liste')
        AND cardinality(dates) <= 60
    )
);

-- ───────────────────────── Isolation par établissement (RLS, mode permissif) ─────────────────────────
DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_eleve_capacite', 'ent_eleve_mobilite', 'ent_eleve_modalite', 'ent_param_eleve_regime']
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
