-- Socle 7 (« Documents, pièces et signatures »), lot D2 — migration 3700 : identité des documents, entités émettrices,
-- paramétrage des natures. Réf. : docs/socles/07_DOCUMENTS.md §8.2, §8.14 (R-D13), §23 (ligne D2).
--
-- Les natures sont des DÉCLARATIONS DE CODE (§5.3) : la base ne garde que le paramétrage de l'établissement.
-- Additive et rejouable (E8). Aucune donnée semi-structurée, aucun tableau, aucun contenu binaire (D-036, E5) :
-- les « textes encadrés » et les identifiants d'entité sont des tables de lignes typées.
-- Toutes les clés étrangères sont composites (etablissement_id, …) et en ON DELETE RESTRICT.

-- ───────────────────────── Entité émettrice (R-D13) ─────────────────────────
-- Un établissement émet au nom de lui-même, mais aussi, selon la famille de document, au nom de son organisme de
-- gestion (contrats, finances) ou d'un organisme de formation. Une série de numéros par entité (migration 3702).
CREATE TABLE IF NOT EXISTS ent_doc_entite (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    type TEXT NOT NULL CHECK (type IN ('etablissement', 'organisme_gestion', 'organisme_formation', 'ensemble')),
    raison_sociale TEXT NOT NULL CHECK (length(btrim(raison_sociale)) BETWEEN 1 AND 200),
    adresse TEXT CHECK (adresse IS NULL OR length(adresse) <= 500),
    mentions_pied TEXT CHECK (mentions_pied IS NULL OR length(mentions_pied) <= 500),
    actif BOOLEAN NOT NULL DEFAULT true,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_doc_entite_etab_id_uq UNIQUE (etablissement_id, id)
);
-- Une seule entité « établissement » par établissement : c'est l'entité par défaut de toutes les familles.
CREATE UNIQUE INDEX IF NOT EXISTS ent_doc_entite_etablissement_uq
    ON ent_doc_entite (etablissement_id) WHERE type = 'etablissement';

-- Identifiants officiels de l'entité (UAI, SIRET, numéro de déclaration d'activité) : une ligne par type.
CREATE TABLE IF NOT EXISTS ent_doc_entite_identifiant (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    entite_id UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('uai', 'siret', 'nda')),
    valeur TEXT NOT NULL CHECK (length(btrim(valeur)) BETWEEN 1 AND 40),
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, entite_id, type),
    CONSTRAINT ent_doc_entite_identifiant_entite_cfk FOREIGN KEY (etablissement_id, entite_id)
        REFERENCES ent_doc_entite (etablissement_id, id) ON DELETE RESTRICT
);

-- Famille de nature → entité émettrice (les familles sans ligne retombent sur l'entité « établissement »).
CREATE TABLE IF NOT EXISTS ent_doc_entite_famille (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    famille_nature TEXT NOT NULL CHECK (famille_nature IN (
        'scolarite', 'admission', 'vie_scolaire', 'discipline', 'pedagogie', 'stages', 'finances',
        'personnels', 'etablissement', 'courrier')),
    entite_id UUID NOT NULL,
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, famille_nature),
    CONSTRAINT ent_doc_entite_famille_entite_cfk FOREIGN KEY (etablissement_id, entite_id)
        REFERENCES ent_doc_entite (etablissement_id, id) ON DELETE RESTRICT
);

-- ───────────────────────── Identité des documents ─────────────────────────
-- Ce qui est propre aux documents ; nom officiel, adresse et coordonnées viennent du profil d'établissement (socle 5).
CREATE TABLE IF NOT EXISTS ent_doc_identite (
    etablissement_id UUID PRIMARY KEY
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    style_code TEXT NOT NULL DEFAULT 'classique'
        CHECK (style_code IN ('classique', 'contemporaine', 'epuree', 'institutionnelle')),
    -- Contraste AA sur blanc vérifié à l'enregistrement par le service (le modèle ne garde que la forme).
    couleur_principale TEXT CHECK (couleur_principale IS NULL OR couleur_principale ~ '^#[0-9A-Fa-f]{6}$'),
    mentions_pied TEXT CHECK (mentions_pied IS NULL OR length(mentions_pied) <= 500),
    fenetre_enveloppe TEXT NOT NULL DEFAULT 'gauche' CHECK (fenetre_enveloppe IN ('gauche', 'droite')),
    version INT NOT NULL DEFAULT 1 CHECK (version >= 1),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_par UUID
);

-- ───────────────────────── Paramétrage des natures ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_doc_nature_param (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nature_code TEXT NOT NULL
        CHECK (nature_code ~ '^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$' AND length(nature_code) <= 80),
    active BOOLEAN NOT NULL DEFAULT true,
    -- NULL = style de l'établissement ; sinon dérogation (réglage signalé S-D3).
    style_code TEXT CHECK (style_code IS NULL OR style_code IN ('classique', 'contemporaine', 'epuree', 'institutionnelle')),
    signataire_defaut_agent_id UUID,
    -- L'établissement peut monter le niveau de signature exigé, jamais le baisser (contrôle du service).
    niveau_signature_min TEXT CHECK (niveau_signature_min IS NULL OR niveau_signature_min IN ('S0', 'S1', 'S2', 'S3')),
    libre_service_actif BOOLEAN,
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    maj_par UUID,
    PRIMARY KEY (etablissement_id, nature_code),
    CONSTRAINT ent_doc_nature_param_agent_cfk FOREIGN KEY (etablissement_id, signataire_defaut_agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT
);

-- Textes encadrés (0 à 2 emplacements par nature, longueur bornée, sans mise en forme) : une ligne par emplacement.
CREATE TABLE IF NOT EXISTS ent_doc_nature_texte (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nature_code TEXT NOT NULL
        CHECK (nature_code ~ '^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$' AND length(nature_code) <= 80),
    emplacement TEXT NOT NULL CHECK (emplacement ~ '^[a-z][a-z0-9_]{0,39}$'),
    texte TEXT NOT NULL CHECK (length(btrim(texte)) BETWEEN 1 AND 400),
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    maj_par UUID,
    PRIMARY KEY (etablissement_id, nature_code, emplacement),
    CONSTRAINT ent_doc_nature_texte_param_cfk FOREIGN KEY (etablissement_id, nature_code)
        REFERENCES ent_doc_nature_param (etablissement_id, nature_code) ON DELETE RESTRICT
);

-- ───────────────────────── Cloisonnement par établissement ─────────────────────────
SELECT ent_rls_poser(ARRAY[
    'ent_doc_entite',
    'ent_doc_entite_identifiant',
    'ent_doc_entite_famille',
    'ent_doc_identite',
    'ent_doc_nature_param',
    'ent_doc_nature_texte'
]);
