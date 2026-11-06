-- 255_email_confirmation.sql — Revue de sécurité socle 1, RF-01 : changement d'e-mail
-- en deux temps (confirmation par la NOUVELLE adresse, annulation par l'ANCIENNE) et
-- gel de la réinitialisation de mot de passe / récupération de passkey après un
-- changement fait par un tiers.
--
-- `ent_email_change` : table volatile (état de sécurité éphémère, exclue des inventaires
-- PRA, comme `ent_recuperation_passkey`). Seuls les SHA-256 des jetons sont stockés.
-- Les adresses y figurent en clair le temps de la demande (nécessaire pour appliquer
-- ou annuler) ; elles ne sont jamais écrites dans le journal.

ALTER TABLE ent_user ADD COLUMN IF NOT EXISTS recuperation_gel_jusqu TIMESTAMPTZ;

CREATE TABLE IF NOT EXISTS ent_email_change (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES ent_user(id) ON DELETE CASCADE,
    etablissement_id UUID NOT NULL,
    actor_user_id UUID NOT NULL,
    -- Changement demandé par un tiers (gel 72 h) ou par la personne elle-même.
    tiers BOOLEAN NOT NULL,
    ancien_email TEXT NOT NULL,
    nouveau_email TEXT NOT NULL,
    statut TEXT NOT NULL DEFAULT 'demandee'
        CHECK (statut IN ('demandee', 'confirmee', 'annulee', 'revertee', 'expiree')),
    confirm_hash TEXT NOT NULL UNIQUE,
    annul_hash TEXT NOT NULL UNIQUE,
    confirm_expires_at TIMESTAMPTZ NOT NULL,
    confirmed_at TIMESTAMPTZ,
    closed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Une seule demande en attente de confirmation par personne.
CREATE UNIQUE INDEX IF NOT EXISTS ent_email_change_demandee_idx
    ON ent_email_change (user_id) WHERE statut = 'demandee';
CREATE INDEX IF NOT EXISTS ent_email_change_created_idx ON ent_email_change (created_at);
