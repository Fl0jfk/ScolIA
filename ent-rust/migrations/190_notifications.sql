-- Socle 2, lot S2-8 : domaine Notifications unifié (D0, PLT-05) — préférences
-- par personne, envois par canal tracés et dédoublonnés, plafonds.
-- Additive : aucune table existante modifiée. Le canal « dans l'ENT » est
-- `ent_notification` (S2-0), toujours actif et non désactivable.

CREATE TABLE IF NOT EXISTS ent_preference_notification (
    etablissement_id UUID NOT NULL,
    user_id UUID NOT NULL REFERENCES ent_user (id) ON DELETE CASCADE,
    canal_email BOOLEAN NOT NULL DEFAULT true,
    -- SMS : consentement explicite (désactivé tant que la personne ne l'a pas choisi).
    canal_sms BOOLEAN NOT NULL DEFAULT false,
    canal_push BOOLEAN NOT NULL DEFAULT true,
    -- « quotidien » : les messages regroupables d'un canal sont réunis en un résumé.
    resume TEXT NOT NULL DEFAULT 'immediat',
    -- Heures calmes (heure de Paris), en minutes depuis minuit ; plage pouvant
    -- passer minuit (début > fin).
    calme_debut SMALLINT,
    calme_fin SMALLINT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, user_id),
    CONSTRAINT ent_pref_notif_resume_chk CHECK (resume IN ('immediat', 'quotidien')),
    CONSTRAINT ent_pref_notif_calme_chk CHECK (
        (calme_debut IS NULL AND calme_fin IS NULL)
        OR (calme_debut BETWEEN 0 AND 1439 AND calme_fin BETWEEN 0 AND 1439
            AND calme_debut <> calme_fin)
    )
);

CREATE TABLE IF NOT EXISTS ent_notification_plafond (
    etablissement_id UUID PRIMARY KEY,
    sms_par_jour INTEGER NOT NULL DEFAULT 200 CHECK (sms_par_jour >= 0),
    sms_par_personne_jour INTEGER NOT NULL DEFAULT 3 CHECK (sms_par_personne_jour >= 0),
    sms_par_campagne INTEGER NOT NULL DEFAULT 300 CHECK (sms_par_campagne >= 0),
    campagnes_par_jour INTEGER NOT NULL DEFAULT 5 CHECK (campagnes_par_jour >= 0),
    updated_by UUID,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS ent_notification_envoi (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    destinataire_user_id UUID NOT NULL REFERENCES ent_user (id) ON DELETE CASCADE,
    canal TEXT NOT NULL,
    kind TEXT NOT NULL,
    titre TEXT NOT NULL,
    corps TEXT NOT NULL,
    eleve_id UUID,
    notification_id UUID REFERENCES ent_notification (id) ON DELETE SET NULL,
    source_type TEXT NOT NULL,
    source_id UUID NOT NULL,
    -- Dédoublonnage : un même fait ne produit qu'un envoi par destinataire et canal.
    cle_dedup TEXT NOT NULL,
    -- Droit relu au moment de l'envoi (le lien peut avoir été suspendu depuis).
    droit_requis TEXT,
    regroupable BOOLEAN NOT NULL DEFAULT false,
    statut TEXT NOT NULL DEFAULT 'en_attente',
    motif TEXT,
    planifie_pour TIMESTAMPTZ NOT NULL DEFAULT now(),
    tentative_le TIMESTAMPTZ,
    traite_le TIMESTAMPTZ,
    resume_id UUID,
    message_id TEXT,
    -- SMS : emplacement réservé dans les plafonds du jour avant l'appel au fournisseur.
    plafond_reserve BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_notif_envoi_canal_chk CHECK (canal IN ('email', 'sms', 'push')),
    CONSTRAINT ent_notif_envoi_source_chk CHECK (
        source_type IN ('evenement', 'campagne', 'rencontre', 'systeme')
    ),
    CONSTRAINT ent_notif_envoi_droit_chk CHECK (
        droit_requis IS NULL OR droit_requis IN ('lien', 'peut_voir_vs', 'peut_voir_notes')
    ),
    CONSTRAINT ent_notif_envoi_statut_chk CHECK (
        statut IN ('en_attente', 'envoi', 'envoye', 'simule', 'echec', 'ignore', 'plafonne', 'regroupe')
    ),
    CONSTRAINT ent_notif_envoi_dedup_uq
        UNIQUE (etablissement_id, destinataire_user_id, canal, cle_dedup)
);

CREATE INDEX IF NOT EXISTS ent_notif_envoi_a_envoyer_idx
    ON ent_notification_envoi (planifie_pour) WHERE statut = 'en_attente';
CREATE INDEX IF NOT EXISTS ent_notif_envoi_source_idx
    ON ent_notification_envoi (etablissement_id, source_type, source_id);
CREATE INDEX IF NOT EXISTS ent_notif_envoi_dest_idx
    ON ent_notification_envoi (etablissement_id, destinataire_user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS ent_notif_envoi_sms_jour_idx
    ON ent_notification_envoi (etablissement_id, destinataire_user_id, tentative_le)
    WHERE canal = 'sms' AND plafond_reserve;
