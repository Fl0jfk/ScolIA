-- 646 — Socle 42, lot V0d (2/3) : journal de voyage pour les familles (publications, photos marquées,
-- délégués, liens nominatifs révocables). Réf. : 42_SORTIES_VOYAGES.md §11.4 ; NX-33, NX-34, NX-80,
-- VOY-176 à VOY-179, VOY-254, VOY-265, R-SV40, R-SV42 ; corrige VD-10, VD-11, VD-12, VD-43.
--
-- Exigences E1 à E14 (PRA) : `etablissement_id` NOT NULL partout, clés étrangères COMPOSITES, UUID fournis par
-- l'application, aucun BYTEA (les photos vont au magasin d'objets), aucun JSON (D-036), horodatages, migration
-- rejouable (aucune modification des migrations 640 à 645), RLS posée en fin de fichier.
--
-- * Le droit à l'image n'a PAS de table ici : il est lu dans les consentements du socle 2 (formulaires,
--   `droit_image`). Une absence de décision n'est jamais un consentement pour une publication (R-SV40).
-- * Le jeton d'un lien nominatif n'est jamais conservé : seule son empreinte l'est ; il expire à la fin du
--   séjour + 15 jours et se révoque (corrige VD-10, VD-43).

CREATE TABLE IF NOT EXISTS ent_sortie_journal_voyage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    active_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    active_par_user_id UUID,
    ferme_le TIMESTAMPTZ,
    ferme_motif TEXT CHECK (ferme_motif IS NULL OR ferme_motif IN ('organisateur', 'direction', 'purge')),
    CONSTRAINT ent_sortie_journal_voyage_ferme_chk CHECK ((ferme_le IS NULL) = (ferme_motif IS NULL)),
    CONSTRAINT ent_sortie_journal_voyage_sortie_uq UNIQUE (sortie_id),
    CONSTRAINT ent_sortie_journal_voyage_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_journal_voyage_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id)
);

-- Délégués de publication : des encadrants DE LA SORTIE (NX-33), jamais un nom libre.
CREATE TABLE IF NOT EXISTS ent_sortie_journal_delegue (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    encadrant_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_sortie_journal_delegue_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_journal_delegue_uq UNIQUE (sortie_id, encadrant_id),
    CONSTRAINT ent_sortie_journal_delegue_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_journal_delegue_encadrant_fk FOREIGN KEY (etablissement_id, encadrant_id)
        REFERENCES ent_sortie_encadrant (etablissement_id, id)
);

CREATE TABLE IF NOT EXISTS ent_sortie_publication (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    auteur_user_id UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('message', 'point_de_situation')),
    -- Un point de situation est CODÉ : jamais d'incident médical ni de nom d'un autre élève (§11.4).
    nature_point TEXT CHECK (nature_point IS NULL OR nature_point IN ('arrivee', 'depart', 'changement_programme')),
    texte TEXT NOT NULL DEFAULT '' CHECK (length(texte) <= 4000),
    -- Déclaration explicite de l'auteur quand une photo ne montre aucun élève identifiable.
    aucun_eleve_identifiable BOOLEAN NOT NULL DEFAULT false,
    publiee_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    retiree_le TIMESTAMPTZ,
    retiree_par_user_id UUID,
    gardee_pour_bilan BOOLEAN NOT NULL DEFAULT false,
    CONSTRAINT ent_sortie_publication_point_chk CHECK ((type = 'point_de_situation') = (nature_point IS NOT NULL)),
    CONSTRAINT ent_sortie_publication_point_texte_chk CHECK (type <> 'point_de_situation' OR length(texte) <= 300),
    CONSTRAINT ent_sortie_publication_retrait_chk CHECK ((retiree_le IS NULL) = (retiree_par_user_id IS NULL)),
    CONSTRAINT ent_sortie_publication_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_publication_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_publication_sortie_idx
    ON ent_sortie_publication (etablissement_id, sortie_id, publiee_le);

CREATE TABLE IF NOT EXISTS ent_sortie_publication_photo (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    publication_id UUID NOT NULL,
    cle_objet TEXT NOT NULL,
    nom TEXT NOT NULL CHECK (length(nom) BETWEEN 1 AND 200),
    mime TEXT NOT NULL CHECK (mime IN ('image/png', 'image/jpeg', 'image/webp')),
    taille INTEGER NOT NULL CHECK (taille > 0),
    sha256 TEXT NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_publication_photo_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_publication_photo_pub_fk FOREIGN KEY (etablissement_id, publication_id)
        REFERENCES ent_sortie_publication (etablissement_id, id)
);

-- Marquage des élèves visibles sur une photo : INTERNE (jamais montré à une famille, R-SV42).
CREATE TABLE IF NOT EXISTS ent_sortie_publication_photo_eleve (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    photo_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_publication_photo_eleve_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_publication_photo_eleve_uq UNIQUE (photo_id, eleve_id),
    CONSTRAINT ent_sortie_publication_photo_eleve_photo_fk FOREIGN KEY (etablissement_id, photo_id)
        REFERENCES ent_sortie_publication_photo (etablissement_id, id),
    CONSTRAINT ent_sortie_publication_photo_eleve_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id)
);

-- Lien nominatif pour une famille SANS compte (NX-33, Q-V3) : un lien par (participant, titulaire), révocable,
-- à durée courte, avec limitation de débit. Seule l'empreinte du jeton est conservée.
CREATE TABLE IF NOT EXISTS ent_sortie_journal_lien (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    participant_id UUID NOT NULL,
    personne_id UUID NOT NULL,
    jeton_hash TEXT NOT NULL CHECK (jeton_hash ~ '^[0-9a-f]{64}$'),
    expire_le TIMESTAMPTZ NOT NULL,
    revoque_le TIMESTAMPTZ,
    dernier_acces_le TIMESTAMPTZ,
    fenetre_debut TIMESTAMPTZ,
    fenetre_nombre INTEGER NOT NULL DEFAULT 0 CHECK (fenetre_nombre >= 0),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_sortie_journal_lien_hash_uq UNIQUE (jeton_hash),
    CONSTRAINT ent_sortie_journal_lien_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_journal_lien_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_journal_lien_participant_fk FOREIGN KEY (etablissement_id, participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id),
    CONSTRAINT ent_sortie_journal_lien_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_journal_lien_sortie_idx
    ON ent_sortie_journal_lien (etablissement_id, sortie_id);

SELECT ent_rls_poser(ARRAY[
    'ent_sortie_journal_voyage',
    'ent_sortie_journal_delegue',
    'ent_sortie_publication',
    'ent_sortie_publication_photo',
    'ent_sortie_publication_photo_eleve',
    'ent_sortie_journal_lien'
]);
