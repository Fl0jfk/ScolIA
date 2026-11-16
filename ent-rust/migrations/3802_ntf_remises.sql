-- Socle 8, lot N1 — migration 3802 : remises (une par destinataire et canal) et résumés.
-- Réf. : docs/socles/08_NOTIFICATIONS.md §19 (3802), §8.3 (cycle d'une remise), §12 (échecs, reprises).
--
-- Une REMISE est un essai d'atteindre un destinataire par un canal. En N1 : l'ENT (`inapp`) est remis en
-- écrivant `ent_notification` ; les canaux externes sont transmis à l'ancien moteur (`fournisseur =
-- 'ancien_moteur'`) qui garde les préférences, les heures calmes et les plafonds jusqu'aux lots N3 et N5–N7.
-- Aucune adresse ni aucun numéro n'est copié ici : la coordonnée est relue au moment de l'expédition.

CREATE TABLE IF NOT EXISTS ent_ntf_remise (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    destinataire_id UUID NOT NULL,
    canal TEXT NOT NULL CHECK (canal IN ('inapp', 'push', 'email', 'sms', 'lien', 'courrier', 'appel')),
    planifiee_pour TIMESTAMPTZ NOT NULL,
    motif_planification TEXT NOT NULL DEFAULT 'immediat' CHECK (motif_planification ~ '^[a-z][a-z0-9_]{1,59}$'),
    statut TEXT NOT NULL DEFAULT 'planifiee'
        CHECK (statut IN ('planifiee', 'reservee', 'transmise', 'remise', 'echec_temporaire', 'echec_definitif',
                          'ignoree', 'annulee', 'plafonnee')),
    motif TEXT CHECK (motif IS NULL OR motif ~ '^[a-z][a-z0-9_]{1,59}$'),
    tentatives SMALLINT NOT NULL DEFAULT 0,
    fournisseur TEXT CHECK (fournisseur IS NULL OR fournisseur ~ '^[a-z][a-z0-9_]{1,39}$'),
    -- Lien vers la notification dans l'ENT ou l'envoi de l'ancien moteur (façade N1, retirée au lot N11).
    notification_id UUID,
    transmise_le TIMESTAMPTZ,
    remise_le TIMESTAMPTZ,
    echec_code TEXT CHECK (echec_code IS NULL OR echec_code ~ '^[a-z][a-z0-9_]{1,59}$'),
    escalade_de UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_ntf_remise_etab_id_uq UNIQUE (etablissement_id, id),
    -- Idempotence : une remise par destinataire et canal (l'escalade du lot N8 en crée une autre, liée).
    CONSTRAINT ent_ntf_remise_dest_canal_uq UNIQUE (destinataire_id, canal),
    CONSTRAINT ent_ntf_remise_dest_cfk FOREIGN KEY (etablissement_id, destinataire_id)
        REFERENCES ent_ntf_destinataire (etablissement_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ent_ntf_remise_a_faire_idx
    ON ent_ntf_remise (planifiee_pour) WHERE statut = 'planifiee';

CREATE TABLE IF NOT EXISTS ent_ntf_resume (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    compte_id UUID NOT NULL,
    canal TEXT NOT NULL CHECK (canal IN ('push', 'email')),
    periode_debut TIMESTAMPTZ NOT NULL,
    periode_fin TIMESTAMPTZ NOT NULL,
    statut TEXT NOT NULL DEFAULT 'a_composer' CHECK (statut IN ('a_composer', 'compose', 'remis', 'annule')),
    remise_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_ntf_resume_periode_chk CHECK (periode_fin > periode_debut),
    CONSTRAINT ent_ntf_resume_remise_cfk FOREIGN KEY (etablissement_id, remise_id)
        REFERENCES ent_ntf_remise (etablissement_id, id) ON DELETE SET NULL (remise_id)
);

SELECT ent_rls_poser(ARRAY[
    'ent_ntf_remise',
    'ent_ntf_resume'
]);
