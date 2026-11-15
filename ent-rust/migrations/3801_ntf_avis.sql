-- Socle 8, lot N1 — migration 3801 : avis, paramètres d'avis et destinataires.
-- Réf. : docs/socles/08_NOTIFICATIONS.md §19 (3801), §6 (contrat), §8.1–8.2 (cycles de vie), §9.1 (qui).
--
-- Un AVIS est « ce qui s'est passé, pour qui ». Il naît dans la transaction du fait (consommateur
-- `notifications.avis` ou service `aviser`) ; aucune remise avant le commit. Les PARAMÈTRES d'un avis sont des
-- identifiants et des valeurs courtes typées (D-036 : pas de JSON) ; le texte vient d'un gabarit du code, rendu au
-- dernier moment (lot N4). Les DESTINATAIRES sont retenus ou exclus avec leur raison (R-N1) ; le lot N2 les
-- relit à chaque remise.

CREATE TABLE IF NOT EXISTS ent_ntf_avis (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nature TEXT NOT NULL CHECK (nature ~ '^[a-z][a-z0-9_]*\.[a-z][a-z0-9_]*$'),
    nature_version SMALLINT NOT NULL CHECK (nature_version BETWEEN 1 AND 99),
    categorie TEXT NOT NULL,
    urgence TEXT NOT NULL CHECK (urgence IN ('u0', 'u1', 'u2', 'u3')),
    sensibilite TEXT NOT NULL CHECK (sensibilite IN ('c0', 'c1', 'c2', 'c3')),
    sujet_type TEXT NOT NULL CHECK (sujet_type IN ('eleve', 'personne', 'cours', 'etablissement')),
    sujet_id UUID,
    eleve_id UUID,
    -- Idempotence : une même clé pour une même nature ne produit jamais deux avis (rejeu, double clic).
    cle_idempotence TEXT NOT NULL CHECK (length(cle_idempotence) BETWEEN 1 AND 200),
    source_brique TEXT NOT NULL CHECK (length(source_brique) BETWEEN 1 AND 60),
    source_type TEXT NOT NULL CHECK (length(source_type) BETWEEN 1 AND 80),
    source_id UUID,
    cause_event_id UUID,
    statut TEXT NOT NULL DEFAULT 'planifie'
        CHECK (statut IN ('prepare', 'planifie', 'en_cours', 'termine', 'annule', 'gele')),
    effet_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Délai de confirmation de la nature (CN-01) : aucune remise avant cet instant.
    confirme_apres TIMESTAMPTZ NOT NULL DEFAULT now(),
    annule_le TIMESTAMPTZ,
    annule_motif TEXT CHECK (annule_motif IS NULL OR annule_motif ~ '^[a-z][a-z0-9_]{1,59}$'),
    -- R-N59 : les textes des avis d'assiduité sont gardés assez longtemps pour la preuve.
    conservation_preuve BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_ntf_avis_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_ntf_avis_cle_uq UNIQUE (etablissement_id, cle_idempotence, nature),
    CONSTRAINT ent_ntf_avis_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_ntf_avis_annule_chk CHECK ((statut = 'annule') = (annule_le IS NOT NULL))
);
CREATE INDEX IF NOT EXISTS ent_ntf_avis_a_planifier_idx
    ON ent_ntf_avis (etablissement_id, confirme_apres) WHERE statut = 'planifie';
CREATE INDEX IF NOT EXISTS ent_ntf_avis_eleve_idx ON ent_ntf_avis (etablissement_id, eleve_id, nature);

CREATE TABLE IF NOT EXISTS ent_ntf_avis_param (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    avis_id UUID NOT NULL,
    cle TEXT NOT NULL CHECK (cle ~ '^[a-z][a-z0-9_]{0,39}$'),
    -- Identifiants et valeurs courtes seulement : aucune chaîne libre longue (§6.3).
    valeur_texte TEXT CHECK (valeur_texte IS NULL OR length(valeur_texte) <= 64),
    valeur_uuid UUID,
    valeur_jour DATE,
    PRIMARY KEY (avis_id, cle),
    CONSTRAINT ent_ntf_avis_param_une_valeur_chk CHECK (
        (valeur_texte IS NOT NULL)::int + (valeur_uuid IS NOT NULL)::int + (valeur_jour IS NOT NULL)::int = 1),
    CONSTRAINT ent_ntf_avis_param_avis_cfk FOREIGN KEY (etablissement_id, avis_id)
        REFERENCES ent_ntf_avis (etablissement_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS ent_ntf_destinataire (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    avis_id UUID NOT NULL,
    -- Provisoire (N1) : le destinataire est un compte ; le lot N2 pose `personne_id` (personnes sans compte,
    -- organisations) et relit les droits à chaque remise.
    compte_id UUID,
    personne_id UUID,
    organisation_ref UUID,
    raison_code TEXT NOT NULL CHECK (raison_code ~ '^[a-z][a-z0-9_]{1,59}$'),
    retenu BOOLEAN NOT NULL DEFAULT true,
    exclusion_code TEXT CHECK (exclusion_code IS NULL OR exclusion_code ~ '^[a-z][a-z0-9_]{1,59}$'),
    langue TEXT NOT NULL DEFAULT 'fr' CHECK (langue ~ '^[a-z]{2,3}$'),
    statut TEXT NOT NULL DEFAULT 'a_remettre'
        CHECK (statut IN ('a_remettre', 'remis', 'ouvert', 'lu', 'accuse', 'repondu', 'non_joint', 'annule')),
    premier_remis_le TIMESTAMPTZ,
    ouvert_le TIMESTAMPTZ,
    lu_le TIMESTAMPTZ,
    accuse_le TIMESTAMPTZ,
    notifie_le TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_ntf_destinataire_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_ntf_destinataire_avis_cfk FOREIGN KEY (etablissement_id, avis_id)
        REFERENCES ent_ntf_avis (etablissement_id, id) ON DELETE CASCADE,
    CONSTRAINT ent_ntf_destinataire_cible_chk
        CHECK ((compte_id IS NOT NULL)::int + (personne_id IS NOT NULL)::int + (organisation_ref IS NOT NULL)::int >= 1),
    CONSTRAINT ent_ntf_destinataire_exclusion_chk CHECK (retenu = (exclusion_code IS NULL))
);
-- Un destinataire n'est retenu qu'une fois par avis (par compte, ou par personne sans compte).
CREATE UNIQUE INDEX IF NOT EXISTS ent_ntf_destinataire_compte_uq
    ON ent_ntf_destinataire (avis_id, compte_id) WHERE compte_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_ntf_destinataire_personne_uq
    ON ent_ntf_destinataire (avis_id, personne_id) WHERE compte_id IS NULL AND personne_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_ntf_destinataire_orga_uq
    ON ent_ntf_destinataire (avis_id, organisation_ref)
    WHERE compte_id IS NULL AND personne_id IS NULL AND organisation_ref IS NOT NULL;

SELECT ent_rls_poser(ARRAY[
    'ent_ntf_avis',
    'ent_ntf_avis_param',
    'ent_ntf_destinataire'
]);
