-- Socle 2, lot S2-3 : vérification de l'e-mail avant toute préinscription
-- (D-014, IDENTITE_ET_PROFILS_FAMILLES §3 et §6), profil personnel et journal
-- d'identité (niveau ENT, jamais lu par un établissement).
--
-- * `ent_preinscription_verification` : une demande par e-mail saisi, code à
--   6 chiffres (empreinte seulement), 5 essais, 15 minutes ; une fois vérifiée,
--   un jeton de préinscription (empreinte seulement, 2 h) ouvre l'espace famille.
--   Créée À L'IDENTIQUE que le compte existe ou non (anti-énumération).
-- * `ent_profil_personnel` : informations que la PERSONNE choisit de garder dans
--   son profil (« Enregistrer ces informations dans votre profil ? », non par défaut).
-- * `ent_identite_journal` : réutilisations d'informations, copies d'IBAN,
--   rattachements (qui, quand, vers quel établissement). L'établissement source
--   d'un IBAN n'est jamais écrit dans les tables de l'établissement cible.
-- Migration additive et rejouable.

CREATE TABLE IF NOT EXISTS ent_preinscription_verification (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    email TEXT NOT NULL CHECK (length(email) BETWEEN 3 AND 254),
    code_hash TEXT NOT NULL,
    tentatives SMALLINT NOT NULL DEFAULT 0,
    expire_le TIMESTAMPTZ NOT NULL,
    verifie_le TIMESTAMPTZ,
    -- Méthode de vérification : code reçu par e-mail ou session existante (passkey).
    methode TEXT NOT NULL DEFAULT 'code' CHECK (methode IN ('code', 'session')),
    jeton_hash TEXT,
    jeton_expire_le TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_preinscription_verification_jeton_uq
    ON ent_preinscription_verification (jeton_hash) WHERE jeton_hash IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_preinscription_verification_email_idx
    ON ent_preinscription_verification (etablissement_id, email, created_at DESC);

CREATE TABLE IF NOT EXISTS ent_profil_personnel (
    user_id UUID PRIMARY KEY REFERENCES ent_user (id) ON DELETE CASCADE,
    nom TEXT CHECK (nom IS NULL OR length(nom) <= 120),
    prenom TEXT CHECK (prenom IS NULL OR length(prenom) <= 120),
    civilite TEXT CHECK (civilite IS NULL OR civilite IN ('mme', 'm', 'autre')),
    telephone TEXT CHECK (telephone IS NULL OR length(telephone) <= 40),
    adresse TEXT CHECK (adresse IS NULL OR length(adresse) <= 500),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS ent_identite_journal (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES ent_user (id) ON DELETE CASCADE,
    action TEXT NOT NULL CHECK (action IN (
        'informations_reutilisees', 'profil_enregistre', 'iban_repris', 'compte_rattache', 'compte_cree')),
    etablissement_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS ent_identite_journal_user_idx
    ON ent_identite_journal (user_id, created_at DESC);
