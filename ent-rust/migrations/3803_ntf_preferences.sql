-- Socle 8, lot N3 (préférences et temps) — migration 3803 : préférences par catégorie, réglage de la personne,
-- budget de bruit, et reprise de `ent_preference_notification`.
-- Réf. : docs/socles/08_NOTIFICATIONS.md §19 (3803), §9.2 (canaux), §9.3 (quand), §9.4 (combien), CN-49.
--
-- Additive et rejouable. Aucune donnée semi-structurée ni tableau (D-036) : les heures calmes de la personne sont
-- deux heures et un masque de jours (bit 0 = lundi … bit 6 = dimanche). Les préférences sont par PERSONNE (la
-- personne survit au compte, 120_personne) et par CATÉGORIE (dix, `ntf::contrat::Categorie`).
-- L'indisponibilité d'un personnel (troisième étage des heures calmes) relève du lot N12.

-- Budget de bruit de l'établissement (R-N24) : au-delà de N avis U0 par personne et par jour, le résumé.
ALTER TABLE ent_ntf_parametre ADD COLUMN IF NOT EXISTS budget_bruit_u0 SMALLINT NOT NULL DEFAULT 5
    CHECK (budget_bruit_u0 BETWEEN 1 AND 50);

CREATE TABLE IF NOT EXISTS ent_ntf_reglage_personne (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    personne_id UUID NOT NULL,
    preset TEXT NOT NULL DEFAULT 'equilibre' CHECK (preset IN ('essentiel', 'equilibre', 'tout', 'discret')),
    -- Heures calmes de la personne (deuxième étage) : les deux ou aucune ; la plage peut passer minuit.
    calme_debut TIME,
    calme_fin TIME,
    jours_calmes_masque SMALLINT CHECK (jours_calmes_masque IS NULL OR jours_calmes_masque BETWEEN 0 AND 127),
    resume_mode TEXT NOT NULL DEFAULT 'immediat' CHECK (resume_mode IN ('immediat', 'quotidien', 'hebdo')),
    resume_heure TIME NOT NULL DEFAULT '18:00',
    regrouper_fratrie BOOLEAN NOT NULL DEFAULT false,
    apercus_verrouille BOOLEAN NOT NULL DEFAULT false,
    -- Pause « je suis en voyage » : retient U0 et U1, jamais U2 ni U3.
    pause_jusqua TIMESTAMPTZ,
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, personne_id),
    CONSTRAINT ent_ntf_reglage_personne_calme_chk CHECK ((calme_debut IS NULL) = (calme_fin IS NULL)),
    CONSTRAINT ent_ntf_reglage_personne_cfk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS ent_ntf_preference (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    personne_id UUID NOT NULL,
    categorie TEXT NOT NULL CHECK (categorie IN ('vie_scolaire', 'evaluations', 'sante', 'finances', 'documents',
                                                  'messagerie', 'agenda', 'inscriptions', 'securite', 'systeme')),
    push BOOLEAN NOT NULL DEFAULT true,
    email BOOLEAN NOT NULL DEFAULT true,
    -- SMS : consentement explicite (désactivé tant que la personne ne l'a pas choisi).
    sms BOOLEAN NOT NULL DEFAULT false,
    resume BOOLEAN NOT NULL DEFAULT false,
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, personne_id, categorie),
    CONSTRAINT ent_ntf_preference_cfk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE CASCADE
);

-- Reprise sans perte de `ent_preference_notification` (une ligne par compte) vers la personne du compte :
-- canaux (les trois booléens valent pour toutes les catégories), résumé quotidien (heure historique 7 h 30,
-- conservée), heures calmes (minutes → heures). Rejouable : ce qui existe déjà n'est pas écrasé.
INSERT INTO ent_ntf_reglage_personne
    (etablissement_id, personne_id, preset, calme_debut, calme_fin, resume_mode, resume_heure, maj_le)
SELECT l.etablissement_id, pe.id, 'equilibre',
       CASE WHEN l.calme_debut IS NULL THEN NULL ELSE make_time(l.calme_debut / 60, l.calme_debut % 60, 0) END,
       CASE WHEN l.calme_fin IS NULL THEN NULL ELSE make_time(l.calme_fin / 60, l.calme_fin % 60, 0) END,
       l.resume, TIME '07:30', l.updated_at
  FROM ent_preference_notification l
  JOIN ent_personne pe ON pe.etablissement_id = l.etablissement_id AND pe.user_id = l.user_id
 WHERE pe.fusionnee_dans IS NULL
ON CONFLICT (etablissement_id, personne_id) DO NOTHING;

INSERT INTO ent_ntf_preference (etablissement_id, personne_id, categorie, push, email, sms, resume, maj_le)
SELECT l.etablissement_id, pe.id, c.categorie, l.canal_push, l.canal_email, l.canal_sms, l.resume = 'quotidien', l.updated_at
  FROM ent_preference_notification l
  JOIN ent_personne pe ON pe.etablissement_id = l.etablissement_id AND pe.user_id = l.user_id
 CROSS JOIN (VALUES ('vie_scolaire'), ('evaluations'), ('sante'), ('finances'), ('documents'), ('messagerie'),
                    ('agenda'), ('inscriptions'), ('securite'), ('systeme')) AS c (categorie)
 WHERE pe.fusionnee_dans IS NULL
ON CONFLICT (etablissement_id, personne_id, categorie) DO NOTHING;

SELECT ent_rls_poser(ARRAY[
    'ent_ntf_reglage_personne',
    'ent_ntf_preference'
]);
