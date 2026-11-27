-- Socle 2 (l'élève), lot E5 : paramétrage des champs de l'élève par établissement.
--
-- Pour chaque champ du catalogue du logiciel (voir `domain-eleve::proposition`), l'établissement
-- règle : visible ou non, obligatoire à quels moments, saisissable par la famille, QUI FAIT FOI
-- (l'établissement, SIECLE, ONDE, une reprise, la famille) et si la modification locale est
-- libre, soumise à proposition ou interdite ; un champ peut en outre être VERROUILLÉ (gel
-- total : rien ne s'y applique tant qu'il l'est).
--
-- Sans ligne, un champ prend la valeur par défaut du catalogue (modification libre, la source
-- est l'établissement, la famille ne propose rien) : un établissement existant ne change pas
-- de comportement tant qu'il n'a pas appliqué un profil ou réglé un champ.
--
-- Le catalogue des champs, les formats et les bornes (ex. : l'INE n'est jamais proposable par
-- la famille) sont du LOGICIEL, vérifiés par le code ; la base ne contrôle que les valeurs
-- énumérées. Additif, idempotent, rejoué à chaque démarrage. Plage 287-289 : lot E5.

CREATE TABLE IF NOT EXISTS ent_param_eleve_profil (
    etablissement_id  UUID PRIMARY KEY,
    profil            TEXT NOT NULL,
    applique_le       TIMESTAMPTZ NOT NULL DEFAULT now(),
    applique_par      UUID,
    CONSTRAINT ent_param_eleve_profil_chk CHECK (profil IN (
        'public', 'prive_sous_contrat', 'prive_sous_contrat_logiciel_agree',
        'hors_contrat', 'premier_degre', 'personnalise'))
);

CREATE TABLE IF NOT EXISTS ent_param_eleve_champ (
    etablissement_id     UUID NOT NULL,
    champ                TEXT NOT NULL,
    visible              BOOLEAN NOT NULL DEFAULT true,
    obligatoire_a        TEXT[] NOT NULL DEFAULT '{}',
    saisie_famille       TEXT NOT NULL DEFAULT 'non',
    source_qui_fait_foi  TEXT NOT NULL DEFAULT 'scolia',
    modification_locale  TEXT NOT NULL DEFAULT 'libre',
    verrouille           BOOLEAN NOT NULL DEFAULT false,
    modifie_le           TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_par          UUID,
    PRIMARY KEY (etablissement_id, champ),
    CONSTRAINT ent_param_eleve_champ_nom_chk CHECK (champ ~ '^[a-z][a-z0-9_]{0,59}$'),
    CONSTRAINT ent_param_eleve_champ_famille_chk CHECK (saisie_famille IN ('non', 'proposer', 'modifier')),
    CONSTRAINT ent_param_eleve_champ_source_chk CHECK (
        source_qui_fait_foi IN ('scolia', 'siecle', 'onde', 'reprise', 'famille')),
    CONSTRAINT ent_param_eleve_champ_local_chk CHECK (
        modification_locale IN ('libre', 'par_proposition', 'interdite')),
    CONSTRAINT ent_param_eleve_champ_moments_chk CHECK (
        obligatoire_a <@ ARRAY['creation', 'inscription', 'export_bee', 'export_gar']::TEXT[])
);

-- Règles d'application automatique d'une proposition : toujours explicites, par champ et par
-- source, tracées sur la proposition appliquée.
CREATE TABLE IF NOT EXISTS ent_param_eleve_regle_auto (
    etablissement_id  UUID NOT NULL,
    champ             TEXT NOT NULL,
    source            TEXT NOT NULL,
    actif             BOOLEAN NOT NULL DEFAULT true,
    modifie_le        TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_par       UUID,
    PRIMARY KEY (etablissement_id, champ, source),
    CONSTRAINT ent_param_eleve_regle_auto_nom_chk CHECK (champ ~ '^[a-z][a-z0-9_]{0,59}$'),
    CONSTRAINT ent_param_eleve_regle_auto_source_chk CHECK (
        source IN ('import', 'educonnect', 'systeme', 'verification_campagne', 'famille'))
);

-- ───────────────────────── Isolation par établissement (RLS, mode permissif) ─────────────────────────
DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_param_eleve_profil', 'ent_param_eleve_champ', 'ent_param_eleve_regle_auto'] LOOP
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
