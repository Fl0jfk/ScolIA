-- Socle 2, lot S2-0 : outbox transactionnelle et consommateurs idempotents.
--
-- Un événement est écrit dans la MÊME transaction que le fait métier :
--   * `ent_metier_event` reste le journal (contrat existant, lecture inchangée) ;
--   * `ent_outbox` porte l'état de distribution (une ligne par événement) ;
--   * `ent_outbox_agregat` attribue le numéro d'ordre par agrégat ;
--   * `ent_event_traitement` rend chaque consommateur idempotent ;
--   * `ent_notification` est le premier consommateur (D0 : notification interne).
-- Migration additive : aucune table existante n'est modifiée.

CREATE TABLE IF NOT EXISTS ent_outbox_agregat (
    etablissement_id UUID NOT NULL,
    aggregate TEXT NOT NULL,
    aggregate_id UUID NOT NULL,
    dernier_seq BIGINT NOT NULL,
    PRIMARY KEY (etablissement_id, aggregate, aggregate_id)
);

CREATE TABLE IF NOT EXISTS ent_outbox (
    event_id UUID PRIMARY KEY REFERENCES ent_metier_event (id) ON DELETE CASCADE,
    -- Ordre global technique (lots) ; l'ordre métier est `seq_agregat`.
    seq BIGSERIAL NOT NULL UNIQUE,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    type TEXT NOT NULL CHECK (btrim(type) <> ''),
    aggregate TEXT NOT NULL,
    aggregate_id UUID NOT NULL,
    seq_agregat BIGINT NOT NULL CHECK (seq_agregat >= 1),
    schema_version SMALLINT NOT NULL DEFAULT 1,
    correlation_id UUID,
    causation_id UUID,
    profondeur SMALLINT NOT NULL DEFAULT 0 CHECK (profondeur BETWEEN 0 AND 5),
    -- a_distribuer → distribue ; après le plafond de tentatives → echec (file des échecs).
    statut TEXT NOT NULL DEFAULT 'a_distribuer'
        CHECK (statut IN ('a_distribuer', 'distribue', 'echec')),
    tentatives INT NOT NULL DEFAULT 0,
    prochaine_tentative TIMESTAMPTZ NOT NULL DEFAULT now(),
    derniere_erreur TEXT,
    distribue_le TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_outbox_ordre_agregat_uq
        UNIQUE (etablissement_id, aggregate, aggregate_id, seq_agregat)
);

CREATE INDEX IF NOT EXISTS ent_outbox_a_distribuer_idx
    ON ent_outbox (prochaine_tentative, seq) WHERE statut = 'a_distribuer';

CREATE INDEX IF NOT EXISTS ent_outbox_echec_idx
    ON ent_outbox (etablissement_id, seq) WHERE statut = 'echec';

CREATE TABLE IF NOT EXISTS ent_event_traitement (
    consommateur TEXT NOT NULL,
    event_id UUID NOT NULL REFERENCES ent_outbox (event_id) ON DELETE CASCADE,
    etablissement_id UUID NOT NULL,
    traite_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (consommateur, event_id)
);

CREATE INDEX IF NOT EXISTS ent_event_traitement_etab_idx
    ON ent_event_traitement (etablissement_id, consommateur);

CREATE TABLE IF NOT EXISTS ent_notification (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    destinataire_user_id UUID NOT NULL REFERENCES ent_user (id) ON DELETE CASCADE,
    eleve_id UUID,
    kind TEXT NOT NULL,
    titre TEXT NOT NULL,
    corps TEXT NOT NULL,
    source_event_id UUID NOT NULL,
    lu_le TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_notification_unique UNIQUE (source_event_id, destinataire_user_id, kind)
);

CREATE INDEX IF NOT EXISTS ent_notification_destinataire_idx
    ON ent_notification (etablissement_id, destinataire_user_id, created_at DESC);
