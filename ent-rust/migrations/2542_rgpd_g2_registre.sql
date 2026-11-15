-- 2542 — Socle 27, lot G2 : registre des traitements généré, écarts, destinataires externes, mentions,
-- documents de conformité. Réf. : 27_RGPD_ARCHIVES.md §8.1, §17.1. Additive et rejouable, sans JSON ni tableau.
--
-- * Le registre est GÉNÉRÉ par le code (traitements de base, domaines du registre des tables, droits de
--   lecture réels) : la base ne porte que ce que l'établissement complète ou valide.
-- * `rgp_traitement_version` : une ligne par validation d'une fiche. Elle garde les empreintes SHA-256 des
--   facettes calculées (destinataires, données, sous-traitants, durées) : un écart se détecte en comparant
--   la fiche recalculée à la dernière version validée (R-RG10), sans copier de donnée.
-- * Écart à la conception : `rgp_traitement.etat` admet aussi `complete` (fiche confirmée par l'établissement,
--   pas encore validée) ; `rgp_document.document_id` (socle 7) est reporté : pas de pièce jointe en G2.

CREATE TABLE IF NOT EXISTS rgp_traitement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    code TEXT NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{1,59}$'),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 200),
    finalite TEXT NOT NULL CHECK (length(btrim(finalite)) BETWEEN 1 AND 1000),
    base_legale TEXT NOT NULL DEFAULT 'mission_interet_public' CHECK (base_legale IN (
        'mission_interet_public', 'obligation_legale', 'contrat', 'consentement', 'interet_legitime',
        'sauvegarde_interets_vitaux')),
    base_legale_motif TEXT CHECK (base_legale_motif IS NULL OR length(btrim(base_legale_motif)) BETWEEN 3 AND 500),
    responsable TEXT NOT NULL DEFAULT 'etablissement' CHECK (responsable IN ('etablissement', 'ogec', 'autre')),
    responsable_libelle TEXT CHECK (responsable_libelle IS NULL OR length(btrim(responsable_libelle)) BETWEEN 1 AND 200),
    hors_scolia BOOLEAN NOT NULL DEFAULT false,
    etat TEXT NOT NULL DEFAULT 'a_completer' CHECK (etat IN ('a_completer', 'complete', 'a_revoir', 'valide')),
    aipd_requise BOOLEAN NOT NULL DEFAULT false,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_traitement_code_uq UNIQUE (etablissement_id, code),
    CONSTRAINT rgp_traitement_id_etab_uq UNIQUE (etablissement_id, id),
    -- Un traitement hors ScolIA est déclaratif : jamais de domaine du registre des tables (R-RG10).
    CONSTRAINT rgp_traitement_local_code CHECK (NOT hors_scolia OR code LIKE 'local\_%')
);

CREATE TABLE IF NOT EXISTS rgp_traitement_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    traitement_id UUID NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    empreinte_destinataires TEXT NOT NULL CHECK (empreinte_destinataires ~ '^[0-9a-f]{64}$'),
    empreinte_donnees TEXT NOT NULL CHECK (empreinte_donnees ~ '^[0-9a-f]{64}$'),
    empreinte_soustraitants TEXT NOT NULL CHECK (empreinte_soustraitants ~ '^[0-9a-f]{64}$'),
    empreinte_durees TEXT NOT NULL CHECK (empreinte_durees ~ '^[0-9a-f]{64}$'),
    empreinte_activation TEXT NOT NULL CHECK (empreinte_activation ~ '^[0-9a-f]{64}$'),
    valide_par_compte_id UUID NOT NULL REFERENCES ent_user (id),
    valide_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_traitement_version_uq UNIQUE (traitement_id, version),
    CONSTRAINT rgp_traitement_version_fk FOREIGN KEY (etablissement_id, traitement_id)
        REFERENCES rgp_traitement (etablissement_id, id)
);

CREATE TABLE IF NOT EXISTS rgp_ecart (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    traitement_id UUID NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN (
        'destinataire_ajoute', 'donnees_changees', 'soustraitant_change', 'categorie_changee', 'module_active')),
    detecte_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    traite_le TIMESTAMPTZ,
    traite_par_compte_id UUID REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_ecart_fk FOREIGN KEY (etablissement_id, traitement_id)
        REFERENCES rgp_traitement (etablissement_id, id),
    CONSTRAINT rgp_ecart_traite_coherent CHECK ((traite_le IS NULL) = (traite_par_compte_id IS NULL))
);
-- Un écart de même nature ouvert par traitement : la détection est idempotente.
CREATE UNIQUE INDEX IF NOT EXISTS rgp_ecart_ouvert_uq
    ON rgp_ecart (etablissement_id, traitement_id, nature) WHERE traite_le IS NULL;

CREATE TABLE IF NOT EXISTS rgp_destinataire_externe (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    traitement_id UUID NOT NULL,
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 200),
    nature TEXT NOT NULL CHECK (nature IN ('administration', 'partenaire')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_destinataire_externe_fk FOREIGN KEY (etablissement_id, traitement_id)
        REFERENCES rgp_traitement (etablissement_id, id),
    CONSTRAINT rgp_destinataire_externe_uq UNIQUE (etablissement_id, traitement_id, libelle)
);

CREATE TABLE IF NOT EXISTS rgp_mention (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    public TEXT NOT NULL CHECK (public IN (
        'familles', 'eleves_college', 'eleves_lycee', 'personnels', 'visiteurs', 'candidats')),
    version INTEGER NOT NULL CHECK (version >= 1),
    empreinte TEXT NOT NULL CHECK (empreinte ~ '^[0-9a-f]{64}$'),
    publiee_par_compte_id UUID NOT NULL REFERENCES ent_user (id),
    publiee_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_mention_version_uq UNIQUE (etablissement_id, public, version)
);

CREATE TABLE IF NOT EXISTS rgp_document (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    code TEXT NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{1,59}$'),
    etat TEXT NOT NULL CHECK (etat IN ('manquant', 'brouillon', 'genere', 'importe', 'valide', 'sans_objet')),
    sans_objet_raison TEXT CHECK (sans_objet_raison IS NULL OR length(btrim(sans_objet_raison)) BETWEEN 3 AND 500),
    modifie_par_compte_id UUID REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_document_code_uq UNIQUE (etablissement_id, code),
    CONSTRAINT rgp_document_sans_objet CHECK ((etat = 'sans_objet') = (sans_objet_raison IS NOT NULL))
);

SELECT ent_rls_poser(ARRAY['rgp_traitement', 'rgp_traitement_version', 'rgp_ecart', 'rgp_destinataire_externe',
                           'rgp_mention', 'rgp_document']);
