-- 2306 — Socle 15 (absences des personnels), lot A5 : absence longue et suppléance
-- (`docs/socles/15_ABSENCES_PERSONNELS.md` §11.4, R-18, CAP-36, CAP-37, AR-23 ; ABP-180, 181, 239 ; EDT-228).
-- Additive et rejouable (E8) ; aucune colonne JSON (D-036) ; RLS posée.
--
-- 1. `ent_absp_demande_suppleance` : la demande de suppléance faite à l'employeur (rectorat, diocèse, IA-IPR/DPE)
--    pour une absence longue, avec la trace de la réponse et du suppléant proposé (R-18). Aucun texte libre :
--    destinataire codé, référence du courrier bornée, jamais la nature de l'absence.
-- 2. `ent_absp_remplacement` : l'ÉTAT DU REMPLACEMENT d'une absence (en recherche, proposé, nommé, en cours,
--    terminé). Une ligne par absence ; chaque changement d'étape est écrit dans la même transaction que l'événement
--    publié, donc l'événement part une seule fois par étape (ABP-239). Ni nature ni motif.
-- La suppléance elle-même (intervention datée sur le service) reste dans `ent_service_intervenant` (socle 6) ;
-- son `origine_ref` est l'identifiant de l'absence : c'est le lien entre les deux notions de remplacement (AR-23).

CREATE TABLE IF NOT EXISTS ent_absp_demande_suppleance (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    absence_id UUID NOT NULL,
    destinataire TEXT NOT NULL CHECK (destinataire IN ('rectorat', 'ia_dpe', 'diocese', 'autre')),
    reference VARCHAR(60),
    souhaitee_du DATE,
    etat TEXT NOT NULL DEFAULT 'demandee' CHECK (etat IN (
        'demandee', 'proposee', 'acceptee', 'refusee', 'sans_suite', 'annulee')),
    demandee_par_user_id UUID NOT NULL,
    demandee_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    reponse_le TIMESTAMPTZ,
    reponse_par_user_id UUID,
    -- Suppléant proposé par l'employeur : un agent connu de l'établissement (créé au socle 6 si contractuel).
    suppleant_agent_id UUID,
    suppleant_du DATE,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_demande_suppleance_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_demande_suppleance_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_absp_absence (etablissement_id, id),
    CONSTRAINT ent_absp_demande_suppleance_agent_fk FOREIGN KEY (etablissement_id, suppleant_agent_id)
        REFERENCES ent_agent (etablissement_id, id),
    CONSTRAINT ent_absp_demande_suppleance_reponse_chk CHECK (
        (etat IN ('demandee')) = (reponse_le IS NULL) OR etat = 'annulee'),
    CONSTRAINT ent_absp_demande_suppleance_proposee_chk CHECK (
        etat NOT IN ('proposee', 'acceptee') OR suppleant_agent_id IS NOT NULL)
);
-- Une seule demande en cours (sans réponse définitive) par absence.
CREATE UNIQUE INDEX IF NOT EXISTS ent_absp_demande_suppleance_ouverte_uq
    ON ent_absp_demande_suppleance (etablissement_id, absence_id) WHERE etat IN ('demandee', 'proposee');
CREATE INDEX IF NOT EXISTS ent_absp_demande_suppleance_absence_idx
    ON ent_absp_demande_suppleance (etablissement_id, absence_id, demandee_le);

CREATE TABLE IF NOT EXISTS ent_absp_remplacement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    absence_id UUID NOT NULL,
    etape TEXT NOT NULL CHECK (etape IN ('en_recherche', 'propose', 'nomme', 'en_cours', 'termine')),
    -- Suppléant et date de début de la première suppléance nommée (« remplacé par M. Y à partir du … »).
    suppleant_agent_id UUID,
    a_partir_du DATE,
    etape_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_remplacement_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_remplacement_absence_uq UNIQUE (etablissement_id, absence_id),
    CONSTRAINT ent_absp_remplacement_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_absp_absence (etablissement_id, id),
    CONSTRAINT ent_absp_remplacement_agent_fk FOREIGN KEY (etablissement_id, suppleant_agent_id)
        REFERENCES ent_agent (etablissement_id, id),
    CONSTRAINT ent_absp_remplacement_nomme_chk CHECK (
        etape NOT IN ('nomme', 'en_cours') OR (suppleant_agent_id IS NOT NULL AND a_partir_du IS NOT NULL))
);

SELECT ent_rls_poser(ARRAY['ent_absp_demande_suppleance', 'ent_absp_remplacement']);
