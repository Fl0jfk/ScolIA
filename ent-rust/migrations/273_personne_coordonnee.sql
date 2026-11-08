-- Socle 4, lot A2 (§8.4, §7) : les coordonnées d'une personne sont portées par la FICHE de la
-- personne, pas par son compte. Une ligne par coordonnée (e-mail, téléphone, adresse postale de
-- contact), datée (`du`, `au`) : une correction ferme la ligne et en ouvre une autre, rien ne
-- s'écrase. L'historique de chaque changement (qui, quand, pourquoi) est dans
-- `ent_personne_coordonnee_histo` : il ne contient JAMAIS de valeur (les valeurs sont dans les
-- lignes datées, lisibles par qui a le droit), seulement la nature, la raison codée et l'auteur.
-- Les colonnes héritées `ent_personne.email / telephone / telephone_2 / adresse` deviennent une
-- PROJECTION maintenue par la base (migration 274) tant que des lecteurs existent (§16).
-- Migration additive et rejouable (rejouée à chaque démarrage).

-- Normalisation E.164 : renvoie NULL quand le numéro n'est pas sûr (il est alors gardé tel quel,
-- « à vérifier »). Source unique côté base ; l'équivalent Rust est testé contre cette fonction.
CREATE OR REPLACE FUNCTION ent_tel_e164(brut TEXT) RETURNS TEXT
LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE
    s TEXT;
BEGIN
    IF brut IS NULL THEN
        RETURN NULL;
    END IF;
    s := regexp_replace(btrim(brut), '[[:space:]  .\-()/]', '', 'g');
    IF s = '' THEN
        RETURN NULL;
    END IF;
    IF s LIKE '00%' THEN
        s := '+' || substr(s, 3);
    END IF;
    IF s ~ '^\+330[1-9][0-9]{8}$' THEN
        s := '+33' || substr(s, 5);
    END IF;
    IF s LIKE '+33%' THEN
        IF s ~ '^\+33[1-9][0-9]{8}$' THEN
            RETURN s;
        END IF;
        RETURN NULL;
    END IF;
    IF s ~ '^\+[1-9][0-9]{7,14}$' THEN
        RETURN s;
    END IF;
    IF s ~ '^0[1-9][0-9]{8}$' THEN
        RETURN '+33' || substr(s, 2);
    END IF;
    RETURN NULL;
END
$$;

-- Groupe d'une coordonnée : un « principal » par groupe et par personne.
CREATE OR REPLACE FUNCTION ent_coord_groupe(t TEXT) RETURNS TEXT
LANGUAGE sql IMMUTABLE AS $$
    SELECT CASE t WHEN 'email' THEN 'email' WHEN 'adresse' THEN 'adresse' ELSE 'telephone' END
$$;

CREATE TABLE IF NOT EXISTS ent_personne_coordonnee (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    personne_id UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('email', 'mobile', 'fixe', 'professionnel', 'adresse')),
    valeur TEXT NOT NULL CHECK (length(btrim(valeur)) BETWEEN 1 AND 500),
    libelle TEXT CHECK (libelle IS NULL OR length(libelle) <= 80),
    usages TEXT[] NOT NULL DEFAULT ARRAY['notifications']::TEXT[]
        CHECK (cardinality(usages) >= 1
               AND usages <@ ARRAY['notifications', 'urgence', 'facturation', 'connexion']::TEXT[]),
    principal BOOLEAN NOT NULL DEFAULT false
        CHECK (NOT principal OR 'notifications' = ANY (usages)),
    -- NULL = simplement déclarée (guichet, formulaire) ; renseignée après preuve de possession.
    verifiee_le TIMESTAMPTZ,
    verification_voie TEXT CHECK (verification_voie IS NULL
        OR verification_voie IN ('code_email', 'code_sms', 'compte', 'guichet')),
    -- Valeur non normalisable (numéro inconnu du format) : gardée telle quelle, à faire vérifier.
    a_verifier BOOLEAN NOT NULL DEFAULT false,
    delivrabilite TEXT NOT NULL DEFAULT 'inconnue'
        CHECK (delivrabilite IN ('inconnue', 'ok', 'rebond_temporaire', 'rebond_definitif',
                                 'non_attribue', 'desinscrit')),
    delivrabilite_le TIMESTAMPTZ,
    source TEXT NOT NULL DEFAULT 'saisie'
        CHECK (source IN ('saisie', 'famille', 'compte', 'reprise', 'ecriture_historique',
                          'declaration', 'import', 'fusion')),
    du TIMESTAMPTZ NOT NULL DEFAULT now(),
    au TIMESTAMPTZ,
    remplace_id UUID,
    cree_par UUID,
    CONSTRAINT ent_personne_coordonnee_periode CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_personne_coordonnee_valeur_email CHECK (
        type <> 'email' OR (length(valeur) <= 254 AND valeur = lower(valeur))),
    CONSTRAINT ent_personne_coordonnee_valeur_tel CHECK (
        type IN ('email', 'adresse') OR length(valeur) <= 40),
    CONSTRAINT ent_personne_coordonnee_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE CASCADE
);

-- Une même valeur n'est ouverte qu'une fois par personne et par type.
CREATE UNIQUE INDEX IF NOT EXISTS ent_personne_coordonnee_valeur_uq
    ON ent_personne_coordonnee (personne_id, type, valeur) WHERE au IS NULL;
-- Un seul principal ouvert par groupe (e-mail, téléphone, adresse) et par personne.
CREATE UNIQUE INDEX IF NOT EXISTS ent_personne_coordonnee_principal_uq
    ON ent_personne_coordonnee (personne_id, (ent_coord_groupe(type)))
    WHERE au IS NULL AND principal;
CREATE INDEX IF NOT EXISTS ent_personne_coordonnee_personne_idx
    ON ent_personne_coordonnee (etablissement_id, personne_id, du);
CREATE INDEX IF NOT EXISTS ent_personne_coordonnee_email_idx
    ON ent_personne_coordonnee (etablissement_id, valeur) WHERE au IS NULL AND type = 'email';

-- Historique de chaque changement : qui, quand, pourquoi. Aucune valeur de coordonnée ici.
CREATE TABLE IF NOT EXISTS ent_personne_coordonnee_histo (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    personne_id UUID NOT NULL,
    coordonnee_id UUID NOT NULL REFERENCES ent_personne_coordonnee (id) ON DELETE CASCADE,
    type TEXT NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN ('ajout', 'correction', 'remplacement', 'retrait',
        'principal', 'usages', 'verification', 'invalide', 'reprise', 'ecriture_historique',
        'divergence_resolue')),
    -- Pourquoi (code) : toujours présent ; le texte libre est facultatif.
    raison TEXT NOT NULL CHECK (raison IN ('erreur_de_saisie', 'changement_declare',
        'demande_de_la_personne', 'rebond', 'verification', 'compte_rattache', 'reprise',
        'ecriture_historique', 'divergence_compte', 'autre')),
    motif TEXT CHECK (motif IS NULL OR length(btrim(motif)) BETWEEN 3 AND 500),
    via TEXT NOT NULL CHECK (via IN ('personnel', 'personne', 'systeme')),
    par UUID,
    quand TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_personne_coordonnee_histo_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ent_personne_coordonnee_histo_personne_idx
    ON ent_personne_coordonnee_histo (etablissement_id, personne_id, quand);
CREATE INDEX IF NOT EXISTS ent_personne_coordonnee_histo_coord_idx
    ON ent_personne_coordonnee_histo (coordonnee_id);

-- Marqueur des reprises uniques (la liste des migrations est rejouée à chaque démarrage).
CREATE TABLE IF NOT EXISTS ent_coord_reprise_marque (
    cle TEXT PRIMARY KEY,
    fait_le TIMESTAMPTZ NOT NULL DEFAULT now()
);
