-- Socle N1 (« Demandes et flux de travail »), lot N1-3 — migration 3047 : reprographie (nature `reprographie.tirage`).
-- Réf. : docs/socles/N1_DEMANDES.md §18, §21.6, §22.10, §36 (R-DM52), §38 ; conception du socle 44 §15 (NRM-01 à
-- NRM-10, DRM-01 à DRM-13) ; R-DM45 (le socle 44 reprendra ces tables telles quelles) ; NJ-77 (file de photocopies de
-- tous les sites), NJ-78 (PDF d'un autre rattachable), NJ-92 (« prête » par un lien public valable 30 jours).
--
-- Contenu : ent_dem_repro_site_reglage, ent_dem_repro_lieu_retrait, ent_dem_repro_receptionnaire, ent_dem_repro,
-- ent_dem_repro_document (RÉFÉRENCES au socle 7 : jamais de nom de fichier ni d'octets), ent_dem_repro_cible,
-- ent_dem_repro_oeuvre, ent_dem_repro_recurrence.
-- Aucune adresse électronique, aucun jeton de lien : « prête » est un geste authentifié (NJ-92). Aucune donnée
-- semi-structurée, aucun tableau, aucun contenu binaire (D-036). Additive et rejouable (E8). Clés étrangères composites
-- et ON DELETE RESTRICT (NJ-89).

-- ───────────────────────── Réglages d'un site (SN1-11, SN1-12) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_dem_repro_site_reglage (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    site_id UUID NOT NULL,
    -- Noir et blanc accepté d'emblée jusqu'à ce nombre de copies (pages × exemplaires) ; au-delà : validation.
    seuil_copies INT NOT NULL DEFAULT 500 CHECK (seuil_copies BETWEEN 1 AND 100000),
    -- Délai réaliste de la reprographie, en jours ouvrés : un « pour quand » plus court avertit, sans refuser.
    delai_jours SMALLINT NOT NULL DEFAULT 1 CHECK (delai_jours BETWEEN 0 AND 15),
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    maj_par_user_id UUID,
    PRIMARY KEY (etablissement_id, site_id),
    CONSTRAINT ent_dem_repro_site_reglage_site_cfk FOREIGN KEY (etablissement_id, site_id)
        REFERENCES ent_site (etablissement_id, id) ON DELETE RESTRICT
);

-- Lieux de retrait d'un site (liste fermée choisie par l'établissement).
CREATE TABLE IF NOT EXISTS ent_dem_repro_lieu_retrait (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    site_id UUID NOT NULL,
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 80),
    actif BOOLEAN NOT NULL DEFAULT true,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_repro_lieu_retrait_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_dem_repro_lieu_retrait_site_cfk FOREIGN KEY (etablissement_id, site_id)
        REFERENCES ent_site (etablissement_id, id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_dem_repro_lieu_retrait_uq
    ON ent_dem_repro_lieu_retrait (etablissement_id, site_id, lower(btrim(libelle)));

-- Qui tient la file d'impression de quel site (NJ-77, R-DM46, R-RM27 du 44) : une désignation datée par agent et par
-- site. Aucune désignation = personne ne voit la file ; jamais « tout un rôle » par défaut.
CREATE TABLE IF NOT EXISTS ent_dem_repro_receptionnaire (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    site_id UUID NOT NULL,
    agent_id UUID NOT NULL,
    du DATE NOT NULL,
    -- Fin EXCLUE : la désignation vaut jusqu'à la veille de `au`.
    au DATE,
    designe_par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_repro_receptionnaire_site_cfk FOREIGN KEY (etablissement_id, site_id)
        REFERENCES ent_site (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_repro_receptionnaire_agent_cfk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_repro_receptionnaire_periode_chk CHECK (au IS NULL OR au >= du)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_dem_repro_receptionnaire_actif_uq
    ON ent_dem_repro_receptionnaire (etablissement_id, site_id, agent_id) WHERE au IS NULL;
CREATE INDEX IF NOT EXISTS ent_dem_repro_receptionnaire_agent_idx
    ON ent_dem_repro_receptionnaire (etablissement_id, agent_id);

-- ───────────────────────── Le volet d'une demande `reprographie.tirage` ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_dem_repro (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    site_id UUID NOT NULL,
    couleur BOOLEAN NOT NULL DEFAULT false,
    recto_verso BOOLEAN NOT NULL DEFAULT false,
    agrafage BOOLEAN NOT NULL DEFAULT false,
    format TEXT NOT NULL DEFAULT 'a4' CHECK (format IN ('a4', 'a3')),
    exemplaires_total INT NOT NULL CHECK (exemplaires_total BETWEEN 1 AND 5000),
    pages_total INT NOT NULL CHECK (pages_total BETWEEN 1 AND 5000),
    -- Copies à imprimer = pages × exemplaires (recto-verso : mêmes pages, feuilles comptées par l'imprimante).
    copies_total INT NOT NULL CHECK (copies_total BETWEEN 1 AND 1000000),
    pour_le DATE NOT NULL,
    pour_heure TIME,
    retrait_lieu_id UUID,
    oeuvres_protegees BOOLEAN NOT NULL DEFAULT false,
    -- Avertissements calculés au dépôt, montrés à l'enseignant, jamais bloquants (DEM-293, DEM-294).
    avertissement_delai BOOLEAN NOT NULL DEFAULT false,
    avertissement_cfc BOOLEAN NOT NULL DEFAULT false,
    etat TEXT NOT NULL CHECK (etat IN (
        'a_valider', 'a_imprimer', 'en_impression', 'prete', 'retiree', 'refusee', 'annulee')),
    validation_requise BOOLEAN NOT NULL DEFAULT false,
    valide_par_personne_id UUID,
    valide_le TIMESTAMPTZ,
    validation_note TEXT CHECK (validation_note IS NULL OR length(validation_note) <= 300),
    en_impression_le TIMESTAMPTZ,
    prete_par_personne_id UUID,
    prete_le TIMESTAMPTZ,
    prete_retrait_lieu_id UUID,
    -- « Prête, pas encore vue » : posé quand le demandeur ouvre sa demande (§18.4).
    prete_vue_le TIMESTAMPTZ,
    retiree_le TIMESTAMPTZ,
    rappel_retrait_le TIMESTAMPTZ,
    probleme_signale_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, demande_id),
    CONSTRAINT ent_dem_repro_demande_cfk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_dem_demande (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_repro_site_cfk FOREIGN KEY (etablissement_id, site_id)
        REFERENCES ent_site (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_repro_retrait_cfk FOREIGN KEY (etablissement_id, retrait_lieu_id)
        REFERENCES ent_dem_repro_lieu_retrait (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_repro_prete_retrait_cfk FOREIGN KEY (etablissement_id, prete_retrait_lieu_id)
        REFERENCES ent_dem_repro_lieu_retrait (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_repro_valide_cfk FOREIGN KEY (etablissement_id, valide_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_repro_prete_par_cfk FOREIGN KEY (etablissement_id, prete_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    -- DRM-07 : aucune demande couleur ou au-delà du seuil n'entre dans la file d'impression sans une validation datée.
    CONSTRAINT ent_dem_repro_validation_chk CHECK (
        NOT validation_requise OR etat IN ('a_valider', 'refusee', 'annulee') OR valide_le IS NOT NULL),
    -- NJ-92 : « prête » porte toujours une PERSONNE authentifiée et sa date.
    CONSTRAINT ent_dem_repro_prete_chk CHECK (
        etat NOT IN ('prete', 'retiree') OR (prete_par_personne_id IS NOT NULL AND prete_le IS NOT NULL))
);
CREATE INDEX IF NOT EXISTS ent_dem_repro_file_idx ON ent_dem_repro (etablissement_id, site_id, etat, pour_le);

CREATE OR REPLACE FUNCTION ent_dem_repro_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_dem_repro : suppression interdite (NJ-89)' USING ERRCODE = '23001';
    END IF;
    IF NEW.etablissement_id <> OLD.etablissement_id OR NEW.demande_id <> OLD.demande_id
       OR NEW.couleur <> OLD.couleur OR NEW.copies_total <> OLD.copies_total OR NEW.cree_le <> OLD.cree_le
       OR NEW.validation_requise <> OLD.validation_requise THEN
        RAISE EXCEPTION 'ent_dem_repro : couleur, quantité et site de la demande figés après dépôt (DRM-07)'
            USING ERRCODE = '23001';
    END IF;
    -- Le site change seulement par un transfert (CDM-54), jamais par le demandeur après la validation.
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_dem_repro_garde_trg ON ent_dem_repro;
CREATE TRIGGER ent_dem_repro_garde_trg BEFORE UPDATE OR DELETE ON ent_dem_repro
    FOR EACH ROW EXECUTE FUNCTION ent_dem_repro_garde();

-- ───────────────────────── Fichiers à imprimer : références au socle 7 (NJ-78) ─────────────────────────
-- Le document est TOUJOURS déposé par la personne qui rattache (contrôle au dépôt, port du socle 7) ; ici n'est gardé
-- que l'identifiant, le nombre de pages lu du PDF par le socle 7, et la date d'effacement (§18.5).
CREATE TABLE IF NOT EXISTS ent_dem_repro_document (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    document_id UUID NOT NULL,
    pages INT NOT NULL CHECK (pages BETWEEN 1 AND 2000),
    depose_par_personne_id UUID NOT NULL,
    efface_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_repro_document_uq UNIQUE (etablissement_id, demande_id, document_id),
    CONSTRAINT ent_dem_repro_document_repro_cfk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_dem_repro (etablissement_id, demande_id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_repro_document_depose_cfk FOREIGN KEY (etablissement_id, depose_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_dem_repro_document_demande_idx ON ent_dem_repro_document (etablissement_id, demande_id);

-- ───────────────────────── Cibles : classes ou groupes, exemplaires et retrait par cible (DEM-368) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_dem_repro_cible (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    classe_id UUID,
    groupe_id UUID,
    exemplaires INT NOT NULL CHECK (exemplaires BETWEEN 1 AND 5000),
    retrait_lieu_id UUID,
    CONSTRAINT ent_dem_repro_cible_repro_cfk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_dem_repro (etablissement_id, demande_id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_repro_cible_classe_cfk FOREIGN KEY (etablissement_id, classe_id)
        REFERENCES ent_classe (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_repro_cible_groupe_cfk FOREIGN KEY (etablissement_id, groupe_id)
        REFERENCES ent_groupe (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_repro_cible_retrait_cfk FOREIGN KEY (etablissement_id, retrait_lieu_id)
        REFERENCES ent_dem_repro_lieu_retrait (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_repro_cible_au_plus_un_chk CHECK (num_nonnulls(classe_id, groupe_id) <= 1)
);
CREATE INDEX IF NOT EXISTS ent_dem_repro_cible_demande_idx ON ent_dem_repro_cible (etablissement_id, demande_id);

-- ───────────────────────── Œuvres protégées (plafonds CFC) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_dem_repro_oeuvre (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    genre TEXT NOT NULL CHECK (genre IN ('livre', 'revue')),
    titre TEXT NOT NULL CHECK (length(btrim(titre)) BETWEEN 1 AND 200),
    auteur TEXT CHECK (auteur IS NULL OR length(auteur) <= 120),
    editeur TEXT CHECK (editeur IS NULL OR length(editeur) <= 120),
    pages_copiees INT NOT NULL CHECK (pages_copiees BETWEEN 1 AND 5000),
    pages_oeuvre INT NOT NULL CHECK (pages_oeuvre BETWEEN 1 AND 20000),
    CONSTRAINT ent_dem_repro_oeuvre_repro_cfk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_dem_repro (etablissement_id, demande_id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_repro_oeuvre_pages_chk CHECK (pages_copiees <= pages_oeuvre)
);

-- ───────────────────────── Demande récurrente : un rappel, jamais un dépôt automatique (R-DM52, DEM-367) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_dem_repro_recurrence (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    personne_id UUID NOT NULL,
    modele_demande_id UUID NOT NULL,
    -- 1 = lundi … 7 = dimanche (ISO).
    jour_semaine SMALLINT NOT NULL CHECK (jour_semaine BETWEEN 1 AND 7),
    actif BOOLEAN NOT NULL DEFAULT true,
    dernier_rappel_le DATE,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_dem_repro_recurrence_personne_cfk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_dem_repro_recurrence_modele_cfk FOREIGN KEY (etablissement_id, modele_demande_id)
        REFERENCES ent_dem_repro (etablissement_id, demande_id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_dem_repro_recurrence_uq
    ON ent_dem_repro_recurrence (etablissement_id, personne_id, modele_demande_id) WHERE actif;

-- ───────────────────────── Cloisonnement par établissement ─────────────────────────
SELECT ent_rls_poser(ARRAY[
    'ent_dem_repro_site_reglage',
    'ent_dem_repro_lieu_retrait',
    'ent_dem_repro_receptionnaire',
    'ent_dem_repro',
    'ent_dem_repro_document',
    'ent_dem_repro_cible',
    'ent_dem_repro_oeuvre',
    'ent_dem_repro_recurrence'
]);
