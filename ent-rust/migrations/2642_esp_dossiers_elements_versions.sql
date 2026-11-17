-- Socle 32, lot ET1 — migration 2642 : dossiers, éléments, versions, figeages, dérivés, envois
-- (`32_BUREAUTIQUE.md` §10, §24, R-ET3, R-ET4, R-ET6, R-ET25, ND32-02, ND32-08, ND32-09, ND32-10).
--
-- * L'arborescence est **en base** : déplacer ou renommer ne touche jamais l'objet stocké (R-ET4, ND32-09). Le nom est
--   unique parmi les frères non supprimés (index unique partiel sur le nom normalisé : « refus si le nom est déjà pris »).
-- * Un élément est vivant, une **version est figée** (R-ET3, ND32-02) : chaque dépôt crée un `ent_objet` et une ligne de
--   version ; aucune ligne de version ne change de contenu. L'élément pointe sa version courante ; on revient en arrière
--   en **créant** une version. Une version peut être en cours d'analyse (`analyse`), disponible, ou en quarantaine ;
--   l'élément n'est visible des autres membres qu'après analyse (fermé par défaut, CET-14, CET-15).
-- * Les clés d'objet sont opaques (`{etablissement}/espaces/{uuid}`, E14) : le nom n'y figure jamais (ND32-10).
-- * `ent_espace_version_figee` : plusieurs figeages possibles par version (R-ET6) ; une version figée n'est jamais élaguée.
-- * `ent_espace_derive` : aperçus et textes extraits (reconstructibles, E12) ; aucune écriture avant le lot ET5a.
-- * `ent_espace_envoi` : l'intention d'un envoi direct (nom, dossier, élément visé) attachée à sa réservation de quota.
-- Sans JSON, sans tableau, sans BYTEA. Additif, rejouable.

CREATE TABLE IF NOT EXISTS ent_espace_dossier (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    espace_id UUID NOT NULL,
    parent_id UUID,
    nom TEXT NOT NULL CHECK (length(btrim(nom)) BETWEEN 1 AND 120),
    nom_normalise TEXT NOT NULL CHECK (length(nom_normalise) BETWEEN 1 AND 240),
    regle TEXT NOT NULL DEFAULT 'aucune' CHECK (regle IN
        ('aucune', 'lecture_seule_eleves', 'depot_sans_lecture', 'animateurs_seuls')),
    cree_par_personne_id UUID NOT NULL,
    supprime_le TIMESTAMPTZ,
    supprime_par_personne_id UUID,
    -- Une mise à la corbeille (dossier et contenu) forme un lot : on restaure le lot d'un coup.
    corbeille_lot_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_espace_dossier_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_espace_dossier_etab_espace_id_uq UNIQUE (etablissement_id, espace_id, id),
    CONSTRAINT ent_espace_dossier_supprime_chk CHECK ((supprime_le IS NULL) = (supprime_par_personne_id IS NULL)),
    CONSTRAINT ent_espace_dossier_espace_fk FOREIGN KEY (etablissement_id, espace_id)
        REFERENCES ent_espace (etablissement_id, id) ON DELETE RESTRICT,
    -- Le parent est dans le même espace (un dossier ne « passe » jamais dans un autre espace).
    CONSTRAINT ent_espace_dossier_parent_fk FOREIGN KEY (etablissement_id, espace_id, parent_id)
        REFERENCES ent_espace_dossier (etablissement_id, espace_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_dossier_auteur_fk FOREIGN KEY (etablissement_id, cree_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_dossier_pas_soi_meme_chk CHECK (parent_id IS NULL OR parent_id <> id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_espace_dossier_nom_uq
    ON ent_espace_dossier (etablissement_id, espace_id, COALESCE(parent_id, '00000000-0000-0000-0000-000000000000'::uuid), nom_normalise)
    WHERE supprime_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_espace_dossier_parent_idx ON ent_espace_dossier (etablissement_id, espace_id, parent_id);

CREATE TABLE IF NOT EXISTS ent_espace_element (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    espace_id UUID NOT NULL,
    dossier_id UUID,
    nature TEXT NOT NULL CHECK (nature IN ('fichier', 'document', 'lien', 'page', 'ressource_gar', 'notice_cdi')),
    nom TEXT NOT NULL CHECK (length(btrim(nom)) BETWEEN 1 AND 200),
    nom_normalise TEXT NOT NULL CHECK (length(nom_normalise) BETWEEN 1 AND 400),
    auteur_personne_id UUID NOT NULL,
    version_courante_id UUID,
    -- `analyse` : pas encore de version disponible (invisible des autres) ; `quarantaine` : analyse positive ;
    -- `corbeille` : 30 jours puis `efface` ; `masque` : modération (lot ET6).
    etat TEXT NOT NULL DEFAULT 'analyse' CHECK (etat IN ('analyse', 'disponible', 'quarantaine', 'masque', 'corbeille', 'efface')),
    etat_avant_corbeille TEXT CHECK (etat_avant_corbeille IS NULL OR etat_avant_corbeille IN ('analyse', 'disponible', 'quarantaine', 'masque')),
    corbeille_le TIMESTAMPTZ,
    corbeille_par_personne_id UUID,
    corbeille_lot_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_espace_element_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_espace_element_etab_espace_id_uq UNIQUE (etablissement_id, espace_id, id),
    CONSTRAINT ent_espace_element_corbeille_chk CHECK (
        (etat = 'corbeille') = (corbeille_le IS NOT NULL AND corbeille_par_personne_id IS NOT NULL AND etat_avant_corbeille IS NOT NULL)
        OR etat = 'efface'),
    CONSTRAINT ent_espace_element_espace_fk FOREIGN KEY (etablissement_id, espace_id)
        REFERENCES ent_espace (etablissement_id, id) ON DELETE RESTRICT,
    -- Le dossier est dans le même espace que l'élément.
    CONSTRAINT ent_espace_element_dossier_fk FOREIGN KEY (etablissement_id, espace_id, dossier_id)
        REFERENCES ent_espace_dossier (etablissement_id, espace_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_element_auteur_fk FOREIGN KEY (etablissement_id, auteur_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT
);
-- Deux éléments vivants de même nom dans le même dossier : impossible (jamais d'écrasement, ND32-02).
CREATE UNIQUE INDEX IF NOT EXISTS ent_espace_element_nom_uq
    ON ent_espace_element (etablissement_id, espace_id, COALESCE(dossier_id, '00000000-0000-0000-0000-000000000000'::uuid), nom_normalise)
    WHERE etat IN ('analyse', 'disponible', 'masque');
CREATE INDEX IF NOT EXISTS ent_espace_element_dossier_idx ON ent_espace_element (etablissement_id, espace_id, dossier_id);
CREATE INDEX IF NOT EXISTS ent_espace_element_auteur_idx ON ent_espace_element (etablissement_id, auteur_personne_id, modifie_le DESC);
CREATE INDEX IF NOT EXISTS ent_espace_element_corbeille_idx ON ent_espace_element (etablissement_id, corbeille_le) WHERE etat = 'corbeille';

CREATE TABLE IF NOT EXISTS ent_espace_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    element_id UUID NOT NULL,
    numero INTEGER NOT NULL CHECK (numero >= 1),
    objet_id UUID NOT NULL,
    auteur_personne_id UUID NOT NULL,
    commentaire TEXT CHECK (commentaire IS NULL OR length(commentaire) <= 300),
    nom_version TEXT CHECK (nom_version IS NULL OR length(btrim(nom_version)) BETWEEN 1 AND 120),
    type_reel TEXT NOT NULL CHECK (length(type_reel) BETWEEN 3 AND 127),
    taille BIGINT NOT NULL CHECK (taille > 0),
    origine TEXT NOT NULL CHECK (origine IN ('depot', 'editeur', 'conversion', 'restauration', 'brouillon_session')),
    -- Numéro de la version dont celle-ci est la restauration (retour arrière = nouvelle version, R-ET3).
    restaure_de_numero INTEGER CHECK (restaure_de_numero IS NULL OR restaure_de_numero >= 1),
    aide_ia BOOLEAN NOT NULL DEFAULT false,
    -- Analyse (socle 7) : fermé par défaut, la version n'est lisible des autres qu'une fois `disponible`.
    etat TEXT NOT NULL DEFAULT 'analyse' CHECK (etat IN ('analyse', 'disponible', 'quarantaine')),
    analyse_tentatives INTEGER NOT NULL DEFAULT 0 CHECK (analyse_tentatives >= 0),
    derniere_analyse_le TIMESTAMPTZ,
    analyse_motif TEXT CHECK (analyse_motif IS NULL OR length(analyse_motif) <= 120),
    elaguee_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_espace_version_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_espace_version_numero_uq UNIQUE (etablissement_id, element_id, numero),
    CONSTRAINT ent_espace_version_objet_uq UNIQUE (etablissement_id, objet_id),
    CONSTRAINT ent_espace_version_element_fk FOREIGN KEY (etablissement_id, element_id)
        REFERENCES ent_espace_element (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_version_objet_fk FOREIGN KEY (etablissement_id, objet_id)
        REFERENCES ent_objet (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_version_auteur_fk FOREIGN KEY (etablissement_id, auteur_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_espace_version_analyse_idx
    ON ent_espace_version (etablissement_id, cree_le) WHERE etat = 'analyse';
CREATE INDEX IF NOT EXISTS ent_espace_version_element_idx ON ent_espace_version (etablissement_id, element_id, numero DESC);

-- Immuabilité (R-ET3) : le contenu, l'auteur, la taille, le numéro d'une version ne changent jamais ; seuls l'état
-- d'analyse, l'élagage et le nom de version peuvent évoluer. Pas de DELETE.
CREATE OR REPLACE FUNCTION ent_espace_version_immuable() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF current_setting('ent.purge_etablissement', true) = 'on' THEN
            RETURN OLD;
        END IF;
        RAISE EXCEPTION 'ent_espace_version : suppression interdite (une version est figée)' USING ERRCODE = '42501';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.element_id IS DISTINCT FROM OLD.element_id
       OR NEW.numero IS DISTINCT FROM OLD.numero
       OR NEW.objet_id IS DISTINCT FROM OLD.objet_id
       OR NEW.auteur_personne_id IS DISTINCT FROM OLD.auteur_personne_id
       OR NEW.type_reel IS DISTINCT FROM OLD.type_reel
       OR NEW.taille IS DISTINCT FROM OLD.taille
       OR NEW.origine IS DISTINCT FROM OLD.origine
       OR NEW.commentaire IS DISTINCT FROM OLD.commentaire
       OR NEW.cree_le IS DISTINCT FROM OLD.cree_le
       OR (OLD.elaguee_le IS NOT NULL AND NEW.elaguee_le IS DISTINCT FROM OLD.elaguee_le)
       OR (OLD.etat <> 'analyse' AND NEW.etat IS DISTINCT FROM OLD.etat) THEN
        RAISE EXCEPTION 'ent_espace_version : version immuable' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS ent_espace_version_immuable_trg ON ent_espace_version;
CREATE TRIGGER ent_espace_version_immuable_trg BEFORE UPDATE OR DELETE ON ent_espace_version
    FOR EACH ROW EXECUTE FUNCTION ent_espace_version_immuable();

-- L'élément pointe sa version courante (clé composite : même établissement).
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_espace_element_version_courante_fk') THEN
        ALTER TABLE ent_espace_element ADD CONSTRAINT ent_espace_element_version_courante_fk
            FOREIGN KEY (etablissement_id, version_courante_id)
            REFERENCES ent_espace_version (etablissement_id, id) ON DELETE RESTRICT;
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS ent_espace_version_figee (
    version_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    socle_code TEXT NOT NULL CHECK (socle_code IN ('remise_20', 'emission_7', 'message_16', 'seance_33', 'portfolio')),
    reference_id UUID NOT NULL,
    jusqu_au DATE,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, version_id, socle_code, reference_id),
    CONSTRAINT ent_espace_version_figee_version_fk FOREIGN KEY (etablissement_id, version_id)
        REFERENCES ent_espace_version (etablissement_id, id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS ent_espace_derive (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    version_id UUID NOT NULL,
    sorte TEXT NOT NULL CHECK (sorte IN ('apercu_pdf', 'vignette', 'texte')),
    objet_id UUID NOT NULL,
    moteur_code TEXT NOT NULL CHECK (moteur_code ~ '^[a-z0-9_.-]{1,40}$'),
    moteur_version TEXT NOT NULL CHECK (moteur_version ~ '^[A-Za-z0-9_.+-]{1,40}$'),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_espace_derive_version_sorte_uq UNIQUE (etablissement_id, version_id, sorte),
    CONSTRAINT ent_espace_derive_version_fk FOREIGN KEY (etablissement_id, version_id)
        REFERENCES ent_espace_version (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_derive_objet_fk FOREIGN KEY (etablissement_id, objet_id)
        REFERENCES ent_objet (etablissement_id, id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS ent_espace_envoi (
    reservation_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    espace_id UUID NOT NULL,
    dossier_id UUID,
    -- Élément visé quand l'envoi est une nouvelle version (choix explicite de l'utilisateur) ; sinon nouvel élément.
    element_id UUID,
    nom TEXT NOT NULL CHECK (length(btrim(nom)) BETWEEN 1 AND 200),
    mime_declare TEXT CHECK (mime_declare IS NULL OR length(mime_declare) <= 127),
    commentaire TEXT CHECK (commentaire IS NULL OR length(commentaire) <= 300),
    demandeur_personne_id UUID NOT NULL,
    version_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, reservation_id),
    CONSTRAINT ent_espace_envoi_reservation_fk FOREIGN KEY (etablissement_id, reservation_id)
        REFERENCES ent_espace_reservation (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_envoi_dossier_fk FOREIGN KEY (etablissement_id, espace_id, dossier_id)
        REFERENCES ent_espace_dossier (etablissement_id, espace_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_envoi_element_fk FOREIGN KEY (etablissement_id, espace_id, element_id)
        REFERENCES ent_espace_element (etablissement_id, espace_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_envoi_demandeur_fk FOREIGN KEY (etablissement_id, demandeur_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_espace_envoi_version_fk FOREIGN KEY (etablissement_id, version_id)
        REFERENCES ent_espace_version (etablissement_id, id) ON DELETE RESTRICT
);

SELECT ent_rls_poser(ARRAY[
    'ent_espace_dossier', 'ent_espace_element', 'ent_espace_version', 'ent_espace_version_figee',
    'ent_espace_derive', 'ent_espace_envoi']);
