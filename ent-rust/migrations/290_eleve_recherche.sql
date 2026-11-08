-- Socle 2 (l'élève), lot E6 : recherche instantanée tolérante aux accents et aux fautes
-- (ELV-20, §7.1 `cle_recherche`, §14), marque « absorbé » d'une fusion (§7.10) et
-- confidentialité renforcée (§7.6, seulement ce dont la recherche a besoin ; le lot E7 en
-- apporte les commandes, le scellement du motif et le journal de consultation).
--
-- DEUX CLÉS DE RECHERCHE, pour ne créer aucun oracle sur l'état civil :
--   * `cle_recherche`       = désignation AFFICHÉE (nom + prénom d'usage ou d'état civil,
--                             tels qu'un lecteur de l'annuaire les voit : groupe G0) ;
--   * `cle_recherche_civil` = nom de naissance + prénoms (groupe G2), interrogeable
--                             seulement par un lecteur qui voit G2 partout.
-- Un lecteur de l'annuaire qui cherche « Yanis » ne peut donc pas découvrir que « Sam » est
-- l'élève dont le prénom d'état civil est Yanis.
--
-- Additif et idempotent. Les extensions pg_trgm et unaccent sont des modules standard de
-- PostgreSQL (« trusted » depuis la version 13).

CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS unaccent;

-- `unaccent()` est seulement STABLE : l'enveloppe IMMUTABLE permet de la figer dans une
-- colonne maintenue par déclencheur (et dans un index si besoin).
CREATE OR REPLACE FUNCTION ent_unaccent(t TEXT) RETURNS TEXT
LANGUAGE sql IMMUTABLE PARALLEL SAFE STRICT
AS $$ SELECT public.unaccent('public.unaccent'::regdictionary, t) $$;

-- Clé normalisée : sans accents, en minuscules, ponctuation et séparateurs (espace,
-- apostrophe, trait d'union) réduits à une espace. Les lettres hors alphabet latin sont
-- conservées telles quelles (écritures d'origine). UNE SEULE définition : la requête de
-- recherche et la colonne passent toutes deux par cette fonction.
CREATE OR REPLACE FUNCTION ent_cle_recherche(t TEXT) RETURNS TEXT
LANGUAGE sql IMMUTABLE PARALLEL SAFE STRICT
AS $$
    SELECT btrim(regexp_replace(
        regexp_replace(lower(ent_unaccent(t)), '[\x01-\x2f\x3a-\x40\x5b-\x60\x7b-\x7f]+', ' ', 'g'),
        '\s+', ' ', 'g'))
$$;

ALTER TABLE ent_eleve ADD COLUMN IF NOT EXISTS cle_recherche TEXT NOT NULL DEFAULT '';
ALTER TABLE ent_eleve ADD COLUMN IF NOT EXISTS cle_recherche_civil TEXT NOT NULL DEFAULT '';
-- Élève absorbé par une fusion : lecture seule, redirigé vers l'élève conservé (§7.10).
ALTER TABLE ent_eleve ADD COLUMN IF NOT EXISTS fusionne_dans UUID;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_eleve_fusionne_dans_chk') THEN
        ALTER TABLE ent_eleve ADD CONSTRAINT ent_eleve_fusionne_dans_chk
            CHECK (fusionne_dans IS NULL OR fusionne_dans <> id);
    END IF;
END $$;

-- Maintenance par la base : aucune écriture (création, import, labo, correction
-- d'identité) ne peut oublier la clé. N'écrit QUE les deux clés : la garde d'identité
-- (E2) n'y voit rien à redire.
CREATE OR REPLACE FUNCTION ent_eleve_cle_recherche_maj() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    NEW.cle_recherche := ent_cle_recherche(coalesce(NEW.nom, '') || ' ' || coalesce(NEW.prenom, ''));
    NEW.cle_recherche_civil := ent_cle_recherche(
        coalesce(NEW.nom_naissance, '') || ' ' || coalesce(array_to_string(NEW.prenoms, ' '), ''));
    RETURN NEW;
END $$;

DROP TRIGGER IF EXISTS ent_eleve_cle_recherche ON ent_eleve;
CREATE TRIGGER ent_eleve_cle_recherche
    BEFORE INSERT OR UPDATE ON ent_eleve
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_cle_recherche_maj();

-- Reprise : les élèves existants reçoivent leur clé (idempotent : seules les lignes
-- encore vides sont touchées ; le déclencheur calcule la valeur).
UPDATE ent_eleve SET cle_recherche = '' WHERE cle_recherche = '';

CREATE INDEX IF NOT EXISTS ent_eleve_cle_recherche_trgm
    ON ent_eleve USING gin (cle_recherche gin_trgm_ops);
CREATE INDEX IF NOT EXISTS ent_eleve_cle_recherche_civil_trgm
    ON ent_eleve USING gin (cle_recherche_civil gin_trgm_ops);
CREATE INDEX IF NOT EXISTS ent_eleve_absorbe_idx
    ON ent_eleve (etablissement_id, fusionne_dans) WHERE fusionne_dans IS NOT NULL;

-- ───────────────────────── Confidentialité renforcée (§7.6) ─────────────────────────
-- Le lot E6 n'a besoin que du FAIT « cet élève est protégé » pour l'exclure de la
-- recherche et des listes de doublons. Le motif est scellé (`champs_chiffres`) : jamais en
-- clair, jamais dans un événement ni un journal. Les commandes (ELV-15), le journal de
-- consultation et les autres effets sont du lot E7, qui réutilise cette table.
CREATE TABLE IF NOT EXISTS ent_eleve_confidentialite (
    id               UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id         UUID NOT NULL,
    niveau           TEXT NOT NULL DEFAULT 'renforcee',
    motif_scelle     TEXT NOT NULL,
    mesures          TEXT[] NOT NULL DEFAULT '{}',
    du               DATE NOT NULL,
    au               DATE,
    decide_par       UUID,
    decide_le        TIMESTAMPTZ NOT NULL DEFAULT now(),
    leve_par         UUID,
    leve_le          TIMESTAMPTZ,
    CONSTRAINT ent_eleve_confidentialite_niveau_chk CHECK (niveau IN ('renforcee')),
    CONSTRAINT ent_eleve_confidentialite_periode_chk CHECK (au IS NULL OR au >= du),
    FOREIGN KEY (etablissement_id, eleve_id) REFERENCES ent_eleve (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_eleve_confidentialite_eleve_idx
    ON ent_eleve_confidentialite (etablissement_id, eleve_id) WHERE leve_le IS NULL;

-- Vrai si une mesure est en vigueur à ce jour (jour de l'établissement par défaut).
CREATE OR REPLACE FUNCTION ent_eleve_confidentiel(p_etab UUID, p_eleve UUID, p_jour DATE DEFAULT NULL)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
    SELECT EXISTS (
        SELECT 1 FROM ent_eleve_confidentialite c
         WHERE c.etablissement_id = p_etab AND c.eleve_id = p_eleve
           AND c.leve_le IS NULL
           AND c.du <= coalesce(p_jour, ent_jour_etablissement(p_etab))
           AND (c.au IS NULL OR c.au > coalesce(p_jour, ent_jour_etablissement(p_etab))))
$$;

DO $$
BEGIN
    ALTER TABLE ent_eleve_confidentialite ENABLE ROW LEVEL SECURITY;
    ALTER TABLE ent_eleve_confidentialite FORCE ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS ent_rls_etablissement ON ent_eleve_confidentialite;
    CREATE POLICY ent_rls_etablissement ON ent_eleve_confidentialite FOR ALL
        USING (etablissement_id = (SELECT ent_rls_etablissement())
            OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
        WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
            OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));
END $$;
