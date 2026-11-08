-- Socle 4, lot A0 bis, A0-b et A0-c (R-A39). Contrat `DeclarationResponsables` v1 : champs
-- réservés, facultatifs dès A0, stockés ici pour qu'aucun ne soit perdu en silence.
--
--  * `ne_pas_solliciter` : AUCUNE invitation, demande d'accord ni relance vers cette personne
--    (une mère protégée par une ordonnance ne doit pas révéler l'établissement de l'enfant).
--    `ne_pas_solliciter_ref` = référence de la pièce (ordonnance…) détenue par la direction,
--    SCELLÉE (AES-256-GCM, `champs_chiffres`), jamais lisible en clair en base.
--    Posée par une déclaration de la famille : « à confirmer » par la direction.
--  * `langue`, `besoin_interprete`, `organisme` (ex. service de l'aide sociale à l'enfance
--    titulaire), `nature_code` (vocabulaire du socle 4, rempli par le lot A1).
-- Migration additive et rejouable.

CREATE TABLE IF NOT EXISTS ent_personne_reserve (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    personne_id UUID NOT NULL,
    ne_pas_solliciter BOOLEAN NOT NULL DEFAULT false,
    ne_pas_solliciter_ref TEXT,
    ne_pas_solliciter_le TIMESTAMPTZ,
    ne_pas_solliciter_par UUID,
    -- Confirmation par la direction d'une protection déclarée par la famille.
    ne_pas_solliciter_confirme_le TIMESTAMPTZ,
    ne_pas_solliciter_confirme_par UUID,
    langue TEXT CHECK (langue IS NULL OR langue ~ '^[a-z]{2,3}(-[A-Za-z0-9]{2,8})?$'),
    besoin_interprete BOOLEAN NOT NULL DEFAULT false,
    organisme TEXT CHECK (organisme IS NULL OR length(btrim(organisme)) BETWEEN 1 AND 200),
    nature_code TEXT CHECK (nature_code IS NULL OR nature_code ~ '^[a-z][a-z0-9_]{0,39}$'),
    contrat_version SMALLINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_personne_reserve_personne_uq UNIQUE (etablissement_id, personne_id),
    CONSTRAINT ent_personne_reserve_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ent_personne_reserve_nps_idx
    ON ent_personne_reserve (etablissement_id) WHERE ne_pas_solliciter;
