-- Socle 5, lot S2 (`docs/socles/05_STRUCTURE.md` §9.1, STR-01 à STR-04 et STR-10).
--
-- Profil de l'établissement (identité, UAI, secteur, degrés, jours ouvrés), paramètres
-- versionnés (clés connues du code, valeur typée) et journal de structure en ajout seul.
--
-- Le FUSEAU HORAIRE n'est PAS dupliqué ici : sa source unique reste
-- `ent_etablissement_parametre.fuseau` (migration 265, lue par `ent_aujourdhui_etab`). Le service
-- du profil l'écrit à cet endroit ; le port `structure_fuseau` le relit. Un seul endroit, donc
-- aucune divergence possible entre « fuseau du profil » et « fuseau des dates métier ».
--
-- Migration additive et rejouable.

CREATE TABLE IF NOT EXISTS ent_etablissement_profil (
    etablissement_id UUID PRIMARY KEY
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nom_officiel TEXT NOT NULL CHECK (length(btrim(nom_officiel)) BETWEEN 1 AND 200),
    nom_usage TEXT CHECK (nom_usage IS NULL OR length(btrim(nom_usage)) BETWEEN 1 AND 200),
    -- Logo : document du socle 7 (pas de clé étrangère tant que ce socle n'existe pas).
    logo_document_id UUID,
    -- UAI (ex-RNE) : 7 chiffres et une lettre. Format seulement : l'existence n'est pas vérifiée.
    uai TEXT CHECK (uai IS NULL OR uai ~ '^[0-9]{7}[A-Z]$'),
    secteur TEXT NOT NULL DEFAULT 'public'
        CHECK (secteur IN ('public', 'prive_sous_contrat', 'prive_hors_contrat')),
    -- Contrat d'association ou simple : seulement pour le privé sous contrat.
    contrat_type TEXT CHECK (contrat_type IS NULL
        OR (secteur = 'prive_sous_contrat' AND contrat_type IN ('association', 'simple'))),
    degres TEXT[] NOT NULL DEFAULT '{}'
        CHECK (degres <@ ARRAY['premier', 'second', 'post_bac']::TEXT[]),
    -- Types d'établissement : préréglages d'affichage, aucune logique n'en dépend.
    types TEXT[] NOT NULL DEFAULT '{}' CHECK (cardinality(types) <= 12),
    academie_code TEXT CHECK (academie_code IS NULL OR length(btrim(academie_code)) BETWEEN 1 AND 16),
    departement_code TEXT CHECK (departement_code IS NULL OR departement_code ~ '^[0-9A-B]{2,3}$'),
    zone_calendrier TEXT CHECK (zone_calendrier IS NULL OR length(btrim(zone_calendrier)) BETWEEN 1 AND 32),
    regime_local TEXT CHECK (regime_local IS NULL OR regime_local IN ('alsace_moselle')),
    tutelle_libelle TEXT CHECK (tutelle_libelle IS NULL OR length(tutelle_libelle) <= 200),
    adresse TEXT CHECK (adresse IS NULL OR length(adresse) <= 400),
    telephone TEXT CHECK (telephone IS NULL OR length(telephone) <= 32),
    courriel_contact TEXT CHECK (courriel_contact IS NULL OR length(courriel_contact) <= 254),
    -- {lun:[matin,apres_midi], mer:[matin], …} ; clés et valeurs vérifiées par le service.
    jours_ouvres JSONB NOT NULL DEFAULT
        '{"lun":["matin","apres_midi"],"mar":["matin","apres_midi"],"mer":["matin"],"jeu":["matin","apres_midi"],"ven":["matin","apres_midi"]}'::jsonb
        CHECK (jsonb_typeof(jours_ouvres) = 'object'),
    -- Profil choisi au départ (§5.6) : information seulement ; non NULL = préréglages déjà appliqués.
    profil_preregle TEXT CHECK (profil_preregle IS NULL OR length(profil_preregle) <= 64),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    modifie_par UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Paramètres enregistrés : clé connue du registre du code, valeur JSON validée par le service.
CREATE TABLE IF NOT EXISTS ent_parametre (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    cle TEXT NOT NULL CHECK (cle ~ '^[a-z0-9_]+(\.[a-z0-9_]+)+$' AND length(cle) <= 80),
    valeur JSONB NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    socle_proprietaire SMALLINT NOT NULL CHECK (socle_proprietaire BETWEEN 1 AND 99),
    modifie_par UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, cle)
);

-- Journal de structure en AJOUT SEUL : profil, paramètres, sites, bâtiments, salles, lieux.
-- `avant` / `apres` portent les champs métier modifiés (jamais de secret : aucun n'existe ici).
CREATE TABLE IF NOT EXISTS ent_structure_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    objet TEXT NOT NULL CHECK (objet IN
        ('profil', 'parametre', 'fuseau', 'site', 'batiment', 'salle', 'lieu', 'disponibilite',
         'indisponibilite', 'import_salles', 'prereglage')),
    objet_ref TEXT NOT NULL CHECK (length(objet_ref) <= 120),
    action TEXT NOT NULL CHECK (length(action) BETWEEN 1 AND 40),
    avant JSONB,
    apres JSONB,
    par UUID,
    le TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_structure_journal_objet_idx
    ON ent_structure_journal (etablissement_id, objet, objet_ref, le DESC);

CREATE OR REPLACE FUNCTION ent_structure_journal_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'ent_structure_journal est en ajout seul' USING ERRCODE = '42501';
END
$$;
DROP TRIGGER IF EXISTS ent_structure_journal_immuable ON ent_structure_journal;
CREATE TRIGGER ent_structure_journal_immuable
    BEFORE UPDATE OR DELETE ON ent_structure_journal
    FOR EACH ROW EXECUTE FUNCTION ent_structure_journal_ajout_seul();

-- RLS : cloisonnement par établissement, même politique que les autres tables du socle.
DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_etablissement_profil', 'ent_parametre', 'ent_structure_journal'] LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('DROP POLICY IF EXISTS ent_rls_etablissement ON %I', t);
        EXECUTE format(
            'CREATE POLICY ent_rls_etablissement ON %I FOR ALL
                USING (etablissement_id = (SELECT ent_rls_etablissement())
                    OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
                WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
                    OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))', t);
    END LOOP;
END
$$;
