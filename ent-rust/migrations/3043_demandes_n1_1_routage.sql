-- Socle N1 (« Demandes et flux de travail »), lot N1-1 — migration 3043 : natures activées, publics, règles de routage,
-- file de repli RH (réglage par établissement) et journal des réglages.
-- Réf. : docs/socles/N1_DEMANDES.md §8.1 (l'établissement active, ne crée ni champ ni circuit), §10.3 (règles de
-- routage), §36 (R-DM47) et §38 (D-061, Q-DM14).
--
-- Contenu : ent_dem_nature_etab, ent_dem_nature_public, ent_dem_routage_regle, ent_dem_repli_rh,
-- ent_dem_reglage_journal. Additive et rejouable (E8). Aucune donnée semi-structurée (D-036) : un public par ligne,
-- jamais un tableau. Aucune donnée de départ : les natures sont désactivées tant que l'établissement ne les active pas
-- (D-023 : peu de réglages, signalés) ; le service propose les préréglages (`initialiser_files_defaut`).

-- ───────────────────────── Natures activées par l'établissement ─────────────────────────
-- La nature est DÉCLARÉE dans le code ; ici l'établissement l'active, raccourcit ses délais (jamais ne les rallonge :
-- contrôle du service) et peut désigner la file qui la reçoit.
CREATE TABLE IF NOT EXISTS ent_dem_nature_etab (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nature_code TEXT NOT NULL CHECK (nature_code ~ '^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$' AND length(nature_code) <= 80),
    actif BOOLEAN NOT NULL DEFAULT false,
    delai_prise_jours SMALLINT CHECK (delai_prise_jours IS NULL OR delai_prise_jours BETWEEN 1 AND 90),
    delai_traitement_jours SMALLINT CHECK (delai_traitement_jours IS NULL OR delai_traitement_jours BETWEEN 1 AND 90),
    file_defaut_id UUID,
    texte_aide TEXT CHECK (texte_aide IS NULL OR length(texte_aide) <= 500),
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    maj_par_user_id UUID,
    CONSTRAINT ent_dem_nature_etab_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_nature_etab_code_uq UNIQUE (etablissement_id, nature_code),
    CONSTRAINT ent_dem_nature_etab_file_cfk FOREIGN KEY (etablissement_id, file_defaut_id)
        REFERENCES ent_dem_file (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_nature_etab_delais_chk CHECK (
        delai_prise_jours IS NULL OR delai_traitement_jours IS NULL OR delai_traitement_jours >= delai_prise_jours)
);

-- Publics ouverts pour la nature dans l'établissement : une ligne par public, parmi ceux que la nature autorise.
CREATE TABLE IF NOT EXISTS ent_dem_nature_public (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nature_code TEXT NOT NULL,
    public_code TEXT NOT NULL CHECK (public_code IN ('personnel', 'responsable', 'eleve_majeur', 'eleve_mineur')),
    PRIMARY KEY (etablissement_id, nature_code, public_code),
    CONSTRAINT ent_dem_nature_public_nature_cfk FOREIGN KEY (etablissement_id, nature_code)
        REFERENCES ent_dem_nature_etab (etablissement_id, nature_code) ON DELETE RESTRICT
);

-- ───────────────────────── Règles de routage ─────────────────────────
-- Une règle = (nature, critères, destination, ordre, période). La première qui s'applique gagne ; à défaut, la
-- destination par défaut de la nature ; à défaut, la file de tri du site (CDM-02) ; jamais nulle part. L'IA n'affecte
-- rien (NJ-87) : tous les critères sont des valeurs structurées, jamais le texte libre. La destination est TOUJOURS
-- une file : « une personne désignée » s'exprime par une file dont une règle de membres désigne cet agent, daté.
CREATE TABLE IF NOT EXISTS ent_dem_routage_regle (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nature_code TEXT NOT NULL CHECK (nature_code ~ '^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$' AND length(nature_code) <= 80),
    -- Critères : NULL = « peu importe ».
    categorie_code TEXT CHECK (categorie_code IS NULL OR categorie_code ~ '^[a-z][a-z0-9_]{0,59}$'),
    site_id UUID,
    niveau_code TEXT CHECK (niveau_code IS NULL OR length(btrim(niveau_code)) BETWEEN 1 AND 40),
    public_code TEXT CHECK (public_code IS NULL OR public_code IN ('personnel', 'responsable', 'eleve_majeur', 'eleve_mineur')),
    urgence TEXT CHECK (urgence IS NULL OR urgence IN ('normale', 'haute', 'securite')),
    destination_file_id UUID NOT NULL,
    ordre SMALLINT NOT NULL DEFAULT 10 CHECK (ordre BETWEEN 1 AND 999),
    du DATE NOT NULL,
    au DATE,
    actif BOOLEAN NOT NULL DEFAULT true,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_dem_routage_regle_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_routage_regle_site_cfk FOREIGN KEY (etablissement_id, site_id)
        REFERENCES ent_site (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_routage_regle_file_cfk FOREIGN KEY (etablissement_id, destination_file_id)
        REFERENCES ent_dem_file (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_routage_regle_dates_chk CHECK (au IS NULL OR au >= du)
);
CREATE INDEX IF NOT EXISTS ent_dem_routage_regle_nature_idx
    ON ent_dem_routage_regle (etablissement_id, nature_code, ordre) WHERE actif;

-- ───────────────────────── File de repli RH (Q-DM14, D-061) ─────────────────────────
-- Réglage par établissement : la file (de type `repli`) qui reçoit une demande confidentielle quand la personne
-- visée est dans la file RH. Défaut posé par le service : attaché de gestion et direction. Une ligne par établissement.
CREATE TABLE IF NOT EXISTS ent_dem_repli_rh (
    etablissement_id UUID PRIMARY KEY
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    file_id UUID NOT NULL,
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    maj_par_user_id UUID,
    CONSTRAINT ent_dem_repli_rh_file_cfk FOREIGN KEY (etablissement_id, file_id)
        REFERENCES ent_dem_file (etablissement_id, id) ON DELETE RESTRICT
);

-- ───────────────────────── Journal des réglages (ajout seul) ─────────────────────────
-- Tout changement de réglage du guichet est tracé : qui, quand, avant, après (codes et libellés courts, jamais de
-- contenu de demande). Le changement de la file de repli RH y figure obligatoirement (Q-DM14).
CREATE TABLE IF NOT EXISTS ent_dem_reglage_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sujet TEXT NOT NULL CHECK (sujet IN (
        'repli_rh', 'nature', 'file', 'membres', 'routage', 'reponse_type', 'palier')),
    cible TEXT NOT NULL CHECK (length(cible) BETWEEN 1 AND 120),
    avant TEXT CHECK (avant IS NULL OR length(avant) <= 200),
    apres TEXT CHECK (apres IS NULL OR length(apres) <= 200),
    par_user_id UUID,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_reglage_journal_etab_id_uq UNIQUE (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_dem_reglage_journal_idx
    ON ent_dem_reglage_journal (etablissement_id, sujet, le DESC);

CREATE OR REPLACE FUNCTION ent_dem_reglage_journal_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'ent_dem_reglage_journal : ajout seul, ni modification ni suppression (R-DM28)'
        USING ERRCODE = '23001';
END
$$;
DROP TRIGGER IF EXISTS ent_dem_reglage_journal_ajout_seul_trg ON ent_dem_reglage_journal;
CREATE TRIGGER ent_dem_reglage_journal_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON ent_dem_reglage_journal
    FOR EACH ROW EXECUTE FUNCTION ent_dem_reglage_journal_ajout_seul();

-- ───────────────────────── Cloisonnement par établissement ─────────────────────────
SELECT ent_rls_poser(ARRAY[
    'ent_dem_nature_etab',
    'ent_dem_nature_public',
    'ent_dem_routage_regle',
    'ent_dem_repli_rh',
    'ent_dem_reglage_journal'
]);
