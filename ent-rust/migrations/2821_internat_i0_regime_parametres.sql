-- Socle 41 (internat), lots I0 / I1 — migration 2821 : régime codé, paramètres de l'internat.
-- Réf. : docs/socles/41_INTERNAT.md §24, §25.3 ; RI-04, RI-09 ; R-IT2 ; S-4100 à S-4117.
-- Additive et rejouable (E8). Aucune colonne JSON (D-036).

-- ───────────────────────── 1. Régime codé (R-IT2, RI-04) ─────────────────────────
-- Trois vocabulaires coexistent dans la base : sigles (EXT, DP, INT) de ent_sortie_regle_regime et
-- des anciennes données, codes du socle 2 (externe, demi_pension, interne : scolarité, tarifs,
-- inscriptions) et saisies libres historiques. Cette table de correspondance est la SEULE
-- référence ; la fonction serveur `RegimeCode::depuis_texte` (domain-eleve) la reproduit et un test
-- vérifie qu'elles coïncident. Aucun texte libre n'est jamais interprété ailleurs.
CREATE TABLE IF NOT EXISTS ent_regime_code (
    code TEXT PRIMARY KEY CHECK (code IN ('externe', 'demi_pensionnaire', 'interne', 'interne_externe')),
    libelle TEXT NOT NULL,
    -- Seul l'interne est hébergé (R-IT1) ; l'interne externé n'a pas de séjour d'hébergement.
    heberge BOOLEAN NOT NULL,
    ordre SMALLINT NOT NULL
);
INSERT INTO ent_regime_code (code, libelle, heberge, ordre) VALUES
    ('externe', 'Externe', false, 1),
    ('demi_pensionnaire', 'Demi-pensionnaire', false, 2),
    ('interne', 'Interne', true, 3),
    ('interne_externe', 'Interne externé', false, 4)
ON CONFLICT (code) DO NOTHING;

CREATE TABLE IF NOT EXISTS ent_regime_correspondance (
    vocabulaire TEXT NOT NULL CHECK (vocabulaire IN ('sigle', 'scolarite', 'historique')),
    valeur TEXT NOT NULL CHECK (valeur = lower(btrim(valeur)) AND length(valeur) BETWEEN 1 AND 40),
    code TEXT NOT NULL REFERENCES ent_regime_code (code),
    PRIMARY KEY (vocabulaire, valeur)
);
INSERT INTO ent_regime_correspondance (vocabulaire, valeur, code) VALUES
    ('sigle', 'ext', 'externe'),
    ('sigle', 'dp', 'demi_pensionnaire'),
    ('sigle', 'int', 'interne'),
    ('scolarite', 'externe', 'externe'),
    ('scolarite', 'demi_pension', 'demi_pensionnaire'),
    ('scolarite', 'interne', 'interne'),
    ('historique', 'e', 'externe'),
    ('historique', 'demi-pension', 'demi_pensionnaire'),
    ('historique', 'demi_pensionnaire', 'demi_pensionnaire'),
    ('historique', '1/2p', 'demi_pensionnaire'),
    ('historique', '1/2 p', 'demi_pensionnaire'),
    ('historique', 'i', 'interne'),
    ('historique', 'internat', 'interne'),
    ('historique', 'interne_externe', 'interne_externe'),
    ('historique', 'interne externé', 'interne_externe'),
    ('historique', 'ie', 'interne_externe')
ON CONFLICT (vocabulaire, valeur) DO NOTHING;

-- Fonction unique côté base (lecture de la table) ; NULL si la valeur n'est pas reconnue.
CREATE OR REPLACE FUNCTION ent_regime_vers_code(texte TEXT) RETURNS TEXT
LANGUAGE sql STABLE AS $$
    SELECT c.code FROM ent_regime_correspondance c
     WHERE c.valeur = lower(btrim(texte))
     ORDER BY CASE c.vocabulaire WHEN 'scolarite' THEN 0 WHEN 'sigle' THEN 1 ELSE 2 END
     LIMIT 1
$$;

-- ───────────────────────── 2. Suivis : borne de 300 caractères (RI-09) ─────────────────────────
-- Les suivis existants (jusqu'à 2000 caractères) sont conservés ; la borne vaut pour les nouveaux.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_suivi_texte_300_chk') THEN
        ALTER TABLE ent_internat_suivi
            ADD CONSTRAINT ent_internat_suivi_texte_300_chk CHECK (char_length(texte) <= 300) NOT VALID;
    END IF;
END
$$;

-- ───────────────────────── 3. Paramètres de l'internat (colonnes typées, pas de clé-valeur) ─────────────────────────
-- Une ligne par établissement ; les défauts sont ceux de §28 (S-4100 à S-4117).
CREATE TABLE IF NOT EXISTS ent_internat_parametres (
    etablissement_id UUID PRIMARY KEY
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    heure_limite_annonce TIME NOT NULL DEFAULT '16:00',
    heure_limite_appel TIME NOT NULL DEFAULT '21:30',
    seuil_retard_minutes SMALLINT NOT NULL DEFAULT 30 CHECK (seuil_retard_minutes BETWEEN 0 AND 720),
    heure_recuperation_vendredi TIME NOT NULL DEFAULT '18:00',
    -- 1 = lundi … 7 = dimanche (ISO).
    jour_bilan SMALLINT NOT NULL DEFAULT 4 CHECK (jour_bilan BETWEEN 1 AND 7),
    voisins_visibles_familles BOOLEAN NOT NULL DEFAULT false,
    depot_telephones BOOLEAN NOT NULL DEFAULT false,
    garde_honorabilite BOOLEAN NOT NULL DEFAULT false,
    impression_presents BOOLEAN NOT NULL DEFAULT true,
    majeur_info_parents TEXT NOT NULL DEFAULT 'si_a_charge'
        CHECK (majeur_info_parents IN ('toujours', 'si_a_charge', 'jamais')),
    -- Rythme par défaut (S-4100) : dimanche soir à jeudi soir.
    rythme_dimanche BOOLEAN NOT NULL DEFAULT true,
    rythme_lundi BOOLEAN NOT NULL DEFAULT true,
    rythme_mardi BOOLEAN NOT NULL DEFAULT true,
    rythme_mercredi BOOLEAN NOT NULL DEFAULT true,
    rythme_jeudi BOOLEAN NOT NULL DEFAULT true,
    rythme_vendredi BOOLEAN NOT NULL DEFAULT false,
    rythme_samedi BOOLEAN NOT NULL DEFAULT false,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_par UUID REFERENCES ent_user (id)
);

-- Types d'appel actifs et heures prévues (S-4101).
CREATE TABLE IF NOT EXISTS ent_internat_type_appel_actif (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    type TEXT NOT NULL CHECK (type IN (
        'arrivee', 'etude', 'soir', 'extinction', 'ronde', 'matin', 'rassemblement')),
    actif BOOLEAN NOT NULL DEFAULT true,
    heure_prevue TIME,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, type)
);

SELECT ent_rls_poser(ARRAY['ent_internat_parametres', 'ent_internat_type_appel_actif']);
