-- Socle 20, lot C1 — migration 2405 : remises, dépôts immuables, signalements « je n'arrive pas à déposer », constats du
-- professeur, coche « fait » de l'élève. Réf. : 20_CAHIER_DE_TEXTES.md §24.1, §10, R-C9, R-C10, R-C12, R-C14.
-- `ent_cdt_vu` n'est PAS créée (R-C42, passe personas : le « vu » est supprimé) ; `ent_travail_eleve.vu_le` (150) reste
-- lu par les avis « modifié après lecture » jusqu'à la bascule (C3), puis est purgé.

CREATE TABLE IF NOT EXISTS ent_cdt_remise (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    travail_id UUID NOT NULL,
    eleve_id UUID,
    groupe_id UUID,
    etat TEXT NOT NULL DEFAULT 'brouillon' CHECK (etat IN ('brouillon', 'deposee', 'a_refaire')),
    dernier_depot_id UUID,
    en_retard BOOLEAN NOT NULL DEFAULT false,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    ancien_id UUID UNIQUE,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_remise_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_remise_cible_chk CHECK ((eleve_id IS NULL) <> (groupe_id IS NULL)),
    CONSTRAINT ent_cdt_remise_travail_fk FOREIGN KEY (etablissement_id, travail_id)
        REFERENCES ent_cdt_travail (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_remise_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_remise_groupe_fk FOREIGN KEY (etablissement_id, groupe_id)
        REFERENCES ent_cdt_groupe (etablissement_id, id) ON DELETE RESTRICT
);
-- Une remise par élève (ou par groupe) et par travail, même sous dépôts concurrents.
CREATE UNIQUE INDEX IF NOT EXISTS ent_cdt_remise_eleve_uq ON ent_cdt_remise (etablissement_id, travail_id, eleve_id) WHERE eleve_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_cdt_remise_groupe_uq ON ent_cdt_remise (etablissement_id, travail_id, groupe_id) WHERE groupe_id IS NOT NULL;
DROP TRIGGER IF EXISTS ent_cdt_remise_jamais_supprime ON ent_cdt_remise;
CREATE TRIGGER ent_cdt_remise_jamais_supprime BEFORE DELETE ON ent_cdt_remise
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_jamais_supprime();

-- Dépôt : IMMUABLE (R-C9). L'heure est celle du serveur au début de la réception ; l'empreinte est le SHA-256 du dépôt.
CREATE TABLE IF NOT EXISTS ent_cdt_depot (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    remise_id UUID NOT NULL,
    numero INTEGER NOT NULL CHECK (numero >= 1),
    recu_le TIMESTAMPTZ NOT NULL,
    par_user_id UUID,
    au_nom_de BOOLEAN NOT NULL DEFAULT false,
    texte TEXT CHECK (texte IS NULL OR length(texte) <= 20000),
    empreinte CHAR(64) NOT NULL CHECK (empreinte ~ '^[0-9a-f]{64}$'),
    nb_fichiers SMALLINT NOT NULL DEFAULT 0 CHECK (nb_fichiers >= 0),
    commentaire_eleve TEXT CHECK (commentaire_eleve IS NULL OR length(commentaire_eleve) <= 2000),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_depot_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_depot_numero_uq UNIQUE (remise_id, numero),
    CONSTRAINT ent_cdt_depot_remise_fk FOREIGN KEY (etablissement_id, remise_id)
        REFERENCES ent_cdt_remise (etablissement_id, id) ON DELETE RESTRICT
);
DROP TRIGGER IF EXISTS ent_cdt_depot_ajout_seul ON ent_cdt_depot;
CREATE TRIGGER ent_cdt_depot_ajout_seul BEFORE UPDATE OR DELETE ON ent_cdt_depot
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_ajout_seul();

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_cdt_remise_dernier_depot_fk') THEN
        ALTER TABLE ent_cdt_remise ADD CONSTRAINT ent_cdt_remise_dernier_depot_fk
            FOREIGN KEY (etablissement_id, dernier_depot_id) REFERENCES ent_cdt_depot (etablissement_id, id) ON DELETE RESTRICT;
    END IF;
END
$$;

CREATE TABLE IF NOT EXISTS ent_cdt_depot_signalement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    travail_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    message TEXT CHECK (message IS NULL OR length(message) <= 1000),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_depot_signalement_travail_fk FOREIGN KEY (etablissement_id, travail_id)
        REFERENCES ent_cdt_travail (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_depot_signalement_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id) ON DELETE RESTRICT
);

-- Constat du professeur : un FAIT (jamais un zéro, R-C12) ; ajout seul, la dernière ligne fait foi.
CREATE TABLE IF NOT EXISTS ent_cdt_statut (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    travail_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    statut TEXT NOT NULL CHECK (statut IN ('rendu_papier', 'non_rendu', 'dispense', 'excuse', 'attendu')),
    par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_statut_travail_fk FOREIGN KEY (etablissement_id, travail_id)
        REFERENCES ent_cdt_travail (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_statut_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_cdt_statut_idx ON ent_cdt_statut (etablissement_id, travail_id, eleve_id, cree_le DESC);
DROP TRIGGER IF EXISTS ent_cdt_statut_ajout_seul ON ent_cdt_statut;
CREATE TRIGGER ent_cdt_statut_ajout_seul BEFORE UPDATE OR DELETE ON ent_cdt_statut
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_ajout_seul();

-- Coche « fait » : outil de l'élève, lue par lui seul (R-C14), purgée à la clôture de l'année.
CREATE TABLE IF NOT EXISTS ent_cdt_coche (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    travail_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    fait_le TIMESTAMPTZ NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (travail_id, eleve_id),
    CONSTRAINT ent_cdt_coche_travail_fk FOREIGN KEY (etablissement_id, travail_id)
        REFERENCES ent_cdt_travail (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_coche_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id) ON DELETE RESTRICT
);

SELECT ent_rls_poser(ARRAY['ent_cdt_remise', 'ent_cdt_depot', 'ent_cdt_depot_signalement', 'ent_cdt_statut', 'ent_cdt_coche']);
