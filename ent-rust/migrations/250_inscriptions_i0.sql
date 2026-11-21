-- Socle 3, lot I0 (correctifs S2-3, `docs/socles/03_INSCRIPTIONS.md` §2.2 pts 1-4, 6, 20).
--
-- 1. Lien de responsabilité EN ATTENTE : un responsable rapproché d'une personne qui
--    a déjà un compte n'obtient aucun accès tant que cette personne n'a pas confirmé
--    (acceptation de l'invitation reçue à son adresse). Le lien réel (`ent_eleve_lien`)
--    n'est créé qu'à la confirmation : aucune lecture existante n'est affectée.
-- 2. Paramétrage des droits du lien, par établissement et par type de lien.
-- 3. File d'envoi : les messages sont écrits dans la transaction métier, expédiés après
--    sa validation, avec reprise et trace des échecs.
-- 4. Choix humains de la validation (rapprochements, foyer, élève existant) tracés.
-- Migration additive et rejouable.

CREATE TABLE IF NOT EXISTS ent_inscription_lien_attente (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    dossier_id UUID NOT NULL REFERENCES ent_inscription_dossier (id) ON DELETE CASCADE,
    responsable_id UUID NOT NULL REFERENCES ent_inscription_responsable (id) ON DELETE CASCADE,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id) ON DELETE CASCADE,
    personne_id UUID NOT NULL REFERENCES ent_personne (id) ON DELETE CASCADE,
    foyer_id UUID REFERENCES ent_foyer (id) ON DELETE SET NULL,
    lien TEXT NOT NULL,
    autorite_parentale BOOLEAN NOT NULL,
    peut_voir_notes BOOLEAN NOT NULL,
    peut_voir_vs BOOLEAN NOT NULL,
    peut_justifier_absence BOOLEAN NOT NULL,
    peut_autoriser_sorties BOOLEAN NOT NULL,
    est_payeur BOOLEAN NOT NULL,
    part_paiement SMALLINT,
    statut TEXT NOT NULL DEFAULT 'en_attente'
        CHECK (statut IN ('en_attente', 'confirme', 'refuse', 'annule')),
    decide_le TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_lien_attente_uq UNIQUE (dossier_id, responsable_id)
);
CREATE INDEX IF NOT EXISTS ent_inscription_lien_attente_personne_idx
    ON ent_inscription_lien_attente (etablissement_id, personne_id, statut);

CREATE TABLE IF NOT EXISTS ent_inscription_parametre_lien (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    lien TEXT NOT NULL CHECK (lien IN ('mere', 'pere', 'tuteur', 'autre')),
    peut_voir_notes BOOLEAN NOT NULL,
    peut_voir_vs BOOLEAN NOT NULL,
    peut_justifier_absence BOOLEAN NOT NULL,
    peut_autoriser_sorties BOOLEAN NOT NULL DEFAULT false,
    modifie_par UUID,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, lien)
);

-- Invitation : nouveaux états (envoi après validation de la transaction ; refus).
ALTER TABLE ent_inscription_invitation DROP CONSTRAINT IF EXISTS ent_inscription_invitation_statut_check;
ALTER TABLE ent_inscription_invitation
    ADD CONSTRAINT ent_inscription_invitation_statut_check
    CHECK (statut IN ('a_envoyer', 'envoyee', 'non_envoyee', 'acceptee', 'annulee', 'refusee'));

CREATE TABLE IF NOT EXISTS ent_inscription_envoi (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    dossier_id UUID REFERENCES ent_inscription_dossier (id) ON DELETE CASCADE,
    invitation_id UUID REFERENCES ent_inscription_invitation (id) ON DELETE CASCADE,
    cle TEXT NOT NULL CHECK (length(cle) BETWEEN 1 AND 200),
    kind TEXT NOT NULL CHECK (kind IN ('invitation', 'accueil')),
    destinataire TEXT NOT NULL CHECK (length(destinataire) BETWEEN 3 AND 254),
    sujet TEXT NOT NULL,
    -- `{LIEN}` : remplacé à l'envoi (le jeton se dérive d'un secret serveur, il n'est jamais stocké).
    corps TEXT NOT NULL,
    statut TEXT NOT NULL DEFAULT 'a_envoyer' CHECK (statut IN ('a_envoyer', 'envoye', 'echec')),
    tentatives INT NOT NULL DEFAULT 0,
    derniere_erreur TEXT,
    prochain_essai TIMESTAMPTZ NOT NULL DEFAULT now(),
    envoye_le TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_envoi_cle_uq UNIQUE (etablissement_id, cle)
);
CREATE INDEX IF NOT EXISTS ent_inscription_envoi_file_idx
    ON ent_inscription_envoi (prochain_essai) WHERE statut = 'a_envoyer';

-- Choix humains de la validation (rapprochement, foyer, élève existant) : trace d'audit.
ALTER TABLE ent_inscription_dossier ADD COLUMN IF NOT EXISTS resolutions JSONB;
