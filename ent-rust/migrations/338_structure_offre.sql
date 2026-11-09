-- Socle 5, lot S5 (`docs/socles/05_STRUCTURE.md` §9.5 et §9.10, STR-36 à STR-44) : matières
-- enrichies, offre d'enseignements de l'établissement PAR ANNÉE, règles de choix.
--
-- Principes tenus PAR LA BASE (pas seulement par le service) :
--   * l'offre appartient à une année et à un établissement (clés composites) ; ses cibles
--     (niveau, MEF), ses règles, incompatibilités, prérequis et groupes ne croisent jamais
--     deux établissements ni deux années ;
--   * une offre ne se supprime jamais : elle se ferme (`ferme_le`) ; une année clôturée gèle
--     son offre ;
--   * une règle de choix liste ses offres dans une table (pas de tableau d'UUID : clés
--     étrangères vérifiées) ;
--   * aucune règle nationale n'est codée : volumes, nombres et incompatibilités sont des données.
-- Additif et rejouable.

-- ───────────────────────── Cibles des clés composites ─────────────────────────
CREATE UNIQUE INDEX IF NOT EXISTS ent_annee_scolaire_id_etab_uq ON ent_annee_scolaire (id, etablissement_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_niveau_id_etab_uq ON ent_niveau (id, etablissement_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_mef_id_etab_uq ON ent_mef (id, etablissement_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_groupe_id_etab_uq ON ent_groupe (id, etablissement_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_matiere_id_etab_uq ON ent_matiere (id, etablissement_id);

-- ───────────────────────── Matières (STR-36, STR-39) ─────────────────────────
ALTER TABLE ent_matiere ADD COLUMN IF NOT EXISTS libelle_court TEXT;
ALTER TABLE ent_matiere ADD COLUMN IF NOT EXISTS couleur TEXT;
-- Regroupement du bulletin (« Langues », « Sciences »…), texte libre de l'établissement.
ALTER TABLE ent_matiere ADD COLUMN IF NOT EXISTS famille TEXT;
-- Sous-matière : « Français – oral » a pour parent « Français » (un seul niveau).
ALTER TABLE ent_matiere ADD COLUMN IF NOT EXISTS parent_id UUID;
-- Code de la nomenclature nationale (BCN) : texte, jamais une dépendance dure.
ALTER TABLE ent_matiere ADD COLUMN IF NOT EXISTS code_national TEXT;
ALTER TABLE ent_matiere ADD COLUMN IF NOT EXISTS archivee_le TIMESTAMPTZ;
ALTER TABLE ent_matiere ADD COLUMN IF NOT EXISTS modifie_le TIMESTAMPTZ NOT NULL DEFAULT now();

ALTER TABLE ent_matiere DROP CONSTRAINT IF EXISTS ent_matiere_s5_chk;
ALTER TABLE ent_matiere ADD CONSTRAINT ent_matiere_s5_chk CHECK (
    (libelle_court IS NULL OR char_length(btrim(libelle_court)) BETWEEN 1 AND 20)
    AND (couleur IS NULL OR couleur ~ '^#[0-9A-Fa-f]{6}$')
    AND (famille IS NULL OR char_length(btrim(famille)) BETWEEN 1 AND 60)
    AND (code_national IS NULL OR code_national ~ '^[0-9A-Za-z]{1,20}$')
    AND (parent_id IS NULL OR parent_id <> id)) NOT VALID;
ALTER TABLE ent_matiere DROP CONSTRAINT IF EXISTS ent_matiere_parent_fk;
ALTER TABLE ent_matiere ADD CONSTRAINT ent_matiere_parent_fk
    FOREIGN KEY (parent_id, etablissement_id) REFERENCES ent_matiere (id, etablissement_id);
CREATE INDEX IF NOT EXISTS ent_matiere_parent_idx ON ent_matiere (etablissement_id, parent_id) WHERE parent_id IS NOT NULL;

-- Un seul niveau de sous-matière ; un parent n'est pas lui-même une sous-matière ; une matière
-- qui a des sous-matières ne devient pas sous-matière.
CREATE OR REPLACE FUNCTION ent_trg_matiere_parent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.parent_id IS NOT NULL THEN
        IF EXISTS (SELECT 1 FROM ent_matiere p WHERE p.id = NEW.parent_id AND p.parent_id IS NOT NULL) THEN
            RAISE EXCEPTION 'matiere_parent_niveau: une sous-matière n''a pas de sous-matière' USING ERRCODE = '23514';
        END IF;
        IF EXISTS (SELECT 1 FROM ent_matiere c WHERE c.parent_id = NEW.id) THEN
            RAISE EXCEPTION 'matiere_parent_niveau: cette matière a déjà des sous-matières' USING ERRCODE = '23514';
        END IF;
    END IF;
    NEW.modifie_le := now();
    RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS ent_matiere_parent_trg ON ent_matiere;
CREATE TRIGGER ent_matiere_parent_trg BEFORE INSERT OR UPDATE OF parent_id ON ent_matiere
    FOR EACH ROW EXECUTE FUNCTION ent_trg_matiere_parent();

-- ───────────────────────── Offre d'enseignement (STR-41) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_offre_enseignement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    -- Code stable dans l'année (clé des imports et de la reprise des options historiques).
    code TEXT NOT NULL CHECK (char_length(btrim(code)) BETWEEN 1 AND 40),
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 160),
    -- NULL pour un dispositif sans matière (« Devoirs faits », « Vie de classe »).
    matiere_id UUID,
    modalite TEXT NOT NULL CHECK (modalite IN ('commun', 'langue_1', 'langue_2', 'langue_3',
        'option_obligatoire', 'option_facultative', 'specialite', 'dispositif', 'accompagnement', 'autre')),
    code_modalite_election TEXT CHECK (code_modalite_election IS NULL OR code_modalite_election ~ '^[0-9A-Za-z]{1,10}$'),
    -- Admission : libre, ou conditionnée à un avis, un test ou un dossier (section européenne, option sélective).
    admission TEXT NOT NULL DEFAULT 'libre' CHECK (admission IN ('libre', 'sur_avis', 'sur_test', 'sur_dossier')),
    -- Privé sous contrat : enseignement hors contrat, non exporté.
    hors_contrat BOOLEAN NOT NULL DEFAULT false,
    capacite INTEGER CHECK (capacite IS NULL OR capacite >= 0),
    -- Lu par le socle 3 : l'offre peut-elle être demandée à l'inscription ?
    ouvert_aux_voeux BOOLEAN NOT NULL DEFAULT false,
    -- Comment l'enseignement est organisé : par classe entière ou par groupe.
    public_type TEXT NOT NULL DEFAULT 'groupe' CHECK (public_type IN ('classe', 'groupe')),
    -- Après cette date, un changement de choix est un avertissement (STR-62), jamais un refus.
    date_limite_choix DATE,
    ordre SMALLINT NOT NULL DEFAULT 0,
    source TEXT NOT NULL DEFAULT 'saisie' CHECK (source IN ('saisie', 'reprise_option', 'import')),
    ferme_le TIMESTAMPTZ,
    ferme_motif TEXT CHECK (ferme_motif IS NULL OR char_length(ferme_motif) <= 300),
    created_by UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_offre_code_uq UNIQUE (etablissement_id, annee_id, code),
    CONSTRAINT ent_offre_id_etab_uq UNIQUE (id, etablissement_id),
    CONSTRAINT ent_offre_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id),
    CONSTRAINT ent_offre_matiere_fk FOREIGN KEY (matiere_id, etablissement_id)
        REFERENCES ent_matiere (id, etablissement_id)
);
CREATE INDEX IF NOT EXISTS ent_offre_annee_idx ON ent_offre_enseignement (etablissement_id, annee_id);
CREATE INDEX IF NOT EXISTS ent_offre_matiere_idx ON ent_offre_enseignement (etablissement_id, matiere_id) WHERE matiere_id IS NOT NULL;

-- Cible d'une offre : un niveau OU un MEF. Aucune cible = proposée à tous.
CREATE TABLE IF NOT EXISTS ent_offre_cible (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    offre_id UUID NOT NULL,
    niveau_id UUID,
    mef_id UUID,
    CONSTRAINT ent_offre_cible_un_seul_chk CHECK ((niveau_id IS NULL) <> (mef_id IS NULL)),
    CONSTRAINT ent_offre_cible_offre_fk FOREIGN KEY (offre_id, etablissement_id)
        REFERENCES ent_offre_enseignement (id, etablissement_id),
    CONSTRAINT ent_offre_cible_niveau_fk FOREIGN KEY (niveau_id, etablissement_id)
        REFERENCES ent_niveau (id, etablissement_id),
    CONSTRAINT ent_offre_cible_mef_fk FOREIGN KEY (mef_id, etablissement_id)
        REFERENCES ent_mef (id, etablissement_id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_offre_cible_niveau_uq ON ent_offre_cible (offre_id, niveau_id) WHERE niveau_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_offre_cible_mef_uq ON ent_offre_cible (offre_id, mef_id) WHERE mef_id IS NOT NULL;

-- Groupes qui portent une offre organisée par groupe (S6 y ajoutera la composition) : un groupe
-- porte au plus une offre, de la même année. Sert au contrôle « choix sans groupe ».
CREATE TABLE IF NOT EXISTS ent_offre_groupe (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    offre_id UUID NOT NULL,
    groupe_id UUID NOT NULL,
    created_by UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_offre_groupe_uq UNIQUE (offre_id, groupe_id),
    CONSTRAINT ent_offre_groupe_un_offre_uq UNIQUE (groupe_id),
    CONSTRAINT ent_offre_groupe_offre_fk FOREIGN KEY (offre_id, etablissement_id)
        REFERENCES ent_offre_enseignement (id, etablissement_id),
    CONSTRAINT ent_offre_groupe_groupe_fk FOREIGN KEY (groupe_id, etablissement_id)
        REFERENCES ent_groupe (id, etablissement_id)
);

-- ───────────────────────── Règles de choix et incompatibilités (STR-42) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_offre_regle_choix (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    -- Portée : un niveau, ou un MEF, ou (les deux NULL) tous les élèves de l'année.
    niveau_id UUID,
    mef_id UUID,
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 160),
    -- Nombre d'enseignements suivis parmi la liste : « LV2 : exactement 1 » = min 1, max 1.
    min SMALLINT NOT NULL DEFAULT 0 CHECK (min >= 0),
    max SMALLINT CHECK (max IS NULL OR max >= 0),
    -- Bloquante : l'écriture est refusée au-delà du maximum ; sinon avertissement.
    bloquant BOOLEAN NOT NULL DEFAULT true,
    archivee_le TIMESTAMPTZ,
    created_by UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_offre_regle_portee_chk CHECK (niveau_id IS NULL OR mef_id IS NULL),
    CONSTRAINT ent_offre_regle_bornes_chk CHECK (max IS NULL OR max >= min),
    CONSTRAINT ent_offre_regle_id_etab_uq UNIQUE (id, etablissement_id),
    CONSTRAINT ent_offre_regle_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id),
    CONSTRAINT ent_offre_regle_niveau_fk FOREIGN KEY (niveau_id, etablissement_id)
        REFERENCES ent_niveau (id, etablissement_id),
    CONSTRAINT ent_offre_regle_mef_fk FOREIGN KEY (mef_id, etablissement_id)
        REFERENCES ent_mef (id, etablissement_id)
);
CREATE INDEX IF NOT EXISTS ent_offre_regle_annee_idx ON ent_offre_regle_choix (etablissement_id, annee_id);

CREATE TABLE IF NOT EXISTS ent_offre_regle_choix_offre (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    regle_id UUID NOT NULL,
    offre_id UUID NOT NULL,
    PRIMARY KEY (regle_id, offre_id),
    CONSTRAINT ent_offre_regle_offre_regle_fk FOREIGN KEY (regle_id, etablissement_id)
        REFERENCES ent_offre_regle_choix (id, etablissement_id),
    CONSTRAINT ent_offre_regle_offre_offre_fk FOREIGN KEY (offre_id, etablissement_id)
        REFERENCES ent_offre_enseignement (id, etablissement_id)
);

-- Deux offres qui ne se suivent pas ensemble (« latin et grec sur la même barrette »).
-- Paire non ordonnée : stockée (offre_a < offre_b).
CREATE TABLE IF NOT EXISTS ent_offre_incompatibilite (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    offre_a UUID NOT NULL,
    offre_b UUID NOT NULL,
    motif TEXT CHECK (motif IS NULL OR char_length(motif) <= 300),
    bloquant BOOLEAN NOT NULL DEFAULT true,
    archivee_le TIMESTAMPTZ,
    created_by UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_offre_incompat_ordre_chk CHECK (offre_a < offre_b),
    CONSTRAINT ent_offre_incompat_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id),
    CONSTRAINT ent_offre_incompat_a_fk FOREIGN KEY (offre_a, etablissement_id)
        REFERENCES ent_offre_enseignement (id, etablissement_id),
    CONSTRAINT ent_offre_incompat_b_fk FOREIGN KEY (offre_b, etablissement_id)
        REFERENCES ent_offre_enseignement (id, etablissement_id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_offre_incompat_uq ON ent_offre_incompatibilite (offre_a, offre_b) WHERE archivee_le IS NULL;

-- Prérequis : pour suivre `offre_id`, l'élève doit suivre une autre offre de la MÊME année
-- (`offre_meme_annee`, ex. LV3 demande une LV2) ou avoir suivi une matière l'année précédente
-- (`matiere_annee_precedente`, ex. latin approfondi demande latin).
CREATE TABLE IF NOT EXISTS ent_offre_prerequis (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    offre_id UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('offre_meme_annee', 'matiere_annee_precedente')),
    prerequis_offre_id UUID,
    prerequis_matiere_id UUID,
    bloquant BOOLEAN NOT NULL DEFAULT true,
    created_by UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_offre_prerequis_forme_chk CHECK (
        (type = 'offre_meme_annee' AND prerequis_offre_id IS NOT NULL AND prerequis_matiere_id IS NULL
            AND prerequis_offre_id <> offre_id)
        OR (type = 'matiere_annee_precedente' AND prerequis_matiere_id IS NOT NULL AND prerequis_offre_id IS NULL)),
    CONSTRAINT ent_offre_prerequis_offre_fk FOREIGN KEY (offre_id, etablissement_id)
        REFERENCES ent_offre_enseignement (id, etablissement_id),
    CONSTRAINT ent_offre_prerequis_pre_offre_fk FOREIGN KEY (prerequis_offre_id, etablissement_id)
        REFERENCES ent_offre_enseignement (id, etablissement_id),
    CONSTRAINT ent_offre_prerequis_pre_matiere_fk FOREIGN KEY (prerequis_matiere_id, etablissement_id)
        REFERENCES ent_matiere (id, etablissement_id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_offre_prerequis_offre_uq ON ent_offre_prerequis (offre_id, prerequis_offre_id) WHERE prerequis_offre_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_offre_prerequis_matiere_uq ON ent_offre_prerequis (offre_id, prerequis_matiere_id) WHERE prerequis_matiere_id IS NOT NULL;

-- ───────────────────────── Cohérence d'année et gel ─────────────────────────
-- Toutes les pièces d'une offre (règles, incompatibilités, groupes, prérequis, cibles) portent
-- sur la même année que l'offre ; une année clôturée gèle son offre.
CREATE OR REPLACE FUNCTION ent_offre_annee_close(p_etab UUID, p_annee UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE AS $$
    SELECT coalesce((SELECT y.cloturee_le IS NOT NULL FROM ent_annee_scolaire y
                      WHERE y.id = p_annee AND y.etablissement_id = p_etab), false)
$$;

CREATE OR REPLACE FUNCTION ent_trg_offre_coherence() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v_annee UUID; v_annee2 UUID;
BEGIN
    IF TG_TABLE_NAME = 'ent_offre_enseignement' THEN
        IF TG_OP = 'UPDATE' AND (NEW.annee_id, NEW.etablissement_id, NEW.code) IS DISTINCT FROM (OLD.annee_id, OLD.etablissement_id, OLD.code) THEN
            RAISE EXCEPTION 'offre_immuable: année et code ne changent pas' USING ERRCODE = '23514';
        END IF;
        -- La reprise des options historiques peut créer l'offre d'une année déjà close (rien ne se perd).
        IF NEW.source <> 'reprise_option' AND ent_offre_annee_close(NEW.etablissement_id, NEW.annee_id) THEN
            RAISE EXCEPTION 'offre_annee_cloturee' USING ERRCODE = '23514';
        END IF;
        IF NEW.matiere_id IS NOT NULL AND EXISTS (
            SELECT 1 FROM ent_matiere m WHERE m.id = NEW.matiere_id AND m.archivee_le IS NOT NULL)
           AND (TG_OP = 'INSERT' OR NEW.matiere_id IS DISTINCT FROM OLD.matiere_id) THEN
            RAISE EXCEPTION 'offre_matiere_archivee' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    IF TG_TABLE_NAME = 'ent_offre_cible' OR TG_TABLE_NAME = 'ent_offre_prerequis' OR TG_TABLE_NAME = 'ent_offre_groupe' THEN
        SELECT annee_id INTO v_annee FROM ent_offre_enseignement WHERE id = NEW.offre_id AND etablissement_id = NEW.etablissement_id;
        IF ent_offre_annee_close(NEW.etablissement_id, v_annee) THEN
            RAISE EXCEPTION 'offre_annee_cloturee' USING ERRCODE = '23514';
        END IF;
        -- (imbriqués : PL/pgSQL n'évalue pas `AND` en court-circuit, et NEW n'a pas ce champ ailleurs)
        IF TG_TABLE_NAME = 'ent_offre_prerequis' THEN
            IF NEW.prerequis_offre_id IS NOT NULL THEN
                SELECT annee_id INTO v_annee2 FROM ent_offre_enseignement WHERE id = NEW.prerequis_offre_id AND etablissement_id = NEW.etablissement_id;
                IF v_annee2 IS DISTINCT FROM v_annee THEN
                    RAISE EXCEPTION 'offre_incoherente: prérequis d''une autre année' USING ERRCODE = '23514';
                END IF;
            END IF;
        END IF;
        IF TG_TABLE_NAME = 'ent_offre_groupe' THEN
            SELECT annee_id INTO v_annee2 FROM ent_groupe WHERE id = NEW.groupe_id AND etablissement_id = NEW.etablissement_id;
            IF v_annee2 IS DISTINCT FROM v_annee THEN
                RAISE EXCEPTION 'offre_incoherente: groupe d''une autre année' USING ERRCODE = '23514';
            END IF;
        END IF;
        RETURN NEW;
    END IF;
    IF TG_TABLE_NAME = 'ent_offre_regle_choix_offre' THEN
        SELECT annee_id INTO v_annee FROM ent_offre_regle_choix WHERE id = NEW.regle_id AND etablissement_id = NEW.etablissement_id;
        SELECT annee_id INTO v_annee2 FROM ent_offre_enseignement WHERE id = NEW.offre_id AND etablissement_id = NEW.etablissement_id;
        IF v_annee2 IS DISTINCT FROM v_annee THEN
            RAISE EXCEPTION 'offre_incoherente: règle et offre d''années différentes' USING ERRCODE = '23514';
        END IF;
        IF ent_offre_annee_close(NEW.etablissement_id, v_annee) THEN
            RAISE EXCEPTION 'offre_annee_cloturee' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    IF TG_TABLE_NAME = 'ent_offre_incompatibilite' THEN
        SELECT annee_id INTO v_annee FROM ent_offre_enseignement WHERE id = NEW.offre_a AND etablissement_id = NEW.etablissement_id;
        SELECT annee_id INTO v_annee2 FROM ent_offre_enseignement WHERE id = NEW.offre_b AND etablissement_id = NEW.etablissement_id;
        IF v_annee IS DISTINCT FROM NEW.annee_id OR v_annee2 IS DISTINCT FROM NEW.annee_id THEN
            RAISE EXCEPTION 'offre_incoherente: incompatibilité d''une autre année' USING ERRCODE = '23514';
        END IF;
        IF ent_offre_annee_close(NEW.etablissement_id, NEW.annee_id) THEN
            RAISE EXCEPTION 'offre_annee_cloturee' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    IF TG_TABLE_NAME = 'ent_offre_regle_choix' THEN
        IF ent_offre_annee_close(NEW.etablissement_id, NEW.annee_id) THEN
            RAISE EXCEPTION 'offre_annee_cloturee' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    RETURN NEW;
END $$;

DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_offre_enseignement', 'ent_offre_cible', 'ent_offre_groupe', 'ent_offre_prerequis',
                             'ent_offre_regle_choix', 'ent_offre_regle_choix_offre', 'ent_offre_incompatibilite']
    LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', t || '_coherence_trg', t);
        EXECUTE format('CREATE TRIGGER %I BEFORE INSERT OR UPDATE ON %I FOR EACH ROW EXECUTE FUNCTION ent_trg_offre_coherence()',
                       t || '_coherence_trg', t);
    END LOOP;
END $$;

-- Aucune suppression physique des offres (on les ferme) ni de leurs règles.
CREATE OR REPLACE FUNCTION ent_trg_offre_pas_de_suppression() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'offre : une offre ne se supprime pas (la fermer)' USING ERRCODE = '42501';
END $$;
DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_offre_enseignement', 'ent_offre_regle_choix', 'ent_offre_incompatibilite']
    LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', t || '_pas_suppr_trg', t);
        EXECUTE format('CREATE TRIGGER %I BEFORE DELETE ON %I FOR EACH ROW EXECUTE FUNCTION ent_trg_offre_pas_de_suppression()',
                       t || '_pas_suppr_trg', t);
    END LOOP;
END $$;

-- ───────────────────────── RLS ─────────────────────────
DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_offre_enseignement', 'ent_offre_cible', 'ent_offre_groupe', 'ent_offre_prerequis',
                             'ent_offre_regle_choix', 'ent_offre_regle_choix_offre', 'ent_offre_incompatibilite']
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
-- Le journal (332) est en liste fermée : on y ajoute les objets du lot S5.
ALTER TABLE ent_structure_journal DROP CONSTRAINT IF EXISTS ent_structure_journal_objet_check;
ALTER TABLE ent_structure_journal ADD CONSTRAINT ent_structure_journal_objet_check CHECK (objet IN
    ('profil', 'parametre', 'fuseau', 'site', 'batiment', 'salle', 'lieu', 'disponibilite',
     'indisponibilite', 'import_salles', 'prereglage',
     'matiere', 'offre', 'offre_groupe', 'regle_choix', 'incompatibilite', 'prerequis',
     'enseignements_suivis', 'enseignement_suivi'));
