-- Socle 2, lot S2-3 : invitations des responsables à la validation du dossier
-- (INS-13, D-014 second parent, DOS-09). Écrites par le consommateur
-- `d2.invitations` de l'événement `inscription.validee` (outbox S2-0).
--
-- Le jeton n'est stocké que sous forme d'empreinte ; le lien part par e-mail.
-- Accepter l'invitation confirme l'adresse et crée le compte, ou rattache une
-- identité existante (D-013) à la personne de l'établissement.
-- Idempotence : une invitation par (dossier, responsable).
-- Migration additive et rejouable.

CREATE TABLE IF NOT EXISTS ent_inscription_invitation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    dossier_id UUID NOT NULL REFERENCES ent_inscription_dossier (id) ON DELETE CASCADE,
    responsable_id UUID NOT NULL REFERENCES ent_inscription_responsable (id) ON DELETE CASCADE,
    personne_id UUID NOT NULL REFERENCES ent_personne (id) ON DELETE CASCADE,
    email TEXT NOT NULL CHECK (length(email) BETWEEN 3 AND 254),
    jeton_hash TEXT NOT NULL,
    statut TEXT NOT NULL DEFAULT 'envoyee'
        CHECK (statut IN ('envoyee', 'non_envoyee', 'acceptee', 'annulee')),
    expire_le TIMESTAMPTZ NOT NULL,
    accepte_le TIMESTAMPTZ,
    user_id UUID REFERENCES ent_user (id) ON DELETE SET NULL,
    source_event_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_inscription_invitation_uq UNIQUE (dossier_id, responsable_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_inscription_invitation_jeton_uq
    ON ent_inscription_invitation (jeton_hash);
