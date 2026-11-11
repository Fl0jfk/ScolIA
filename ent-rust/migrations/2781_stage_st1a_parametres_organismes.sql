-- 2781 — Socle 39 (stages), lot ST1 (a) : dispositifs, paramètres de l'établissement, organismes
-- étendus, sites et contacts. Réf. : docs/socles/39_STAGES.md §9, §10, §19.1, §19.2, §25.
-- Migration ADDITIVE et REJOUABLE (E8) ; aucune migration existante modifiée (PILOTAGE règle 15).
-- Aucune colonne JSON, aucun tableau, aucun BYTEA (D-036, E5, E10). Périmètre D-052 : stages de
-- collège et de lycée général et technologique seulement (ni PFMP ni apprentissage : les
-- dispositifs `pfmp` et `mobilite_internationale` ne sont pas fournis).
-- E1 : etablissement_id NOT NULL ; E2 : clés composées ; E6 : cree_le, modifie_le, version ;
-- E13 : RLS standard (ent_rls_poser).

-- ───────────── Référence globale (E12 : référentiel global, aucune donnée d'établissement) ─────────────
CREATE TABLE IF NOT EXISTS ref_stage_dispositif (
    code TEXT PRIMARY KEY CHECK (code ~ '^[a-z][a-z_]{1,39}$'),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    ordre SMALLINT NOT NULL DEFAULT 0,
    -- Observation seulement : aucun travail, même léger (C. éduc. D331-9).
    observation_seule BOOLEAN NOT NULL,
    -- Régime « accident du travail » (sinon accident scolaire).
    regime_at BOOLEAN NOT NULL,
    allocation BOOLEAN NOT NULL,
    texte_source TEXT NOT NULL CHECK (length(texte_source) BETWEEN 1 AND 300),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO ref_stage_dispositif (code, libelle, ordre, observation_seule, regime_at, allocation, texte_source) VALUES
    ('observation_college', 'Stage d''observation (collège)', 10, true, false, false,
     'C. éduc. D331-6 et D331-9 : 4e et 3e, sans condition d''âge, observation seulement'),
    ('seconde_gt', 'Stage de seconde générale et technologique', 20, false, false, false,
     'C. éduc. D333-3-1 : deux semaines consécutives en fin d''année'),
    ('immersion', 'Immersion (élève d''un autre établissement ou accueilli quelques jours)', 30, true, false, false,
     'Convention entre établissements, 1 à 5 jours'),
    ('stage_individuel', 'Stage individuel (relais, décrochage, prépa-métiers, allophone, aménagement)', 40, false, false, false,
     'Décision de l''équipe éducative, référent obligatoire'),
    ('autre_encadre', 'Autre stage encadré par un texte', 50, true, false, false,
     'Règles les plus strictes par défaut')
ON CONFLICT (code) DO NOTHING;

-- ───────────── Paramètres de l'établissement (colonnes typées, pas de clé-valeur) ─────────────
CREATE TABLE IF NOT EXISTS ent_stage_parametre (
    etablissement_id UUID PRIMARY KEY
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    -- STG-102 / DN-18 : assurance de l'établissement, jamais codée en dur ; obligatoire avant la
    -- première convention (contrôle au lot ST1 (b)).
    assureur_nom TEXT CHECK (assureur_nom IS NULL OR length(btrim(assureur_nom)) BETWEEN 1 AND 200),
    assurance_contrat TEXT CHECK (assurance_contrat IS NULL OR length(btrim(assurance_contrat)) BETWEEN 1 AND 100),
    -- STG-103 : délibération du CA approuvant la convention-type (ou décision équivalente).
    deliberation_ref TEXT CHECK (deliberation_ref IS NULL OR length(btrim(deliberation_ref)) BETWEEN 1 AND 200),
    deliberation_le DATE,
    -- STG-105 : quotas (S-3910, S-3911) ; NULL = non fixé.
    max_stagiaires_referent SMALLINT CHECK (max_stagiaires_referent IS NULL OR max_stagiaires_referent BETWEEN 1 AND 200),
    max_stagiaires_organisme SMALLINT CHECK (max_stagiaires_organisme IS NULL OR max_stagiaires_organisme BETWEEN 1 AND 200),
    -- STG-109 : délais de relance (S-3901 : J-90 à J-15 ; S-3902 ; S-3906).
    relance_ouverture_j SMALLINT NOT NULL DEFAULT 60 CHECK (relance_ouverture_j BETWEEN 15 AND 90),
    relance_sans_lieu_j SMALLINT NOT NULL DEFAULT 30 CHECK (relance_sans_lieu_j BETWEEN 7 AND 90),
    alerte_veille_heure TIME NOT NULL DEFAULT '16:00',
    -- S-3903 : conservation des contacts inactifs (années, bornes du DPO).
    conservation_contacts_annees SMALLINT NOT NULL DEFAULT 3 CHECK (conservation_contacts_annees BETWEEN 1 AND 5),
    -- S-3905 : l'établissement signe en dernier.
    etablissement_signe_en_dernier BOOLEAN NOT NULL DEFAULT true,
    -- S-3914 : règle d'accord des titulaires.
    accord_titulaires TEXT NOT NULL DEFAULT 'un' CHECK (accord_titulaires IN ('un', 'tous')),
    -- S-3913 : stage chez un titulaire ou dans l'entreprise familiale.
    stage_famille TEXT NOT NULL DEFAULT 'autorise_avec_alerte'
        CHECK (stage_famille IN ('autorise_avec_alerte', 'interdit')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_par_personne_id UUID,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    CONSTRAINT ent_stage_parametre_assurance_chk CHECK (
        (assureur_nom IS NULL) = (assurance_contrat IS NULL))
);

CREATE TABLE IF NOT EXISTS ent_stage_dispositif_etab (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    dispositif_code TEXT NOT NULL REFERENCES ref_stage_dispositif (code),
    actif BOOLEAN NOT NULL DEFAULT true,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_par_personne_id UUID,
    PRIMARY KEY (etablissement_id, dispositif_code)
);

-- ───────────── Organismes étendus (STG-110, STG-117) ─────────────
ALTER TABLE ent_entreprise
    ADD COLUMN IF NOT EXISTS type TEXT NOT NULL DEFAULT 'entreprise',
    ADD COLUMN IF NOT EXISTS enseigne TEXT,
    ADD COLUMN IF NOT EXISTS code_postal TEXT,
    ADD COLUMN IF NOT EXISTS commune TEXT,
    ADD COLUMN IF NOT EXISTS pays TEXT NOT NULL DEFAULT 'FR',
    ADD COLUMN IF NOT EXISTS etat TEXT NOT NULL DEFAULT 'actif',
    ADD COLUMN IF NOT EXISTS etat_motif_code TEXT,
    ADD COLUMN IF NOT EXISTS etat_motif_texte TEXT,
    ADD COLUMN IF NOT EXISTS etat_decide_par_personne_id UUID,
    ADD COLUMN IF NOT EXISTS etat_decide_le TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS origine TEXT NOT NULL DEFAULT 'saisie';

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_entreprise_type_chk') THEN
        ALTER TABLE ent_entreprise ADD CONSTRAINT ent_entreprise_type_chk CHECK (type IN (
            'entreprise', 'association', 'administration', 'collectivite',
            'etablissement_scolaire', 'sante', 'etranger', 'autre'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_entreprise_etat_chk') THEN
        ALTER TABLE ent_entreprise ADD CONSTRAINT ent_entreprise_etat_chk CHECK (etat IN (
            'actif', 'a_verifier', 'ferme', 'deconseille', 'interdit'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_entreprise_origine_chk') THEN
        ALTER TABLE ent_entreprise ADD CONSTRAINT ent_entreprise_origine_chk
            CHECK (origine IN ('saisie', 'proposition', 'reprise'));
    END IF;
    -- Motif codé (jamais une personne nommée) ; texte libre court réservé au DDFPT et à la direction.
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_entreprise_motif_chk') THEN
        ALTER TABLE ent_entreprise ADD CONSTRAINT ent_entreprise_motif_chk CHECK (
            (etat_motif_code IS NULL OR etat_motif_code IN (
                'securite', 'encadrement_insuffisant', 'refus_discriminatoire_signale',
                'non_respect_horaires', 'autre'))
            AND (etat_motif_texte IS NULL OR length(etat_motif_texte) <= 300)
            AND (etat <> 'deconseille' OR etat_motif_code IS NOT NULL));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_entreprise_adresse_chk') THEN
        ALTER TABLE ent_entreprise ADD CONSTRAINT ent_entreprise_adresse_chk CHECK (
            (enseigne IS NULL OR length(enseigne) <= 200)
            AND (code_postal IS NULL OR length(code_postal) <= 10)
            AND (commune IS NULL OR length(commune) <= 120)
            AND pays ~ '^[A-Z]{2}$');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_entreprise_etat_decide_fk') THEN
        ALTER TABLE ent_entreprise ADD CONSTRAINT ent_entreprise_etat_decide_fk
            FOREIGN KEY (etablissement_id, etat_decide_par_personne_id)
            REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT NOT VALID;
    END IF;
END
$$;

CREATE INDEX IF NOT EXISTS ent_entreprise_etat_idx ON ent_entreprise (etablissement_id, etat);

-- ───────────── Sites (lieux d'accueil) ─────────────
CREATE TABLE IF NOT EXISTS ent_entreprise_site (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    entreprise_id UUID NOT NULL,
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    adresse_ligne1 TEXT NOT NULL CHECK (length(btrim(adresse_ligne1)) BETWEEN 1 AND 200),
    adresse_ligne2 TEXT CHECK (adresse_ligne2 IS NULL OR length(adresse_ligne2) <= 200),
    code_postal TEXT NOT NULL CHECK (length(btrim(code_postal)) BETWEEN 1 AND 10),
    commune TEXT NOT NULL CHECK (length(btrim(commune)) BETWEEN 1 AND 120),
    pays TEXT NOT NULL DEFAULT 'FR' CHECK (pays ~ '^[A-Z]{2}$'),
    actif BOOLEAN NOT NULL DEFAULT true,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_personne_id UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    CONSTRAINT ent_entreprise_site_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_entreprise_site_entreprise_fk FOREIGN KEY (etablissement_id, entreprise_id)
        REFERENCES ent_entreprise (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_entreprise_site_cree_par_fk FOREIGN KEY (etablissement_id, cree_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_entreprise_site_entreprise_idx
    ON ent_entreprise_site (etablissement_id, entreprise_id);

-- ───────────── Contacts de l'organisme (tuteurs, signataires, RH) ─────────────
-- Un contact n'a jamais de compte (7 §10.5) : il agit par lien nominatif à portée limitée.
CREATE TABLE IF NOT EXISTS ent_entreprise_contact (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    entreprise_id UUID NOT NULL,
    nom TEXT NOT NULL CHECK (length(btrim(nom)) BETWEEN 1 AND 80),
    prenom TEXT NOT NULL CHECK (length(btrim(prenom)) BETWEEN 1 AND 80),
    fonction TEXT CHECK (fonction IS NULL OR length(fonction) <= 120),
    email TEXT CHECK (email IS NULL OR (length(email) BETWEEN 3 AND 254 AND email ~ '^[^@[:space:]]+@[^@[:space:]]+$')),
    telephone TEXT CHECK (telephone IS NULL OR length(telephone) BETWEEN 3 AND 30),
    est_tuteur BOOLEAN NOT NULL DEFAULT false,
    est_signataire BOOLEAN NOT NULL DEFAULT false,
    est_rh BOOLEAN NOT NULL DEFAULT false,
    -- Information du contact sur le traitement de ses données (à la première sollicitation).
    informe_le DATE,
    dernier_stage_le DATE,
    actif BOOLEAN NOT NULL DEFAULT true,
    desactive_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_personne_id UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    CONSTRAINT ent_entreprise_contact_etab_id_uq UNIQUE (etablissement_id, id),
    -- Canal requis pour signer (STG-115).
    CONSTRAINT ent_entreprise_contact_signataire_chk CHECK (NOT est_signataire OR email IS NOT NULL),
    CONSTRAINT ent_entreprise_contact_actif_chk CHECK (actif = (desactive_le IS NULL)),
    CONSTRAINT ent_entreprise_contact_entreprise_fk FOREIGN KEY (etablissement_id, entreprise_id)
        REFERENCES ent_entreprise (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_entreprise_contact_cree_par_fk FOREIGN KEY (etablissement_id, cree_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_entreprise_contact_entreprise_idx
    ON ent_entreprise_contact (etablissement_id, entreprise_id) WHERE actif;

SELECT ent_rls_poser(ARRAY[
    'ent_stage_parametre',
    'ent_stage_dispositif_etab',
    'ent_entreprise_site',
    'ent_entreprise_contact'
]);
