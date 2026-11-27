-- 097_recuperation_passkey.sql — Récupération de passkey en libre-service (D-018).
--
-- Une demande par parcours : e-mail de confirmation -> délai de sécurité ->
-- finalisation (suppression des passkeys + révocation des sessions). Trois
-- jetons distincts de 256 bits, seuls leurs SHA-256 sont stockés ; aucun
-- e-mail en clair. Une seule demande ACTIVE (demandee / en_attente) par
-- personne, garantie par l'index unique partiel. Table volatile (état de
-- sécurité éphémère, exclue des inventaires PRA).

CREATE TABLE IF NOT EXISTS ent_recuperation_passkey (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES ent_user(id) ON DELETE CASCADE,
    -- Établissement choisi à la demande (information ; l'identité est globale).
    etablissement_id UUID NOT NULL,
    statut TEXT NOT NULL DEFAULT 'demandee'
        CHECK (statut IN ('demandee', 'en_attente', 'annulee', 'terminee', 'expiree')),
    confirm_hash TEXT NOT NULL UNIQUE,
    confirm_expires_at TIMESTAMPTZ NOT NULL,
    annul_hash TEXT UNIQUE,
    final_hash TEXT UNIQUE,
    confirmed_at TIMESTAMPTZ,
    -- Fin du délai de sécurité : la finalisation n'est possible qu'après.
    effective_at TIMESTAMPTZ,
    final_expires_at TIMESTAMPTZ,
    closed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_recuperation_passkey_active_idx
    ON ent_recuperation_passkey (user_id)
    WHERE statut IN ('demandee', 'en_attente');

CREATE INDEX IF NOT EXISTS ent_recuperation_passkey_created_idx
    ON ent_recuperation_passkey (created_at);
