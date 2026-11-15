-- Socle N1 (« Demandes et flux de travail »), lot N1-5 — migration 3048 : reprise des données du Next.js (R9).
-- Réf. : docs/socles/N1_DEMANDES.md §29, §31, §38 (D-061), CDM-50, R-DM49 ; NJ-84, NJ-88 (aucun rattachement par e-mail).
--
-- Contenu : ent_dem_reprise_lot (une passe de l'outil, avec l'empreinte de l'export lu), ent_dem_reprise_correspondance
-- (une ligne par objet source : rend la reprise REJOUABLE et le rapport vérifiable), ent_dem_reprise_demandeur (qui est le
-- demandeur d'une demande reprise : compte retrouvé, « à rapprocher » par le secrétariat, ou externe non vérifié ; adresse
-- toujours MASQUÉE), ent_dem_reprise_proposition (organisation du Next.js proposée, jamais appliquée seule).
-- Additive et rejouable (E8). Aucune colonne semi-structurée, aucun tableau, aucun contenu binaire (D-036, E5).
-- Données de reprise : classées « reprise », purgeables après la décision de Florian (registre E8, migration 3059).
-- Aucune adresse électronique complète, aucun téléphone, aucun nom n'est stocké ici : seulement un identifiant source
-- opaque et une adresse masquée.

CREATE TABLE IF NOT EXISTS ent_dem_reprise_lot (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    mode TEXT NOT NULL CHECK (mode IN ('repetition', 'bascule')),
    -- SHA-256 (hexadécimal) du texte de l'export lu : deux passes sur le même export se reconnaissent.
    empreinte_export TEXT NOT NULL CHECK (empreinte_export ~ '^[0-9a-f]{64}$'),
    -- Jour de l'export, en jour d'ÉTABLISSEMENT (jamais le jour UTC) : sert de « aujourd'hui » à la reprise.
    jour_export DATE NOT NULL,
    lance_par_user_id UUID NOT NULL,
    lance_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    termine_le TIMESTAMPTZ,
    -- Critère go/no-go (§29.2) : la reprise ne crée JAMAIS d'heure au socle 15 (CDM-50, double paiement sinon).
    -- Le compteur existe pour que la preuve soit lisible ; la contrainte l'interdit de bouger.
    heures_creees_au_15 INT NOT NULL DEFAULT 0 CHECK (heures_creees_au_15 = 0),
    CONSTRAINT ent_dem_reprise_lot_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_reprise_lot_fin_chk CHECK (termine_le IS NULL OR termine_le >= lance_le)
);
CREATE INDEX IF NOT EXISTS ent_dem_reprise_lot_etab_idx ON ent_dem_reprise_lot (etablissement_id, lance_le DESC);

CREATE TABLE IF NOT EXISTS ent_dem_reprise_correspondance (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    source TEXT NOT NULL CHECK (source IN (
        'request', 'request_piece', 'demandes_hse', 'hse_piece', 'photocopies', 'photocopies_piece', 'routing', 'org')),
    -- Identifiant de l'objet dans le Next.js (opaque, jamais une adresse ni un nom).
    source_id TEXT NOT NULL CHECK (length(source_id) BETWEEN 1 AND 200),
    cible_id UUID,
    decision TEXT NOT NULL CHECK (decision IN ('reprise', 'archive', 'ignoree', 'a_verifier')),
    raison_code TEXT CHECK (raison_code IS NULL OR raison_code ~ '^[a-z0-9_]{1,60}$'),
    -- Dernier lot qui a touché la ligne, et premier lot qui l'a créée ; nombre de passes.
    lot_reprise_id UUID NOT NULL,
    premier_lot_id UUID NOT NULL,
    passages INT NOT NULL DEFAULT 1 CHECK (passages >= 1),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_reprise_corr_uq UNIQUE (etablissement_id, source, source_id),
    CONSTRAINT ent_dem_reprise_corr_lot_cfk FOREIGN KEY (etablissement_id, lot_reprise_id)
        REFERENCES ent_dem_reprise_lot (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_reprise_corr_premier_cfk FOREIGN KEY (etablissement_id, premier_lot_id)
        REFERENCES ent_dem_reprise_lot (etablissement_id, id) ON DELETE RESTRICT,
    -- Une cible existe si et seulement si l'objet a été repris ou archivé.
    CONSTRAINT ent_dem_reprise_corr_cible_chk CHECK ((decision IN ('reprise', 'archive')) = (cible_id IS NOT NULL)),
    -- Ce qu'on n'a pas repris dit pourquoi.
    CONSTRAINT ent_dem_reprise_corr_raison_chk CHECK (decision IN ('reprise', 'archive') OR raison_code IS NOT NULL)
);
CREATE INDEX IF NOT EXISTS ent_dem_reprise_corr_lot_idx
    ON ent_dem_reprise_correspondance (etablissement_id, lot_reprise_id, source, decision);
CREATE INDEX IF NOT EXISTS ent_dem_reprise_corr_cible_idx
    ON ent_dem_reprise_correspondance (etablissement_id, cible_id) WHERE cible_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS ent_dem_reprise_demandeur (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    -- Identifiant du demandeur dans le Next.js, ou « ext: » + 16 premiers caractères d'une empreinte de son adresse
    -- (une même adresse externe donne une même ligne, sans que l'adresse soit conservée).
    source_ref TEXT NOT NULL CHECK (length(source_ref) BETWEEN 1 AND 120),
    personne_id UUID NOT NULL,
    etat TEXT NOT NULL CHECK (etat IN ('compte', 'a_rapprocher', 'externe_non_verifie', 'rapproche')),
    -- Adresse MASQUÉE (« j***@e***.fr ») : de quoi reconnaître, jamais de quoi écrire (NJ-84, NJ-88).
    repere_masque TEXT CHECK (repere_masque IS NULL OR length(repere_masque) <= 80),
    rapproche_par_user_id UUID,
    rapproche_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_reprise_demandeur_uq UNIQUE (etablissement_id, source_ref),
    CONSTRAINT ent_dem_reprise_demandeur_personne_cfk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_reprise_demandeur_rapproche_chk CHECK (
        (etat = 'rapproche') = (rapproche_le IS NOT NULL) AND (rapproche_le IS NULL) = (rapproche_par_user_id IS NULL))
);
CREATE INDEX IF NOT EXISTS ent_dem_reprise_demandeur_etat_idx
    ON ent_dem_reprise_demandeur (etablissement_id, etat);

CREATE TABLE IF NOT EXISTS ent_dem_reprise_proposition (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    lot_reprise_id UUID NOT NULL,
    genre TEXT NOT NULL CHECK (genre IN ('file', 'nature', 'designation', 'adresse_non_retrouvee')),
    -- Identifiant de l'unité ou de la tâche du Next.js ; pour une désignation : « unité|agent ».
    cle TEXT NOT NULL CHECK (length(cle) BETWEEN 1 AND 200),
    libelle TEXT CHECK (libelle IS NULL OR length(libelle) <= 200),
    nature_code TEXT CHECK (nature_code IS NULL OR nature_code ~ '^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$'),
    file_code TEXT CHECK (file_code IS NULL OR file_code ~ '^[a-z][a-z0-9_]{1,59}$'),
    agent_id UUID,
    responsable BOOLEAN NOT NULL DEFAULT false,
    -- Désignation DATÉE (source « agent ») : à convertir en mission avant la rentrée 2027 (MN1-090).
    du DATE,
    repere_masque TEXT CHECK (repere_masque IS NULL OR length(repere_masque) <= 80),
    statut TEXT NOT NULL DEFAULT 'proposee' CHECK (statut IN ('proposee', 'retenue', 'rejetee')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_reprise_proposition_uq UNIQUE (etablissement_id, genre, cle),
    CONSTRAINT ent_dem_reprise_proposition_lot_cfk FOREIGN KEY (etablissement_id, lot_reprise_id)
        REFERENCES ent_dem_reprise_lot (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_reprise_proposition_agent_cfk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_reprise_proposition_forme_chk CHECK (
        (genre <> 'designation' OR (agent_id IS NOT NULL AND du IS NOT NULL))
        AND (genre <> 'adresse_non_retrouvee' OR (agent_id IS NULL AND repere_masque IS NOT NULL)))
);

SELECT ent_rls_poser(ARRAY[
    'ent_dem_reprise_lot',
    'ent_dem_reprise_correspondance',
    'ent_dem_reprise_demandeur',
    'ent_dem_reprise_proposition'
]);
