-- 2543 — Socle 27, lot G2 : sous-traitants (liste de la plateforme, annonces, objections de l'établissement,
-- sous-traitants propres à l'établissement). Réf. : 27_RGPD_ARCHIVES.md §8.8, §17.1. Additive, rejouable.
--
-- * Tables de PLATEFORME (sans `etablissement_id`, sans RLS) : `rgp_soustraitant`, `rgp_soustraitant_donnee`,
--   `rgp_soustraitant_annonce`. Elles décrivent l'éditeur, jamais un établissement. Leur écriture n'est
--   exposée à aucun utilisateur d'établissement (le code la réserve à la console plateforme, à venir).
-- * Aucune ligne n'est semée ici : la liste des sous-traitants de ScolIA est un fait contractuel que
--   l'éditeur renseigne ; ce fichier ne présume d'aucun prestataire.

CREATE TABLE IF NOT EXISTS rgp_soustraitant (
    id UUID PRIMARY KEY,
    nom TEXT NOT NULL CHECK (length(btrim(nom)) BETWEEN 1 AND 200),
    service TEXT NOT NULL CHECK (length(btrim(service)) BETWEEN 1 AND 300),
    region TEXT CHECK (region IS NULL OR length(btrim(region)) BETWEEN 1 AND 100),
    pays TEXT NOT NULL CHECK (length(btrim(pays)) BETWEEN 2 AND 100),
    -- Garanties : texte libre court (accord de traitement, clauses, certification) ; jamais un secret.
    garanties TEXT CHECK (garanties IS NULL OR length(btrim(garanties)) BETWEEN 1 AND 1000),
    accord_version TEXT CHECK (accord_version IS NULL OR length(btrim(accord_version)) BETWEEN 1 AND 60),
    accord_le DATE,
    entree_le DATE NOT NULL,
    sortie_le DATE,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_soustraitant_dates CHECK (sortie_le IS NULL OR sortie_le >= entree_le)
);

CREATE TABLE IF NOT EXISTS rgp_soustraitant_donnee (
    soustraitant_id UUID NOT NULL REFERENCES rgp_soustraitant (id),
    categorie_code TEXT NOT NULL CHECK (categorie_code ~ '^[a-z]{2,8}\.[a-z_]{2,40}$'),
    PRIMARY KEY (soustraitant_id, categorie_code)
);

CREATE TABLE IF NOT EXISTS rgp_soustraitant_annonce (
    id UUID PRIMARY KEY,
    soustraitant_id UUID NOT NULL REFERENCES rgp_soustraitant (id),
    nature TEXT NOT NULL CHECK (nature IN ('entree', 'sortie', 'changement')),
    annonce_le DATE NOT NULL,
    effet_le DATE NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Délai d'objection : l'effet n'arrive pas avant l'annonce (S-547 : 30 jours par défaut, contrôlé par le code).
    CONSTRAINT rgp_soustraitant_annonce_dates CHECK (effet_le >= annonce_le)
);

CREATE TABLE IF NOT EXISTS rgp_soustraitant_objection (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annonce_id UUID NOT NULL REFERENCES rgp_soustraitant_annonce (id),
    motif TEXT NOT NULL CHECK (length(btrim(motif)) BETWEEN 3 AND 1000),
    par_compte_id UUID NOT NULL REFERENCES ent_user (id),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    reponse TEXT CHECK (reponse IS NULL OR length(btrim(reponse)) BETWEEN 1 AND 1000),
    repondu_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_soustraitant_objection_uq UNIQUE (etablissement_id, annonce_id),
    CONSTRAINT rgp_soustraitant_objection_reponse CHECK ((reponse IS NULL) = (repondu_le IS NULL))
);

CREATE TABLE IF NOT EXISTS rgp_soustraitant_local (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nom TEXT NOT NULL CHECK (length(btrim(nom)) BETWEEN 1 AND 200),
    service TEXT NOT NULL CHECK (length(btrim(service)) BETWEEN 1 AND 300),
    region TEXT CHECK (region IS NULL OR length(btrim(region)) BETWEEN 1 AND 100),
    accord_connu BOOLEAN NOT NULL DEFAULT false,
    cree_par_compte_id UUID REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_soustraitant_local_uq UNIQUE (etablissement_id, nom, service)
);

SELECT ent_rls_poser(ARRAY['rgp_soustraitant_objection', 'rgp_soustraitant_local']);
