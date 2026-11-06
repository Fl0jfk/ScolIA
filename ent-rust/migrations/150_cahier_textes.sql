-- Socle 2, lot S2-6 : cahier de textes, travail à faire / à rendre, copies (CDT-01 → CDT-10).
-- Migration additive : aucune table existante n'est modifiée. `ent_cahier_texte` (migration 049)
-- reste lisible par ses routes historiques ; ses lignes sont reprises dans `ent_travail`
-- (reprise idempotente + déclencheur pour les nouvelles insertions).
--
-- Visibilité : un travail cible une classe OU un groupe (jamais les deux, jamais aucun).
-- Fichiers : stockés dans le magasin d'objets (clé `etablissement/cahier/…`), jamais en base ;
-- aucune clé n'est exposée au client, l'accès passe par des liens signés courts.

-- CDT-01 : contenu d'une séance datée (une ligne par séance, versionnée).
CREATE TABLE IF NOT EXISTS ent_seance_contenu (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    seance_id UUID NOT NULL REFERENCES ent_edt_seance (id) ON DELETE CASCADE,
    auteur_user_id UUID NOT NULL REFERENCES ent_user (id),
    contenu TEXT NOT NULL,
    ressources TEXT NOT NULL DEFAULT '',
    version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_seance_contenu_seance_uq UNIQUE (etablissement_id, seance_id),
    CONSTRAINT ent_seance_contenu_chk CHECK (length(btrim(contenu)) > 0)
);

-- CDT-02 / CDT-05 : travail à faire, à rendre en ligne, ou contrôle annoncé.
CREATE TABLE IF NOT EXISTS ent_travail (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    seance_id UUID REFERENCES ent_edt_seance (id) ON DELETE SET NULL,
    classe_id UUID REFERENCES ent_classe (id),
    groupe_id UUID REFERENCES ent_groupe (id),
    matiere_libelle TEXT NOT NULL,
    auteur_user_id UUID NOT NULL REFERENCES ent_user (id),
    type TEXT NOT NULL DEFAULT 'a_faire',
    titre TEXT NOT NULL,
    consigne TEXT NOT NULL,
    donne_le DATE NOT NULL,
    pour_le DATE NOT NULL,
    duree_estimee_min INTEGER,
    rendu_en_ligne BOOLEAN NOT NULL DEFAULT false,
    date_limite TIMESTAMPTZ,
    acceptation_retard BOOLEAN NOT NULL DEFAULT true,
    formats_acceptes TEXT[] NOT NULL DEFAULT '{}',
    version INTEGER NOT NULL DEFAULT 1,
    -- Reprise de l'ancien cahier (`ent_cahier_texte`), unique : reprise idempotente.
    ancien_id UUID UNIQUE,
    supprime_le TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_travail_cible_chk CHECK ((classe_id IS NULL) <> (groupe_id IS NULL)),
    CONSTRAINT ent_travail_type_chk CHECK (type IN ('a_faire', 'a_rendre', 'controle')),
    CONSTRAINT ent_travail_dates_chk CHECK (pour_le >= donne_le),
    CONSTRAINT ent_travail_duree_chk CHECK (duree_estimee_min IS NULL OR duree_estimee_min BETWEEN 1 AND 600),
    CONSTRAINT ent_travail_rendu_chk CHECK (
        (type = 'a_rendre') = rendu_en_ligne
        AND (rendu_en_ligne OR date_limite IS NULL)
    ),
    CONSTRAINT ent_travail_titre_chk CHECK (length(btrim(titre)) > 0)
);

CREATE INDEX IF NOT EXISTS ent_travail_classe_idx
    ON ent_travail (etablissement_id, classe_id, pour_le) WHERE classe_id IS NOT NULL AND supprime_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_travail_groupe_idx
    ON ent_travail (etablissement_id, groupe_id, pour_le) WHERE groupe_id IS NOT NULL AND supprime_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_travail_auteur_idx
    ON ent_travail (etablissement_id, auteur_user_id, pour_le);
CREATE INDEX IF NOT EXISTS ent_travail_seance_idx
    ON ent_travail (etablissement_id, seance_id) WHERE seance_id IS NOT NULL;

-- CDT-06 / CDT-07 : remise d'un élève (une par travail et par élève) et retour du professeur.
CREATE TABLE IF NOT EXISTS ent_travail_remise (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    travail_id UUID NOT NULL REFERENCES ent_travail (id) ON DELETE CASCADE,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    soumise_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    derniere_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    en_retard BOOLEAN NOT NULL DEFAULT false,
    commentaire_eleve TEXT NOT NULL DEFAULT '',
    statut TEXT NOT NULL DEFAULT 'rendu',
    retour_commentaire TEXT,
    retour_par_user_id UUID REFERENCES ent_user (id),
    retour_le TIMESTAMPTZ,
    note_id UUID REFERENCES ent_note (id) ON DELETE SET NULL,
    CONSTRAINT ent_travail_remise_uq UNIQUE (travail_id, eleve_id),
    CONSTRAINT ent_travail_remise_statut_chk CHECK (statut IN ('rendu', 'corrige'))
);

CREATE INDEX IF NOT EXISTS ent_travail_remise_eleve_idx
    ON ent_travail_remise (etablissement_id, eleve_id);

-- CDT-04 : pièces jointes. `genre` dit à qui la pièce appartient :
--   ressource (professeur, sur un travail), seance (professeur, sur le contenu d'une séance),
--   copie (élève, sur sa remise), annotation (professeur, sur la remise : copie annotée).
CREATE TABLE IF NOT EXISTS ent_cdt_piece (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    genre TEXT NOT NULL,
    travail_id UUID REFERENCES ent_travail (id) ON DELETE CASCADE,
    seance_contenu_id UUID REFERENCES ent_seance_contenu (id) ON DELETE CASCADE,
    remise_id UUID REFERENCES ent_travail_remise (id) ON DELETE CASCADE,
    nom TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    taille_octets BIGINT NOT NULL,
    storage_key TEXT NOT NULL,
    created_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_piece_genre_chk CHECK (genre IN ('ressource', 'seance', 'copie', 'annotation')),
    CONSTRAINT ent_cdt_piece_parent_chk CHECK (
        (genre = 'ressource' AND travail_id IS NOT NULL AND seance_contenu_id IS NULL AND remise_id IS NULL)
        OR (genre = 'seance' AND seance_contenu_id IS NOT NULL AND travail_id IS NULL AND remise_id IS NULL)
        OR (genre IN ('copie', 'annotation') AND remise_id IS NOT NULL AND travail_id IS NULL AND seance_contenu_id IS NULL)
    )
);

CREATE INDEX IF NOT EXISTS ent_cdt_piece_travail_idx ON ent_cdt_piece (travail_id) WHERE travail_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_cdt_piece_seance_idx ON ent_cdt_piece (seance_contenu_id) WHERE seance_contenu_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_cdt_piece_remise_idx ON ent_cdt_piece (remise_id) WHERE remise_id IS NOT NULL;

-- CDT-09 : indicateur personnel de l'élève (fait / pas fait) et trace du « vu ».
CREATE TABLE IF NOT EXISTS ent_travail_eleve (
    travail_id UUID NOT NULL REFERENCES ent_travail (id) ON DELETE CASCADE,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    etablissement_id UUID NOT NULL,
    fait BOOLEAN NOT NULL DEFAULT false,
    fait_le TIMESTAMPTZ,
    vu_le TIMESTAMPTZ,
    PRIMARY KEY (travail_id, eleve_id)
);

-- CDT-03 : historique des modifications (avant modification et suppression).
CREATE TABLE IF NOT EXISTS ent_cdt_historique (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    objet TEXT NOT NULL,
    objet_id UUID NOT NULL,
    version INTEGER NOT NULL,
    action TEXT NOT NULL,
    auteur_user_id UUID NOT NULL REFERENCES ent_user (id),
    instantane JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_historique_objet_chk CHECK (objet IN ('travail', 'seance_contenu'))
);

CREATE INDEX IF NOT EXISTS ent_cdt_historique_objet_idx
    ON ent_cdt_historique (etablissement_id, objet, objet_id, created_at);

-- CDT-10 : visa de la direction sur le cahier d'une classe (et, facultativement, d'un professeur).
CREATE TABLE IF NOT EXISTS ent_cdt_visa (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    classe_id UUID NOT NULL REFERENCES ent_classe (id),
    prof_user_id UUID REFERENCES ent_user (id),
    du DATE NOT NULL,
    au DATE NOT NULL,
    commentaire TEXT NOT NULL DEFAULT '',
    vise_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    vise_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_visa_dates_chk CHECK (au >= du)
);

CREATE INDEX IF NOT EXISTS ent_cdt_visa_classe_idx ON ent_cdt_visa (etablissement_id, classe_id, vise_le DESC);

-- Reprise de l'ancien cahier : une ligne par entrée dont la classe est rattachée au référentiel.
INSERT INTO ent_travail (id, etablissement_id, classe_id, matiere_libelle, auteur_user_id, type,
                         titre, consigne, donne_le, pour_le, ancien_id, created_at)
SELECT gen_random_uuid(), c.etablissement_id, c.classe_id, c.matiere_libelle, c.created_by_user_id,
       'a_faire', c.titre, c.consigne, LEAST(c.cree_le, c.pour_le), c.pour_le, c.id, c.created_at
  FROM ent_cahier_texte c
 WHERE c.classe_id IS NOT NULL
ON CONFLICT (ancien_id) DO NOTHING;

-- Les nouvelles entrées de l'ancienne route restent visibles dans le nouvel agenda.
CREATE OR REPLACE FUNCTION ent_cahier_texte_vers_travail() RETURNS trigger AS $$
BEGIN
    IF NEW.classe_id IS NOT NULL THEN
        INSERT INTO ent_travail (id, etablissement_id, classe_id, matiere_libelle, auteur_user_id, type,
                                 titre, consigne, donne_le, pour_le, ancien_id, created_at)
        VALUES (gen_random_uuid(), NEW.etablissement_id, NEW.classe_id, NEW.matiere_libelle,
                NEW.created_by_user_id, 'a_faire', NEW.titre, NEW.consigne,
                LEAST(NEW.cree_le, NEW.pour_le), NEW.pour_le, NEW.id, NEW.created_at)
        ON CONFLICT (ancien_id) DO NOTHING;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_cahier_texte_vers_travail_trg ON ent_cahier_texte;
CREATE TRIGGER ent_cahier_texte_vers_travail_trg
    AFTER INSERT ON ent_cahier_texte
    FOR EACH ROW EXECUTE FUNCTION ent_cahier_texte_vers_travail();
