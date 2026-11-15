-- Socle 8 (notifications et diffusion), lot N1 — migration 3800 : paramètres de l'établissement et réglage par nature.
-- Réf. : docs/socles/08_NOTIFICATIONS.md §19 (3800), §9.3 (fenêtres de silence), §27 (lot N1).
--
-- Additive et rejouable. Aucune donnée semi-structurée ni tableau (D-036) : les jours calmes sont un masque de
-- bits (bit 0 = lundi … bit 6 = dimanche), la disponibilité des personnels (JSONB au §19) relève du lot N12.
-- Les natures et les gabarits restent dans le code (comme les actions) : la base ne garde que le réglage
-- d'un établissement pour une nature (désactivation, bascule depuis l'ancien chemin, texte encadré).

CREATE TABLE IF NOT EXISTS ent_ntf_parametre (
    etablissement_id UUID PRIMARY KEY
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    -- Heures calmes de l'établissement (fuseau de l'établissement, jamais UTC) : lues par le lot N3.
    calme_debut TIME NOT NULL DEFAULT '20:30',
    calme_fin TIME NOT NULL DEFAULT '07:00',
    jours_calmes_masque SMALLINT NOT NULL DEFAULT 64 CHECK (jours_calmes_masque BETWEEN 0 AND 127),
    vacances_mode TEXT NOT NULL DEFAULT 'normal' CHECK (vacances_mode IN ('normal', 'hebdo')),
    -- Délai de confirmation d'une absence constatée avant de prévenir (CN-01, R-N12) ; 0 = tout de suite.
    delai_confirmation_absence_min SMALLINT NOT NULL DEFAULT 20
        CHECK (delai_confirmation_absence_min BETWEEN 0 AND 240),
    escalade_sms_min SMALLINT NOT NULL DEFAULT 30 CHECK (escalade_sms_min BETWEEN 5 AND 1440),
    sms_u2_actif BOOLEAN NOT NULL DEFAULT false,
    maj_par UUID,
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS ent_ntf_nature_etab (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nature TEXT NOT NULL CHECK (nature ~ '^[a-z][a-z0-9_]*\.[a-z][a-z0-9_]*$'),
    desactivee BOOLEAN NOT NULL DEFAULT false,
    -- Bascule progressive (E1, §26) : tant que la valeur est fausse, l'ancien chemin prévient seul ; quand elle
    -- devient vraie pour cet établissement, la brique prévient et l'ancien chemin se tait (jamais les deux).
    via_brique BOOLEAN NOT NULL DEFAULT false,
    -- Phrase ajoutée par l'établissement (encadrée, jamais le texte principal) ; contrôlée par le service.
    texte_encadre TEXT CHECK (texte_encadre IS NULL OR (length(btrim(texte_encadre)) BETWEEN 1 AND 300
                                                       AND texte_encadre !~ '[<>]')),
    maj_par UUID,
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, nature)
);

SELECT ent_rls_poser(ARRAY[
    'ent_ntf_parametre',
    'ent_ntf_nature_etab'
]);
