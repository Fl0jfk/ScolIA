-- 649 — Socle 42, lot V1 (2/2) : liste nominative transmise à un prestataire (VOY-271, R-SV41, VOY-289) ;
-- corrige VD-16 (contacts des parents de toute la liste envoyés au transporteur, INE à la cuisine).
-- Réf. : 42_SORTIES_VOYAGES.md §13.2, §14, §32.7, Q-V11, Q-V43.
--
-- Exigences E1 à E14 (PRA) : `etablissement_id` NOT NULL partout, clés étrangères COMPOSITES, UUID fournis par
-- l'application, aucun BYTEA, aucun JSON (D-036), horodatages, migration rejouable (aucune modification des
-- migrations 640 à 648), RLS posée en fin de fichier.
--
-- * Par défaut un prestataire ne reçoit que des EFFECTIFS (V0c). Une liste nominative n'est transmise qu'avec
--   les champs exigés par le contrat, un motif, la validation de la DIRECTION et la trace (qui, quand, quels
--   champs, quel pays).
-- * Les champs possibles sont une liste FERMÉE par la base : aucun contact de famille, aucun INE, aucune
--   donnée de santé n'y figure et ne peut y figurer (la contrainte refuse tout autre code).
-- * Un destinataire hors de l'Union européenne est signalé (VOY-289) et doit être confirmé.

CREATE TABLE IF NOT EXISTS ent_sortie_transmission (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    prestataire_id UUID NOT NULL,
    motif TEXT NOT NULL CHECK (length(btrim(motif)) BETWEEN 1 AND 500),
    pays_destinataire TEXT NOT NULL CHECK (length(btrim(pays_destinataire)) BETWEEN 2 AND 80),
    hors_ue BOOLEAN NOT NULL,
    nb_lignes INTEGER NOT NULL DEFAULT 0 CHECK (nb_lignes >= 0),
    statut TEXT NOT NULL DEFAULT 'a_valider' CHECK (statut IN ('a_valider', 'validee', 'refusee')),
    demande_par_user_id UUID NOT NULL,
    demande_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    decide_par_user_id UUID,
    decide_le TIMESTAMPTZ,
    motif_refus TEXT CHECK (motif_refus IS NULL OR length(btrim(motif_refus)) BETWEEN 1 AND 500),
    document_id UUID,
    envoi_id UUID,
    CONSTRAINT ent_sortie_transmission_decision_chk CHECK (
        (statut = 'a_valider') = (decide_par_user_id IS NULL AND decide_le IS NULL)),
    CONSTRAINT ent_sortie_transmission_validee_chk CHECK (statut <> 'validee' OR (document_id IS NOT NULL AND envoi_id IS NOT NULL)),
    CONSTRAINT ent_sortie_transmission_refus_chk CHECK ((statut = 'refusee') = (motif_refus IS NOT NULL)),
    CONSTRAINT ent_sortie_transmission_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_transmission_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_transmission_prestataire_fk FOREIGN KEY (etablissement_id, prestataire_id)
        REFERENCES ent_prestataire (etablissement_id, id),
    CONSTRAINT ent_sortie_transmission_document_fk FOREIGN KEY (etablissement_id, document_id)
        REFERENCES ent_sortie_document (etablissement_id, id),
    CONSTRAINT ent_sortie_transmission_envoi_fk FOREIGN KEY (etablissement_id, envoi_id)
        REFERENCES ent_sortie_envoi (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_transmission_sortie_idx
    ON ent_sortie_transmission (etablissement_id, sortie_id, demande_le);

-- Champs transmis : liste FERMÉE (jamais de contact, d'INE ni de santé).
CREATE TABLE IF NOT EXISTS ent_sortie_transmission_champ (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    transmission_id UUID NOT NULL,
    champ TEXT NOT NULL CHECK (champ IN ('nom', 'prenom', 'classe', 'date_naissance', 'sexe')),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_transmission_champ_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_transmission_champ_uq UNIQUE (transmission_id, champ),
    CONSTRAINT ent_sortie_transmission_champ_fk FOREIGN KEY (etablissement_id, transmission_id)
        REFERENCES ent_sortie_transmission (etablissement_id, id)
);

-- L'envoi d'une liste nominative passe par la file d'envoi des prestataires (V0c) : nouveau type.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_sortie_envoi_type_v1_chk') THEN
        ALTER TABLE ent_sortie_envoi DROP CONSTRAINT IF EXISTS ent_sortie_envoi_type_check;
        ALTER TABLE ent_sortie_envoi ADD CONSTRAINT ent_sortie_envoi_type_v1_chk CHECK (
            type IN ('demande_devis', 'relance', 'bon_commande', 'avenant', 'liste_nominative'));
    END IF;
END
$$;

SELECT ent_rls_poser(ARRAY['ent_sortie_transmission', 'ent_sortie_transmission_champ']);
