-- Socle 4, lot A2 : vérification d'une coordonnée par code (ADM-05), préférences de contact et
-- langue (ADM-07), consentements administratifs (ADM-12). Migration additive et rejouable.
--
-- Vérification : un e-mail ou un mobile saisi par la famille est vérifié par code avant d'être
-- utilisé pour des envois sensibles. Seule l'empreinte HMAC du code est stockée (un code à six
-- chiffres n'a que 10^6 valeurs : sans secret serveur une base volée le retrouverait). Au plus un
-- code en attente par coordonnée. Le code n'est jamais écrit dans un journal ni un événement.
CREATE TABLE IF NOT EXISTS ent_coordonnee_verification (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    personne_id UUID NOT NULL,
    coordonnee_id UUID NOT NULL REFERENCES ent_personne_coordonnee (id) ON DELETE CASCADE,
    canal TEXT NOT NULL CHECK (canal IN ('email', 'sms')),
    code_hash TEXT NOT NULL,
    expire_le TIMESTAMPTZ NOT NULL,
    tentatives SMALLINT NOT NULL DEFAULT 0 CHECK (tentatives >= 0),
    statut TEXT NOT NULL DEFAULT 'envoye'
        CHECK (statut IN ('envoye', 'verifiee', 'expiree', 'annulee', 'epuisee')),
    demande_par UUID,
    envoye_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    verifiee_le TIMESTAMPTZ,
    CONSTRAINT ent_coordonnee_verification_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_coordonnee_verification_attente_uq
    ON ent_coordonnee_verification (coordonnee_id) WHERE statut = 'envoye';
CREATE INDEX IF NOT EXISTS ent_coordonnee_verification_personne_idx
    ON ent_coordonnee_verification (etablissement_id, personne_id, envoye_le);

-- Préférences (§8.3). `canal_email = false` / `canal_sms = false` valent « canal refusé »
-- (colonnes du lot A0, conservées). `joignable` : plages horaires pour l'appel, jamais pour
-- bloquer une urgence.
ALTER TABLE ent_personne_preference ADD COLUMN IF NOT EXISTS langue TEXT;
ALTER TABLE ent_personne_preference ADD COLUMN IF NOT EXISTS accessibilite TEXT[]
    NOT NULL DEFAULT '{}'::TEXT[];
ALTER TABLE ent_personne_preference ADD COLUMN IF NOT EXISTS joignable JSONB;
ALTER TABLE ent_personne_preference ADD COLUMN IF NOT EXISTS fuseau TEXT;
ALTER TABLE ent_personne_preference DROP CONSTRAINT IF EXISTS ent_personne_preference_langue_ck;
ALTER TABLE ent_personne_preference ADD CONSTRAINT ent_personne_preference_langue_ck
    CHECK (langue IS NULL OR langue ~ '^[a-z]{2,3}(-[A-Za-z0-9]{2,8})?$');
ALTER TABLE ent_personne_preference DROP CONSTRAINT IF EXISTS ent_personne_preference_acc_ck;
ALTER TABLE ent_personne_preference ADD CONSTRAINT ent_personne_preference_acc_ck
    CHECK (accessibilite <@ ARRAY['gros_caracteres', 'pas_d_appel_vocal', 'interprete']::TEXT[]);
ALTER TABLE ent_personne_preference DROP CONSTRAINT IF EXISTS ent_personne_preference_fuseau_ck;
ALTER TABLE ent_personne_preference ADD CONSTRAINT ent_personne_preference_fuseau_ck
    CHECK (fuseau IS NULL OR length(fuseau) <= 64);

-- Versions des préférences : une ligne par changement (instantané), en ajout seul.
CREATE TABLE IF NOT EXISTS ent_personne_preference_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    personne_id UUID NOT NULL,
    langue TEXT,
    canal_email BOOLEAN NOT NULL,
    canal_sms BOOLEAN NOT NULL,
    courrier_papier BOOLEAN NOT NULL,
    accessibilite TEXT[] NOT NULL DEFAULT '{}'::TEXT[],
    joignable JSONB,
    fuseau TEXT,
    via TEXT NOT NULL CHECK (via IN ('personnel', 'personne', 'systeme')),
    par UUID,
    quand TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_personne_preference_version_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ent_personne_preference_version_idx
    ON ent_personne_preference_version (etablissement_id, personne_id, quand);

-- Consentements propres au dossier administratif (§8.3). Une réponse = une ligne ; une nouvelle
-- réponse ferme la précédente (`retire_le`) : le texte exact présenté reste prouvable. Absence de
-- ligne ouverte = refus (la diffusion est refusée par défaut).
CREATE TABLE IF NOT EXISTS ent_personne_consentement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    personne_id UUID NOT NULL,
    objet TEXT NOT NULL CHECK (objet IN ('diffusion_associations_parents', 'diffusion_transport',
                                         'photo_remise', 'annuaire_parents_classe')),
    reponse TEXT NOT NULL CHECK (reponse IN ('accorde', 'refuse')),
    texte_version TEXT NOT NULL CHECK (length(btrim(texte_version)) BETWEEN 1 AND 80),
    canal TEXT NOT NULL CHECK (canal IN ('portail', 'papier', 'fiche_rentree', 'guichet')),
    piece_reference TEXT CHECK (piece_reference IS NULL OR length(btrim(piece_reference)) BETWEEN 1 AND 80),
    donne_par UUID,
    du TIMESTAMPTZ NOT NULL DEFAULT now(),
    retire_le TIMESTAMPTZ,
    retire_par UUID,
    CONSTRAINT ent_personne_consentement_papier_piece CHECK (canal <> 'papier' OR piece_reference IS NOT NULL),
    CONSTRAINT ent_personne_consentement_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_personne_consentement_ouvert_uq
    ON ent_personne_consentement (personne_id, objet) WHERE retire_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_personne_consentement_personne_idx
    ON ent_personne_consentement (etablissement_id, personne_id, du);

-- Reprise unique : la langue déclarée à la préinscription (A0 bis) devient la préférence.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM ent_coord_reprise_marque WHERE cle = '275_langue') THEN
        RETURN;
    END IF;
    INSERT INTO ent_personne_preference (personne_id, etablissement_id, langue)
    SELECT r.personne_id, r.etablissement_id, r.langue
      FROM ent_personne_reserve r
     WHERE r.langue IS NOT NULL
    ON CONFLICT (personne_id) DO UPDATE SET langue = COALESCE(ent_personne_preference.langue, EXCLUDED.langue);
    INSERT INTO ent_coord_reprise_marque (cle) VALUES ('275_langue') ON CONFLICT DO NOTHING;
END
$$;
