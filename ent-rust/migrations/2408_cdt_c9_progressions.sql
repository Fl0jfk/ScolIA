-- Socle 20, lot C9 — migration 2408 : progressions, partage entre collègues, planification sur les séances, reconduction.
-- Réf. : 20_CAHIER_DE_TEXTES.md §16, §24.1 (ent_cdt_progression*). Numéro libre de la plage 2400–2419.
-- Additive et rejouable. Aucune colonne JSON, tableau ni BYTEA (D-036).
--
-- Modèle de menace :
--  * une progression est PRIVÉE à son auteur ; elle ne s'ouvre qu'à un cours (ses intervenants) ou à une discipline de
--    l'établissement par un partage explicite, jamais aux élèves ni aux familles ;
--  * un partage n'ouvre jamais le cahier d'un autre cours : la progression ne porte que des étapes (titres, notions, durées) ;
--  * la reconduction ne copie jamais de donnée d'élève : une progression n'en contient pas, et la copie d'entrées n'emporte ni
--    remise, ni coche, ni contribution nominative.

CREATE TABLE IF NOT EXISTS ent_cdt_progression (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    auteur_user_id UUID NOT NULL REFERENCES ent_user (id),
    auteur_agent_id UUID,
    annee_id UUID,
    titre TEXT NOT NULL CHECK (length(btrim(titre)) BETWEEN 1 AND 200),
    matiere_id UUID,
    niveau_id UUID,
    copie_de_id UUID,
    archivee_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_progression_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_progression_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_progression_copie_fk FOREIGN KEY (etablissement_id, copie_de_id)
        REFERENCES ent_cdt_progression (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_cdt_progression_auteur_idx ON ent_cdt_progression (etablissement_id, auteur_user_id) WHERE archivee_le IS NULL;

CREATE TABLE IF NOT EXISTS ent_cdt_progression_etape (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    progression_id UUID NOT NULL,
    ordre SMALLINT NOT NULL CHECK (ordre >= 1),
    titre TEXT NOT NULL CHECK (length(btrim(titre)) BETWEEN 1 AND 200),
    notions TEXT NOT NULL DEFAULT '' CHECK (length(notions) <= 2000),
    duree_seances SMALLINT NOT NULL DEFAULT 1 CHECK (duree_seances BETWEEN 1 AND 30),
    CONSTRAINT ent_cdt_progression_etape_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_progression_etape_ordre_uq UNIQUE (progression_id, ordre) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT ent_cdt_progression_etape_fk FOREIGN KEY (etablissement_id, progression_id)
        REFERENCES ent_cdt_progression (etablissement_id, id) ON DELETE RESTRICT
);

-- Partage : avec un cours (ses intervenants, dont un suppléant) OU avec une discipline ; jamais avec les élèves.
CREATE TABLE IF NOT EXISTS ent_cdt_progression_partage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    progression_id UUID NOT NULL,
    cours_id UUID,
    matiere_id UUID,
    depuis TIMESTAMPTZ NOT NULL DEFAULT now(),
    jusqu_au TIMESTAMPTZ,
    par_user_id UUID NOT NULL REFERENCES ent_user (id),
    CONSTRAINT ent_cdt_progression_partage_cible_chk CHECK ((cours_id IS NULL) <> (matiere_id IS NULL)),
    CONSTRAINT ent_cdt_progression_partage_fk FOREIGN KEY (etablissement_id, progression_id)
        REFERENCES ent_cdt_progression (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_progression_partage_cours_fk FOREIGN KEY (cours_id, etablissement_id)
        REFERENCES ent_cours (id, etablissement_id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_cdt_progression_partage_cours_uq
    ON ent_cdt_progression_partage (progression_id, cours_id) WHERE cours_id IS NOT NULL AND jusqu_au IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_cdt_progression_partage_matiere_uq
    ON ent_cdt_progression_partage (progression_id, matiere_id) WHERE matiere_id IS NOT NULL AND jusqu_au IS NULL;

-- Rattachement de l'étape à l'entrée planifiée (colonne posée par 2402 sans clé : elle reçoit la sienne ici).
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_cdt_entree_etape_fk') THEN
        ALTER TABLE ent_cdt_entree ADD CONSTRAINT ent_cdt_entree_etape_fk
            FOREIGN KEY (etablissement_id, progression_etape_id)
            REFERENCES ent_cdt_progression_etape (etablissement_id, id) ON DELETE RESTRICT NOT VALID;
    END IF;
END
$$;

SELECT ent_rls_poser(ARRAY['ent_cdt_progression', 'ent_cdt_progression_etape', 'ent_cdt_progression_partage']);
