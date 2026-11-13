-- 2305 — Socle 15 (absences des personnels), lot A4 : sollicitation d'un remplaçant
-- (`docs/socles/15_ABSENCES_PERSONNELS.md` §11.3, §11.5, R-AP19, R-AP24, R-AP41, CAP-31 ; ABP-169, 175 à 178, 185, 186).
-- Additive et rejouable (E8) ; aucune colonne JSON (D-036) ; RLS posée.
--
-- `ent_absp_sollicitation` : la demande faite à un agent de couvrir un impact (séance), son sort (acceptée,
-- refusée, expirée, annulée) ou la désignation directe d'urgence (sans sollicitation préalable, avec accusé de
-- réception). Elle ne porte JAMAIS la nature de l'absence : seulement l'impact, l'agent, les dates.
--
-- **Garantie de fond (R-AP19, AR-09, CAP-31)** : un agent n'a jamais deux séances acceptées ou désignées qui se
-- chevauchent — contrainte d'exclusion EN BASE sur `(établissement, agent, plage)`. Deux acceptations
-- simultanées ne peuvent pas passer toutes les deux, quel que soit le chemin (service, import, script).
CREATE TABLE IF NOT EXISTS ent_absp_sollicitation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    impact_id UUID NOT NULL,
    agent_id UUID NOT NULL,
    mode TEXT NOT NULL CHECK (mode IN ('sollicitation', 'designation', 'appel_volontaires')),
    -- Appel à volontaires : toutes les sollicitations du même appel partagent ce groupe ; la première
    -- acceptée retient la séance, les autres sont annulées (« prise »).
    groupe_id UUID,
    sollicite_par_user_id UUID NOT NULL,
    -- Rôle de la personne qui a sollicité, au moment de la demande : l'acceptation applique la réponse à
    -- l'emploi du temps en son nom, avec ses droits.
    sollicite_par_role TEXT NOT NULL,
    envoyee_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Plage calme (§11.3) : envoi différé à 6 h 30 hors de la fenêtre de service ; NULL = envoyée maintenant.
    envoi_prevu_le TIMESTAMPTZ,
    expire_le TIMESTAMPTZ,
    etat TEXT NOT NULL DEFAULT 'envoyee' CHECK (etat IN (
        'envoyee', 'acceptee', 'refusee', 'expiree', 'annulee', 'designee')),
    repondue_le TIMESTAMPTZ,
    -- Motif légitime codé (refus d'un volontaire RCD, lot A6) ; libre et sans motif pour un collègue.
    refus_code TEXT CHECK (refus_code IS NULL OR refus_code IN ('absent', 'autre_service', 'delai', 'autre')),
    -- Pourquoi annulée : `prise` (un autre volontaire a accepté), `levee`, `echec` (réponse non applicable).
    annulee_cause TEXT CHECK (annulee_cause IS NULL OR annulee_cause IN ('prise', 'levee', 'echec', 'absence_annulee')),
    accuse_le TIMESTAMPTZ,
    contestee_le TIMESTAMPTZ,
    plage TSTZRANGE NOT NULL,
    reponse_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_sollicitation_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_sollicitation_plage_chk CHECK (NOT isempty(plage)),
    CONSTRAINT ent_absp_sollicitation_impact_fk FOREIGN KEY (etablissement_id, impact_id)
        REFERENCES ent_absp_impact (etablissement_id, id),
    CONSTRAINT ent_absp_sollicitation_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id),
    CONSTRAINT ent_absp_sollicitation_reponse_fk FOREIGN KEY (etablissement_id, reponse_id)
        REFERENCES ent_absp_reponse (etablissement_id, id),
    CONSTRAINT ent_absp_sollicitation_pas_de_chevauchement
        EXCLUDE USING gist (etablissement_id WITH =, agent_id WITH =, plage WITH &&)
        WHERE (etat IN ('acceptee', 'designee'))
);
-- Une seule demande ouverte par impact (hors appel à volontaires) ; une seule acceptation ou désignation.
CREATE UNIQUE INDEX IF NOT EXISTS ent_absp_sollicitation_ouverte_uq
    ON ent_absp_sollicitation (etablissement_id, impact_id)
    WHERE etat = 'envoyee' AND mode <> 'appel_volontaires';
CREATE UNIQUE INDEX IF NOT EXISTS ent_absp_sollicitation_retenue_uq
    ON ent_absp_sollicitation (etablissement_id, impact_id) WHERE etat IN ('acceptee', 'designee');
CREATE INDEX IF NOT EXISTS ent_absp_sollicitation_agent_idx
    ON ent_absp_sollicitation (etablissement_id, agent_id, etat);
CREATE INDEX IF NOT EXISTS ent_absp_sollicitation_expire_idx
    ON ent_absp_sollicitation (etablissement_id, expire_le) WHERE etat = 'envoyee';

-- S-1505 : délais d'expiration (20 minutes pour une séance du jour même, 12 heures sinon).
ALTER TABLE ent_absp_param ADD COLUMN IF NOT EXISTS sollicitation_expire_jour_meme_min SMALLINT NOT NULL DEFAULT 20
    CHECK (sollicitation_expire_jour_meme_min BETWEEN 5 AND 240);
ALTER TABLE ent_absp_param ADD COLUMN IF NOT EXISTS sollicitation_expire_autre_h SMALLINT NOT NULL DEFAULT 12
    CHECK (sollicitation_expire_autre_h BETWEEN 1 AND 72);

SELECT ent_rls_poser(ARRAY['ent_absp_sollicitation']);
