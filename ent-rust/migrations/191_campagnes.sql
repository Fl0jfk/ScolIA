-- Socle 2, lot S2-8 : campagnes d'information (COM-05). Une campagne est
-- préparée (brouillon), prévisualisée (destinataires calculés selon les DROITS
-- de l'auteur et les LIENS actifs), puis lancée après confirmation humaine
-- portant l'empreinte de l'aperçu.

CREATE TABLE IF NOT EXISTS ent_campagne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    titre TEXT NOT NULL,
    corps TEXT NOT NULL,
    canaux TEXT[] NOT NULL,
    audience TEXT NOT NULL,
    cible_ids UUID[] NOT NULL DEFAULT '{}',
    statut TEXT NOT NULL DEFAULT 'brouillon',
    cree_par UUID NOT NULL,
    apercu_empreinte TEXT,
    apercu_nb INTEGER,
    apercu_sms INTEGER,
    apercu_le TIMESTAMPTZ,
    lancee_par UUID,
    lancee_le TIMESTAMPTZ,
    nb_destinataires INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_campagne_audience_chk CHECK (audience IN ('etablissement', 'niveau', 'classe', 'groupe')),
    CONSTRAINT ent_campagne_statut_chk CHECK (statut IN ('brouillon', 'apercu', 'lancee', 'annulee')),
    CONSTRAINT ent_campagne_canaux_chk CHECK (
        cardinality(canaux) BETWEEN 1 AND 3 AND canaux <@ ARRAY['email', 'sms', 'push']::text[]
    ),
    CONSTRAINT ent_campagne_titre_chk CHECK (length(btrim(titre)) > 0 AND length(titre) <= 200),
    CONSTRAINT ent_campagne_corps_chk CHECK (length(btrim(corps)) > 0 AND length(corps) <= 5000),
    CONSTRAINT ent_campagne_cible_chk CHECK (
        (audience = 'etablissement') = (cardinality(cible_ids) = 0)
    )
);

CREATE INDEX IF NOT EXISTS ent_campagne_etab_idx
    ON ent_campagne (etablissement_id, created_at DESC);

CREATE TABLE IF NOT EXISTS ent_campagne_destinataire (
    campagne_id UUID NOT NULL REFERENCES ent_campagne (id) ON DELETE CASCADE,
    etablissement_id UUID NOT NULL,
    user_id UUID NOT NULL REFERENCES ent_user (id) ON DELETE CASCADE,
    -- Un élève représentatif, pour relire le lien au moment de l'envoi.
    eleve_id UUID NOT NULL,
    nb_eleves INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (campagne_id, user_id)
);

CREATE INDEX IF NOT EXISTS ent_campagne_dest_etab_idx
    ON ent_campagne_destinataire (etablissement_id, campagne_id);
