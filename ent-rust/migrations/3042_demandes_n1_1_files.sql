-- Socle N1 (« Demandes et flux de travail »), lot N1-1 — migration 3042 : files, membres calculés, paliers, réponses types.
-- Réf. : docs/socles/N1_DEMANDES.md §10 (files, routage), §13.3 (escalade), §12.1 (réponses types), §36 (R-DM47) et
-- §38 (D-061, Q-DM14) ; défauts NJ-85 (corbeille ouverte à tout le personnel) et NJ-94 (organisation par listes
-- d'adresses) du registre ALERTES_NEXTJS_PROD.md.
--
-- Contenu : ent_dem_file, ent_dem_file_membre_regle, ent_dem_file_palier, ent_dem_reponse_type.
-- Additive et rejouable (E8). Aucune donnée semi-structurée, aucun tableau, aucun contenu binaire (D-036).
-- Clés étrangères composites (etablissement_id, …) en ON DELETE RESTRICT : une file ne se supprime jamais, elle
-- s'archive (`actif = false`) ; les demandes qui l'ont reçue gardent leur référence.
--
-- MEMBRES CALCULÉS (NJ-94, R-DM8) : une file n'a **aucune liste d'adresses ni d'identifiants de comptes**. Elle a des
-- RÈGLES (mission du socle 6, fonction du socle 6, agent désigné et daté, personne désignée et datée pour le seul
-- repli) ; les membres se recalculent à chaque lecture, à la date du jour métier de l'établissement. Aucune règle
-- ne désigne « tout le personnel » (R-DM7, NJ-85) : la liste des sources est fermée par un CHECK.
--
-- `ent_dem_demande.file_id` (migration 3040) reste sans clé étrangère : les tests du cœur (N1-0) y posent des
-- identifiants de file fictifs, et une clé ajoutée ici les ferait échouer ; le service ne pose que des files qui
-- existent (`SourcesFiles`), et les files ne se suppriment jamais.

-- ───────────────────────── Files ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_dem_file (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    code TEXT NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{1,59}$'),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    -- Ce que le demandeur lit sur son accusé : « le secrétariat des élèves » (R-DM53). Sans valeur : le libellé.
    role_traitant TEXT CHECK (role_traitant IS NULL OR length(btrim(role_traitant)) BETWEEN 1 AND 120),
    -- `tri` : corbeille de qualification (membres désignés, jamais tout le personnel, NJ-85) ; `direction` : file
    -- de la direction ; `confidentielle` : natures C2+ ; `repli` : destinataires de dernier recours (RH, R-DM47).
    type TEXT NOT NULL CHECK (type IN ('service', 'tri', 'direction', 'confidentielle', 'repli')),
    site_id UUID,
    -- Calendrier d'ouverture qui compte les jours ouvrés de la file (socle 5, R-S2, R-DM22).
    service_calendrier TEXT NOT NULL DEFAULT 'secretariat'
        CHECK (service_calendrier IN ('secretariat', 'intendance', 'restauration', 'internat', 'accueil', 'cdi', 'vie_scolaire')),
    horaire_debut TIME,
    horaire_fin TIME,
    -- Borne un délai de nature : on ne rallonge jamais un délai déclaré (§10.2).
    delai_vise_jours SMALLINT CHECK (delai_vise_jours IS NULL OR delai_vise_jours BETWEEN 1 AND 90),
    message_accueil TEXT CHECK (message_accueil IS NULL OR length(message_accueil) <= 500),
    -- File de repli : obligatoire (R-DM6, A13 du socle 29) sauf pour le repli lui-même ; hors service pour les
    -- files confidentielles et la file direction (R-DM47, contrôlé par le service).
    file_repli_id UUID,
    -- SN1-02 (modifié, §37.1) : défaut « prénom + initiale » ; « service seul » possible par file (CDM-84).
    affichage_traitant TEXT NOT NULL DEFAULT 'prenom_initiale'
        CHECK (affichage_traitant IN ('prenom_initiale', 'service_seul')),
    -- Jumelage avec une boîte de service du socle 16 (R-DM58, DEM-396) : référence sans clé étrangère, la boîte
    -- relève d'un autre socle ; les membres sont alors définis une seule fois, côté boîte.
    boite_jumelee_id UUID,
    actif BOOLEAN NOT NULL DEFAULT true,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_file_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_file_code_uq UNIQUE (etablissement_id, code),
    CONSTRAINT ent_dem_file_site_cfk FOREIGN KEY (etablissement_id, site_id)
        REFERENCES ent_site (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_file_repli_cfk FOREIGN KEY (etablissement_id, file_repli_id)
        REFERENCES ent_dem_file (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_file_repli_autre_chk CHECK (file_repli_id IS NULL OR file_repli_id <> id),
    -- R-DM6 : toute file a un repli, sauf le repli de dernier recours.
    CONSTRAINT ent_dem_file_repli_obligatoire_chk CHECK (type = 'repli' OR file_repli_id IS NOT NULL),
    CONSTRAINT ent_dem_file_horaires_chk CHECK (
        (horaire_debut IS NULL AND horaire_fin IS NULL)
        OR (horaire_debut IS NOT NULL AND horaire_fin IS NOT NULL AND horaire_debut < horaire_fin))
);
CREATE INDEX IF NOT EXISTS ent_dem_file_type_idx ON ent_dem_file (etablissement_id, type) WHERE actif;

-- ───────────────────────── Règles de membres (calculées) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_dem_file_membre_regle (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    file_id UUID NOT NULL,
    -- Sources FERMÉES : aucune valeur « tous » (R-DM7, NJ-85) ; aucune adresse e-mail (R-DM36, NJ-94).
    --   mission  : agents ayant, à la date, une mission de ce type (socle 6), éventuellement sur un site ;
    --   fonction : agents ayant, à la date, cette fonction (socle 6) ;
    --   agent    : agent désigné, daté ;
    --   personne : personne désignée, datée — réservé au repli (président de l'OGEC, personne extérieure).
    source TEXT NOT NULL CHECK (source IN ('mission', 'fonction', 'agent', 'personne')),
    mission_type_code TEXT CHECK (mission_type_code IS NULL OR mission_type_code ~ '^[a-z][a-z0-9_]{1,59}$'),
    fonction_code TEXT CHECK (fonction_code IS NULL OR fonction_code ~ '^[a-z][a-z0-9_]{1,59}$'),
    agent_id UUID,
    personne_id UUID,
    -- Filtre de site des missions : mission d'établissement entier ou de ce site (socle 6, périmètre).
    site_id UUID,
    responsable BOOLEAN NOT NULL DEFAULT false,
    -- Ordre d'affichage et, pour le repli, ordre de passage au suivant (Q-DM14).
    rang SMALLINT NOT NULL DEFAULT 1 CHECK (rang BETWEEN 1 AND 99),
    du DATE NOT NULL,
    au DATE,
    -- Désigner une personne extérieure à la vie interne exige d'avoir lu l'avertissement : date de confirmation.
    exterieur_confirme_le TIMESTAMPTZ,
    annule_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_dem_file_membre_regle_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_file_membre_regle_file_cfk FOREIGN KEY (etablissement_id, file_id)
        REFERENCES ent_dem_file (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_file_membre_regle_agent_cfk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_file_membre_regle_personne_cfk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_file_membre_regle_site_cfk FOREIGN KEY (etablissement_id, site_id)
        REFERENCES ent_site (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_file_membre_regle_dates_chk CHECK (au IS NULL OR au >= du),
    -- Une colonne de source exactement, selon la source.
    CONSTRAINT ent_dem_file_membre_regle_forme_chk CHECK (
        (source = 'mission' AND mission_type_code IS NOT NULL AND fonction_code IS NULL
            AND agent_id IS NULL AND personne_id IS NULL)
        OR (source = 'fonction' AND fonction_code IS NOT NULL AND mission_type_code IS NULL
            AND agent_id IS NULL AND personne_id IS NULL AND site_id IS NULL)
        OR (source = 'agent' AND agent_id IS NOT NULL AND mission_type_code IS NULL
            AND fonction_code IS NULL AND personne_id IS NULL AND site_id IS NULL)
        OR (source = 'personne' AND personne_id IS NOT NULL AND mission_type_code IS NULL
            AND fonction_code IS NULL AND agent_id IS NULL AND site_id IS NULL))
);
CREATE INDEX IF NOT EXISTS ent_dem_file_membre_regle_file_idx
    ON ent_dem_file_membre_regle (etablissement_id, file_id) WHERE annule_le IS NULL;

-- ───────────────────────── Paliers d'escalade ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_dem_file_palier (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    file_id UUID NOT NULL,
    rang SMALLINT NOT NULL CHECK (rang BETWEEN 1 AND 5),
    -- Jours ouvrés après l'échéance de traitement (défaut : responsable à l'échéance, direction à +5, §13.3).
    apres_jours SMALLINT NOT NULL CHECK (apres_jours BETWEEN 0 AND 60),
    vers TEXT NOT NULL CHECK (vers IN ('responsable_file', 'direction', 'file')),
    vers_file_id UUID,
    CONSTRAINT ent_dem_file_palier_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_file_palier_rang_uq UNIQUE (etablissement_id, file_id, rang),
    CONSTRAINT ent_dem_file_palier_file_cfk FOREIGN KEY (etablissement_id, file_id)
        REFERENCES ent_dem_file (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_file_palier_vers_cfk FOREIGN KEY (etablissement_id, vers_file_id)
        REFERENCES ent_dem_file (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_file_palier_vers_chk CHECK ((vers = 'file') = (vers_file_id IS NOT NULL))
);

-- ───────────────────────── Réponses types ─────────────────────────
-- Texte avec variables TYPÉES entre doubles accolades (prenom, numero, date, service) : aucune variable libre qui
-- injecterait du balisage (NJ-95). Le rendu est ÉCHAPPÉ par le service ; le texte du modèle ne contient jamais de
-- balisage (contrôle ci-dessous et dans le service).
CREATE TABLE IF NOT EXISTS ent_dem_reponse_type (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    file_id UUID NOT NULL,
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    texte TEXT NOT NULL CHECK (length(btrim(texte)) BETWEEN 1 AND 4000 AND texte !~ '[<>]'),
    actif BOOLEAN NOT NULL DEFAULT true,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_reponse_type_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_reponse_type_file_cfk FOREIGN KEY (etablissement_id, file_id)
        REFERENCES ent_dem_file (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_dem_reponse_type_file_idx ON ent_dem_reponse_type (etablissement_id, file_id) WHERE actif;

-- ───────────────────────── Cloisonnement par établissement ─────────────────────────
SELECT ent_rls_poser(ARRAY[
    'ent_dem_file',
    'ent_dem_file_membre_regle',
    'ent_dem_file_palier',
    'ent_dem_reponse_type'
]);
