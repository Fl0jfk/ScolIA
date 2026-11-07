-- Socle 2 (l'élève), lot E3 / étape M3 : identifiants externes de l'élève (INE et autres)
-- avec statut et provenance.
--
-- Un identifiant est un attribut de l'élève DANS L'ÉTABLISSEMENT : l'unicité ne joue
-- qu'à l'intérieur d'un établissement (D-013 : aucune vérification entre établissements,
-- un établissement ne doit jamais apprendre qu'un INE existe ailleurs). Une collision
-- n'échoue pas en silence : la seconde valeur est enregistrée `conteste` (souvent le
-- signe d'un doublon d'élève).
--
-- Les colonnes héritées `ent_eleve.ine` et `ent_eleve.numero_badge` restent des
-- PROJECTIONS (cf. §15, M3) : maintenues par la base depuis cette table ; toute écriture
-- héritée de ces colonnes est convertie en identifiant (pont ci-dessous), rien n'est perdu.
--
-- Additif, idempotent, rejoué à chaque démarrage. Numéros 281-289 : lots E3 et E4.

-- ───────────────────────── Référentiel des systèmes (du logiciel) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_systeme_identifiant (
    code                     TEXT PRIMARY KEY,
    libelle                  TEXT NOT NULL,
    unique_par_etab          BOOLEAN NOT NULL,
    un_seul_actif_par_eleve  BOOLEAN NOT NULL,
    lisible_par_famille      BOOLEAN NOT NULL DEFAULT false,
    exportable_vers          TEXT[] NOT NULL DEFAULT '{}'
);

INSERT INTO ent_systeme_identifiant (code, libelle, unique_par_etab, un_seul_actif_par_eleve, lisible_par_famille, exportable_vers) VALUES
    ('INE', 'Identifiant national élève', true, true, true, ARRAY['bee', 'onde', 'lsu']),
    ('SIECLE_ELEVE', 'Identifiant élève SIECLE', true, true, false, ARRAY['bee']),
    ('ONDE', 'Identifiant ONDE (premier degré)', true, true, false, ARRAY['onde']),
    ('AAF_JOINTURE', 'Clé de jointure AAF (rattachement ÉduConnect)', true, true, false, ARRAY[]::TEXT[]),
    ('BADGE', 'Badge de cantine et de passage', true, true, true, ARRAY[]::TEXT[]),
    ('ANCIEN_LOGICIEL', 'Identifiant dans l''ancien logiciel (reprise)', false, false, false, ARRAY[]::TEXT[])
ON CONFLICT (code) DO NOTHING;

-- ───────────────────────── Identifiants de l'élève ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_eleve_identifiant (
    id                    UUID PRIMARY KEY,
    etablissement_id      UUID NOT NULL,
    eleve_id              UUID NOT NULL,
    systeme               TEXT NOT NULL REFERENCES ent_systeme_identifiant (code),
    valeur                TEXT NOT NULL,
    statut                TEXT NOT NULL,
    provenance            TEXT NOT NULL,
    preuve_document_id    UUID,
    valide_du             DATE NOT NULL,
    valide_au             DATE,
    confirme_le           TIMESTAMPTZ,
    confirme_par_source   TEXT,
    motif                 TEXT,
    enregistre_le         TIMESTAMPTZ NOT NULL DEFAULT now(),
    enregistre_par        UUID,
    CONSTRAINT ent_eleve_identifiant_eleve_fk
        FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_eleve_identifiant_regles_chk CHECK (
        char_length(btrim(valeur)) BETWEEN 1 AND 64
        AND statut IN ('declare', 'a_verifier', 'confirme', 'conteste', 'revoque')
        AND provenance IN ('saisie', 'document_famille', 'ancien_etablissement', 'import_aaf', 'import_bee',
                           'retour_bee', 'import_onde', 'educonnect', 'reprise_logiciel', 'badge')
        AND (valide_au IS NULL OR valide_au >= valide_du)
        AND (statut NOT IN ('conteste', 'revoque') OR (motif IS NOT NULL AND btrim(motif) <> ''))
        AND (statut <> 'revoque' OR valide_au IS NOT NULL)
        AND (statut <> 'confirme' OR confirme_le IS NOT NULL)
        AND (motif IS NULL OR char_length(motif) <= 500)
    )
);

CREATE INDEX IF NOT EXISTS ent_eleve_identifiant_eleve_idx
    ON ent_eleve_identifiant (etablissement_id, eleve_id, systeme);

-- Unicité DANS l'établissement, jamais entre établissements (la clé commence par
-- etablissement_id). Les identifiants contestés ou révoqués ne comptent pas.
CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_identifiant_valeur_uniq
    ON ent_eleve_identifiant (etablissement_id, systeme, lower(valeur))
    WHERE statut IN ('declare', 'a_verifier', 'confirme') AND valide_au IS NULL
      AND systeme IN ('INE', 'SIECLE_ELEVE', 'ONDE', 'AAF_JOINTURE', 'BADGE');
-- Un seul identifiant actif par élève et par système.
CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_identifiant_actif_uniq
    ON ent_eleve_identifiant (eleve_id, systeme)
    WHERE statut IN ('declare', 'a_verifier', 'confirme') AND valide_au IS NULL
      AND systeme IN ('INE', 'SIECLE_ELEVE', 'ONDE', 'AAF_JOINTURE', 'BADGE');
CREATE INDEX IF NOT EXISTS ent_eleve_identifiant_conteste_idx
    ON ent_eleve_identifiant (etablissement_id, systeme) WHERE statut = 'conteste' AND valide_au IS NULL;

-- Immuabilité : l'identité de la ligne (élève, système, valeur, provenance) ne change
-- jamais ; seuls le statut, le motif, la fin de validité et la confirmation évoluent.
-- Aucune suppression (la révocation garde la ligne).
CREATE OR REPLACE FUNCTION ent_eleve_identifiant_immuable() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'identifiant : une ligne ne se supprime pas (révoquer)' USING ERRCODE = '42501';
    END IF;
    IF (NEW.id, NEW.etablissement_id, NEW.eleve_id, NEW.systeme, NEW.valeur, NEW.provenance, NEW.valide_du,
        NEW.enregistre_le, NEW.enregistre_par)
       IS DISTINCT FROM
       (OLD.id, OLD.etablissement_id, OLD.eleve_id, OLD.systeme, OLD.valeur, OLD.provenance, OLD.valide_du,
        OLD.enregistre_le, OLD.enregistre_par) THEN
        RAISE EXCEPTION 'identifiant : élève, système, valeur et provenance sont immuables'
            USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END $$;

DROP TRIGGER IF EXISTS ent_eleve_identifiant_immuable ON ent_eleve_identifiant;
CREATE TRIGGER ent_eleve_identifiant_immuable
    BEFORE UPDATE OR DELETE ON ent_eleve_identifiant
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_identifiant_immuable();

-- ───────────────────────── Projection sur ent_eleve ─────────────────────────
-- ent_eleve.ine / numero_badge = identifiant ACTIF du système ; à défaut, le plus récent
-- identifiant contesté (rien n'est perdu pour les lecteurs hérités, le statut est dans
-- cette table) ; sinon NULL. Seule cette fonction écrit ces colonnes sous le drapeau
-- `ent.identifiant_projection`.
CREATE OR REPLACE FUNCTION ent_eleve_identifiant_projeter(p_etab UUID, p_eleve UUID, p_systeme TEXT)
RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE v TEXT;
BEGIN
    IF p_systeme NOT IN ('INE', 'BADGE') THEN RETURN; END IF;
    SELECT valeur INTO v FROM ent_eleve_identifiant
    WHERE etablissement_id = p_etab AND eleve_id = p_eleve AND systeme = p_systeme
      AND statut <> 'revoque' AND valide_au IS NULL
    ORDER BY (statut = 'conteste'), enregistre_le DESC, id
    LIMIT 1;
    PERFORM set_config('ent.identifiant_projection', 'on', true);
    IF p_systeme = 'INE' THEN
        UPDATE ent_eleve SET ine = v WHERE etablissement_id = p_etab AND id = p_eleve AND ine IS DISTINCT FROM v;
    ELSE
        UPDATE ent_eleve SET numero_badge = v WHERE etablissement_id = p_etab AND id = p_eleve AND numero_badge IS DISTINCT FROM v;
    END IF;
    PERFORM set_config('ent.identifiant_projection', 'off', true);
END $$;

-- Un autre élève de CET établissement porte-t-il déjà cette valeur (actif) ? Jamais de
-- recherche hors de l'établissement.
CREATE OR REPLACE FUNCTION ent_eleve_identifiant_collision(p_etab UUID, p_eleve UUID, p_systeme TEXT, p_valeur TEXT)
RETURNS UUID LANGUAGE sql STABLE AS $$
    SELECT i.eleve_id FROM ent_eleve_identifiant i
    JOIN ent_systeme_identifiant s ON s.code = i.systeme
    WHERE s.unique_par_etab AND i.etablissement_id = p_etab AND i.systeme = p_systeme
      AND lower(i.valeur) = lower(p_valeur) AND i.eleve_id <> p_eleve
      AND i.statut IN ('declare', 'a_verifier', 'confirme') AND i.valide_au IS NULL
    ORDER BY i.enregistre_le, i.id
    LIMIT 1
$$;

-- UNIQUE chemin d'enregistrement d'un identifiant (commande ELV-07, pont hérité, reprise
-- des imports) : sérialisé par valeur, collision → `conteste`, remplacement de l'actif.
-- Renvoie la ligne créée ou, si l'élève porte déjà cette valeur (non révoquée), l'existante
-- (`existant` = true : idempotent). Un identifiant CONFIRMÉ n'est jamais remplacé par une
-- valeur moins sûre sans `p_remplacer_confirme` (SQLSTATE 55000).
CREATE OR REPLACE FUNCTION ent_eleve_identifiant_enregistrer(
    p_etab UUID, p_eleve UUID, p_systeme TEXT, p_valeur TEXT, p_provenance TEXT, p_statut TEXT,
    p_valide_du DATE, p_acteur UUID, p_preuve UUID, p_remplacer_confirme BOOLEAN DEFAULT false
) RETURNS TABLE (id UUID, statut TEXT, collision_eleve UUID, remplace UUID, existant BOOLEAN)
LANGUAGE plpgsql AS $$
DECLARE
    sys ent_systeme_identifiant;
    ex ent_eleve_identifiant;
    actif ent_eleve_identifiant;
    coll UUID;
    nouveau UUID := gen_random_uuid();
    st TEXT := p_statut;
    mot TEXT;
BEGIN
    SELECT * INTO sys FROM ent_systeme_identifiant WHERE code = p_systeme;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'système d''identifiant inconnu' USING ERRCODE = '23503';
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(p_etab::text || '|' || p_systeme || '|' || lower(p_valeur), 281));
    SELECT * INTO ex FROM ent_eleve_identifiant i
    WHERE i.etablissement_id = p_etab AND i.eleve_id = p_eleve AND i.systeme = p_systeme
      AND lower(i.valeur) = lower(p_valeur) AND i.statut <> 'revoque'
    ORDER BY i.enregistre_le DESC LIMIT 1;
    IF FOUND THEN
        RETURN QUERY SELECT ex.id, ex.statut, NULL::UUID, NULL::UUID, true;
        RETURN;
    END IF;
    coll := ent_eleve_identifiant_collision(p_etab, p_eleve, p_systeme, p_valeur);
    IF coll IS NOT NULL THEN
        st := 'conteste';
        mot := 'valeur déjà portée par un autre élève de l''établissement';
    ELSIF sys.un_seul_actif_par_eleve THEN
        SELECT * INTO actif FROM ent_eleve_identifiant i
        WHERE i.etablissement_id = p_etab AND i.eleve_id = p_eleve AND i.systeme = p_systeme
          AND i.statut IN ('declare', 'a_verifier', 'confirme') AND i.valide_au IS NULL;
        IF FOUND THEN
            IF actif.statut = 'confirme' AND st <> 'confirme' AND NOT p_remplacer_confirme THEN
                RAISE EXCEPTION 'un identifiant confirmé existe : le révoquer avant d''en déclarer un autre'
                    USING ERRCODE = '55000';
            END IF;
            UPDATE ent_eleve_identifiant SET statut = 'revoque', valide_au = greatest(valide_du, p_valide_du - 1),
                   motif = 'remplacé par une nouvelle valeur'
            WHERE ent_eleve_identifiant.id = actif.id;
            remplace := actif.id;
        END IF;
    END IF;
    INSERT INTO ent_eleve_identifiant (
        id, etablissement_id, eleve_id, systeme, valeur, statut, provenance, preuve_document_id,
        valide_du, confirme_le, confirme_par_source, motif, enregistre_le, enregistre_par
    ) VALUES (
        nouveau, p_etab, p_eleve, p_systeme, btrim(p_valeur), st, p_provenance, p_preuve,
        p_valide_du, CASE WHEN st = 'confirme' THEN now() END,
        CASE WHEN st = 'confirme' THEN p_provenance END, mot, now(), p_acteur
    );
    PERFORM ent_eleve_identifiant_projeter(p_etab, p_eleve, p_systeme);
    RETURN QUERY SELECT nouveau, st, coll, remplace, false;
END $$;

-- Retire (révoque) les identifiants actifs d'un système (saisie héritée vidée).
CREATE OR REPLACE FUNCTION ent_eleve_identifiant_retirer(p_etab UUID, p_eleve UUID, p_systeme TEXT, p_motif TEXT)
RETURNS VOID LANGUAGE plpgsql AS $$
BEGIN
    UPDATE ent_eleve_identifiant SET statut = 'revoque', valide_au = greatest(valide_du, current_date), motif = p_motif
    WHERE etablissement_id = p_etab AND eleve_id = p_eleve AND systeme = p_systeme
      AND statut <> 'revoque' AND valide_au IS NULL;
    PERFORM ent_eleve_identifiant_projeter(p_etab, p_eleve, p_systeme);
END $$;

-- ───────────────────────── Pont des écritures héritées ─────────────────────────
-- Tout chemin qui écrit encore `ent_eleve.ine` ou `numero_badge` (création, import,
-- inscription, labo, ancienne saisie d'identité) est converti en identifiant : INE déclaré
-- par la saisie, badge confirmé. Les projections ci-dessus écrivent sous le drapeau
-- `ent.identifiant_projection`, qui coupe le pont (pas de rebouclage).
CREATE OR REPLACE FUNCTION ent_eleve_identifiant_pont() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
DECLARE
    acteur UUID := nullif(current_setting('ent.acteur_id', true), '')::uuid;
    v TEXT;
BEGIN
    IF coalesce(current_setting('ent.identifiant_projection', true), '') = 'on' THEN
        RETURN NULL;
    END IF;
    IF TG_OP = 'INSERT' OR NEW.ine IS DISTINCT FROM OLD.ine THEN
        v := nullif(btrim(coalesce(NEW.ine, '')), '');
        IF v IS NULL THEN
            IF TG_OP = 'UPDATE' THEN
                PERFORM ent_eleve_identifiant_retirer(NEW.etablissement_id, NEW.id, 'INE', 'retiré par la saisie héritée');
            END IF;
        ELSIF char_length(v) <= 64 THEN
            PERFORM ent_eleve_identifiant_enregistrer(
                NEW.etablissement_id, NEW.id, 'INE', upper(v), 'saisie', 'declare', current_date, acteur, NULL, true);
        END IF;
    END IF;
    IF TG_OP = 'INSERT' OR NEW.numero_badge IS DISTINCT FROM OLD.numero_badge THEN
        v := nullif(btrim(coalesce(NEW.numero_badge, '')), '');
        IF v IS NULL THEN
            IF TG_OP = 'UPDATE' THEN
                PERFORM ent_eleve_identifiant_retirer(NEW.etablissement_id, NEW.id, 'BADGE', 'retiré par la saisie héritée');
            END IF;
        ELSIF char_length(v) <= 64 THEN
            PERFORM ent_eleve_identifiant_enregistrer(
                NEW.etablissement_id, NEW.id, 'BADGE', v, 'badge', 'confirme', current_date, acteur, NULL, true);
        END IF;
    END IF;
    RETURN NULL;
END $$;

DROP TRIGGER IF EXISTS ent_eleve_identifiant_pont ON ent_eleve;
CREATE TRIGGER ent_eleve_identifiant_pont
    AFTER INSERT OR UPDATE OF ine, numero_badge ON ent_eleve
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_identifiant_pont();

-- ───────────────────────── Reprise de l'existant (idempotente) ─────────────────────────
-- INE existants → `a_verifier`, provenance `reprise_logiciel` ; un même INE porté par
-- plusieurs élèves d'un établissement → TOUS `conteste` (on ne sait pas lequel est juste ;
-- l'index unique ne les voit pas) ; badges → système BADGE, `confirme`, contestés de même
-- en cas de doublon. Aucun INE n'est perdu : la colonne héritée n'est pas modifiée.
CREATE OR REPLACE FUNCTION ent_eleve_identifiant_reprendre() RETURNS BIGINT
LANGUAGE plpgsql AS $$
DECLARE n BIGINT := 0; k BIGINT;
BEGIN
    WITH src AS (
        SELECT e.etablissement_id, e.id AS eleve_id, upper(btrim(e.ine)) AS valeur,
               count(*) OVER (PARTITION BY e.etablissement_id, lower(btrim(e.ine))) AS nb
        FROM ent_eleve e
        WHERE nullif(btrim(coalesce(e.ine, '')), '') IS NOT NULL AND char_length(btrim(e.ine)) <= 64
          AND NOT EXISTS (SELECT 1 FROM ent_eleve_identifiant i
                          WHERE i.etablissement_id = e.etablissement_id AND i.eleve_id = e.id AND i.systeme = 'INE')
    )
    INSERT INTO ent_eleve_identifiant (id, etablissement_id, eleve_id, systeme, valeur, statut, provenance,
                                       valide_du, motif, enregistre_le)
    SELECT gen_random_uuid(), etablissement_id, eleve_id, 'INE', valeur,
           CASE WHEN nb > 1 THEN 'conteste' ELSE 'a_verifier' END, 'reprise_logiciel', current_date,
           CASE WHEN nb > 1 THEN 'INE porté par plusieurs élèves à la reprise' END, now()
    FROM src;
    GET DIAGNOSTICS k = ROW_COUNT; n := n + k;
    WITH src AS (
        SELECT e.etablissement_id, e.id AS eleve_id, btrim(e.numero_badge) AS valeur,
               count(*) OVER (PARTITION BY e.etablissement_id, lower(btrim(e.numero_badge))) AS nb
        FROM ent_eleve e
        WHERE nullif(btrim(coalesce(e.numero_badge, '')), '') IS NOT NULL AND char_length(btrim(e.numero_badge)) <= 64
          AND NOT EXISTS (SELECT 1 FROM ent_eleve_identifiant i
                          WHERE i.etablissement_id = e.etablissement_id AND i.eleve_id = e.id AND i.systeme = 'BADGE')
    )
    INSERT INTO ent_eleve_identifiant (id, etablissement_id, eleve_id, systeme, valeur, statut, provenance,
                                       valide_du, confirme_le, confirme_par_source, motif, enregistre_le)
    SELECT gen_random_uuid(), etablissement_id, eleve_id, 'BADGE', valeur,
           CASE WHEN nb > 1 THEN 'conteste' ELSE 'confirme' END, 'badge', current_date,
           CASE WHEN nb = 1 THEN now() END, CASE WHEN nb = 1 THEN 'badge' END,
           CASE WHEN nb > 1 THEN 'badge porté par plusieurs élèves à la reprise' END, now()
    FROM src;
    GET DIAGNOSTICS k = ROW_COUNT; n := n + k;
    RETURN n;
END $$;

SELECT ent_eleve_identifiant_reprendre();

-- ───────────────────────── Isolation par établissement (RLS, mode permissif) ─────────────────────────
ALTER TABLE ent_eleve_identifiant ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_eleve_identifiant FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_eleve_identifiant;
CREATE POLICY ent_rls_etablissement ON ent_eleve_identifiant FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));
