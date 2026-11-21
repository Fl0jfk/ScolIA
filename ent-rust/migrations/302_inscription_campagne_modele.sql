-- Lot I2 (socle 3, `docs/socles/03_INSCRIPTIONS.md` §8.1, §16 M3) — modèle des campagnes.
--
-- Additif et rejouable : aucune colonne existante n'est modifiée ni supprimée. Les dates restent
-- des DATE (`date_ouverture`, `date_cloture`) ; `annee_cible` (texte) est gardé en lecture le
-- temps de migrer les lecteurs (M12).
--
-- * `annee_id` : l'année du référentiel (S2-2) ; reprise par libellé quand elle existe. Une
--   année qui n'existe pas encore dans le référentiel n'est PAS créée par la reprise (c'est la
--   préparation de l'année du référentiel qui la crée) : la campagne est rangée dans le rapport
--   de reprise (`ent_inscription_reprise_rapport`) et le service la résout à la prochaine
--   écriture.
-- * Natures : `affectes` (élèves affectés par l'administration) et `en_cours_annee` (arrivées en
--   cours d'année) s'ajoutent à `admission` et `reinscription` ; états de campagne étendus.
-- * Règles paramétrables de la campagne (§8.1) : accord des responsables, décision requise,
--   motif communiqué, mode de réinscription, délais, relances, délai d'annulation d'une
--   validation (R-I11, bornes 0 à 168 h).
-- * Chaque modification d'une campagne après sa création écrit une **version** (journal).

-- Rapport de reprise commun à tout le lot : ce que la reprise n'a pas pu rapprocher (rien
-- n'échoue sur des données imprévues, elles sont rangées « à traiter »).
CREATE TABLE IF NOT EXISTS ent_inscription_reprise_rapport (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    objet TEXT NOT NULL CHECK (objet IN ('campagne', 'offre', 'dossier', 'voeu', 'responsable')),
    objet_id UUID NOT NULL,
    cause TEXT NOT NULL CHECK (char_length(cause) BETWEEN 1 AND 80),
    detecte_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    traite_le TIMESTAMPTZ,
    CONSTRAINT ent_inscription_reprise_rapport_uq UNIQUE (objet, objet_id, cause)
);
CREATE INDEX IF NOT EXISTS ent_inscription_reprise_rapport_etab_idx
    ON ent_inscription_reprise_rapport (etablissement_id, objet) WHERE traite_le IS NULL;

ALTER TABLE ent_inscription_campagne
    ADD COLUMN IF NOT EXISTS annee_id UUID REFERENCES ent_annee_scolaire (id),
    ADD COLUMN IF NOT EXISTS site_id UUID REFERENCES ent_site (id),
    ADD COLUMN IF NOT EXISTS date_cloture_depot DATE,
    ADD COLUMN IF NOT EXISTS mode_reinscription TEXT,
    ADD COLUMN IF NOT EXISTS date_limite_famille DATE,
    ADD COLUMN IF NOT EXISTS delai_reponse_jours SMALLINT,
    ADD COLUMN IF NOT EXISTS accord_responsables TEXT NOT NULL DEFAULT 'un_seul',
    ADD COLUMN IF NOT EXISTS decision_requise BOOLEAN NOT NULL DEFAULT true,
    ADD COLUMN IF NOT EXISTS motif_communique TEXT NOT NULL DEFAULT 'facultatif',
    ADD COLUMN IF NOT EXISTS regle_relance JSONB NOT NULL
        DEFAULT '{"intervalle_jours":3,"max":3,"heures_calmes":true}'::jsonb,
    ADD COLUMN IF NOT EXISTS page_publique BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS au_fil_de_l_eau BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS delai_annulation_validation_h SMALLINT NOT NULL DEFAULT 72,
    ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1;

DO $$
DECLARE
    c TEXT;
BEGIN
    -- Natures et états étendus (le nom des contraintes d'origine est celui donné par PostgreSQL).
    ALTER TABLE ent_inscription_campagne DROP CONSTRAINT IF EXISTS ent_inscription_campagne_nature_check;
    ALTER TABLE ent_inscription_campagne DROP CONSTRAINT IF EXISTS ent_inscription_campagne_nature_i2_chk;
    ALTER TABLE ent_inscription_campagne
        ADD CONSTRAINT ent_inscription_campagne_nature_i2_chk
        CHECK (nature IN ('admission', 'affectes', 'reinscription', 'en_cours_annee'));
    ALTER TABLE ent_inscription_campagne DROP CONSTRAINT IF EXISTS ent_inscription_campagne_statut_check;
    ALTER TABLE ent_inscription_campagne DROP CONSTRAINT IF EXISTS ent_inscription_campagne_statut_i2_chk;
    ALTER TABLE ent_inscription_campagne
        ADD CONSTRAINT ent_inscription_campagne_statut_i2_chk
        CHECK (statut IN ('brouillon', 'planifiee', 'ouverte', 'suspendue', 'fermee', 'archivee'));
    -- Règles paramétrables (valeurs du logiciel, bornes du logiciel).
    FOR c IN SELECT conname FROM pg_constraint
              WHERE conrelid = 'ent_inscription_campagne'::regclass AND conname LIKE 'ent_inscription_campagne_regles_i2%'
    LOOP
        EXECUTE format('ALTER TABLE ent_inscription_campagne DROP CONSTRAINT %I', c);
    END LOOP;
    ALTER TABLE ent_inscription_campagne
        ADD CONSTRAINT ent_inscription_campagne_regles_i2_chk
        CHECK (
            (mode_reinscription IS NULL OR mode_reinscription IN ('tacite', 'explicite', 'explicite_avec_acompte'))
            AND accord_responsables IN ('un_seul', 'tous_les_titulaires')
            AND motif_communique IN ('jamais', 'facultatif', 'toujours')
            AND (delai_reponse_jours IS NULL OR delai_reponse_jours BETWEEN 2 AND 30)
            AND delai_annulation_validation_h BETWEEN 0 AND 168
            AND version >= 1
            AND (date_cloture_depot IS NULL OR date_ouverture IS NULL OR date_cloture_depot >= date_ouverture)
        );
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_campagne_etab_id_uq') THEN
        ALTER TABLE ent_inscription_campagne
            ADD CONSTRAINT ent_inscription_campagne_etab_id_uq UNIQUE (etablissement_id, id);
    END IF;
END $$;

-- Une seule campagne ouverte par (établissement, nature, année, site) : l'index d'origine ne
-- connaissait pas le site.
DROP INDEX IF EXISTS ent_inscription_campagne_ouverte_uq;
CREATE UNIQUE INDEX IF NOT EXISTS ent_inscription_campagne_ouverte_site_uq
    ON ent_inscription_campagne (etablissement_id, nature, annee_cible,
                                 COALESCE(site_id, '00000000-0000-0000-0000-000000000000'::uuid))
    WHERE statut = 'ouverte';
CREATE INDEX IF NOT EXISTS ent_inscription_campagne_annee_idx
    ON ent_inscription_campagne (etablissement_id, annee_id) WHERE annee_id IS NOT NULL;

-- Campagne arrêtée pour une année du référentiel : reprise par libellé, sans rien créer.
UPDATE ent_inscription_campagne c
   SET annee_id = a.id
  FROM ent_annee_scolaire a
 WHERE c.annee_id IS NULL AND a.etablissement_id = c.etablissement_id AND a.libelle = c.annee_cible;

INSERT INTO ent_inscription_reprise_rapport (etablissement_id, objet, objet_id, cause)
SELECT c.etablissement_id, 'campagne', c.id, 'annee_absente_du_referentiel'
  FROM ent_inscription_campagne c
 WHERE c.annee_id IS NULL
ON CONFLICT DO NOTHING;

-- Réinscription ancienne (018) → mode explicite par défaut ; le public reste « tacite » par réglage.
-- Une campagne `affectes` ne décide pas (§7.2).
UPDATE ent_inscription_campagne SET decision_requise = false
 WHERE nature IN ('affectes', 'en_cours_annee', 'reinscription') AND decision_requise;
UPDATE ent_inscription_campagne SET mode_reinscription = 'explicite'
 WHERE nature = 'reinscription' AND mode_reinscription IS NULL;
-- Admission dans le privé : accord des deux titulaires ; préréglage prudent pour l'admission.
-- (Le préréglage par profil d'établissement relève du lot I13 : ici la règle la moins permissive
-- n'est PAS imposée aux campagnes existantes.)

-- Journal des versions d'une campagne (§8.1 : « toute modification après ouverture crée une
-- nouvelle version »).
CREATE TABLE IF NOT EXISTS ent_inscription_campagne_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    campagne_id UUID NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    instantane JSONB NOT NULL,
    motif TEXT CHECK (motif IS NULL OR char_length(motif) <= 500),
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_campagne_version_uq UNIQUE (campagne_id, version),
    CONSTRAINT ent_inscription_campagne_version_fk
        FOREIGN KEY (etablissement_id, campagne_id)
        REFERENCES ent_inscription_campagne (etablissement_id, id) ON DELETE CASCADE
);
