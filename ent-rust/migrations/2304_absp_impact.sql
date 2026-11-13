-- 2304 — Socle 15 (absences des personnels), lot A3 : impact d'une absence
-- (`docs/socles/15_ABSENCES_PERSONNELS.md` §10, §26, R-AP23, R-AP25, CAP-05, 06, 25, 26, 27, 46 ; ABP-150 à 157, 165 à 174, 189).
-- Additive et rejouable (E8) ; aucune colonne JSON (D-036) ; RLS posée sur les nouvelles tables.
--
-- 1. `ent_absp_impact` : une ligne par séance (plus tard par poste ou engagement d'un autre socle) qu'une
--    absence touche, avec son état de traitement (§10.2). Calculée par le serveur à chaque changement de
--    l'absence et de l'emploi du temps ; recalculable à volonté (idempotent). La séance est une RÉFÉRENCE
--    SOUPLE vers `ent_edt_seance` (socle 10), sans clé étrangère : le 10 peut recalculer ou supprimer une
--    séance, le 15 le constate au recalcul (CAP-46) et ne bloque jamais l'emploi du temps. Le 15 n'écrit
--    JAMAIS dans les tables du 10 (règle 1 de la carte).
-- 2. `ent_absp_reponse` : la réponse posée sur un impact (remplacement, annulation, continuité…). Créée ici
--    pour la reprise des liens `ent_edt_exception.absence_rh_id` (remplacements et annulations déjà posés
--    par les écrans existants) ; le lot A4 y ajoute les propositions et les sollicitations.
-- Ni la nature de l'absence ni son motif n'apparaissent dans ces tables (niveau « organisation », R-AP2).

CREATE TABLE IF NOT EXISTS ent_absp_impact (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    absence_id UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('seance', 'poste', 'engagement')),
    seance_id UUID,
    poste_ref UUID,
    engagement_socle TEXT CHECK (engagement_socle IS NULL OR engagement_socle IN ('19', '23', '38', '42')),
    engagement_ref UUID,
    debut TIMESTAMPTZ NOT NULL,
    fin TIMESTAMPTZ NOT NULL,
    etat TEXT NOT NULL DEFAULT 'a_traiter' CHECK (etat IN (
        'a_traiter', 'propose', 'sollicite', 'remplace', 'autre_cours', 'deplace', 'continuite',
        'annule', 'supplee', 'sans_objet', 'non_assure')),
    reponse_active_id UUID,
    -- CAP-46 : la séance a disparu au recalcul (nouvelle version d'EDT) alors qu'une réponse y était posée :
    -- la réponse est conservée et SIGNALÉE, jamais perdue en silence.
    a_revoir BOOLEAN NOT NULL DEFAULT false,
    calcule_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_impact_type_chk CHECK (
        (type = 'seance' AND seance_id IS NOT NULL)
        OR (type = 'poste' AND poste_ref IS NOT NULL)
        OR (type = 'engagement' AND engagement_socle IS NOT NULL AND engagement_ref IS NOT NULL)),
    CONSTRAINT ent_absp_impact_dates_chk CHECK (fin > debut),
    CONSTRAINT ent_absp_impact_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_impact_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_absp_absence (etablissement_id, id)
);
-- Une séance ne porte qu'un impact par absence ; une même séance peut l'être par deux absences distinctes
-- (double engagement : deux absences liées, R-AP4).
CREATE UNIQUE INDEX IF NOT EXISTS ent_absp_impact_seance_uq
    ON ent_absp_impact (etablissement_id, absence_id, seance_id) WHERE seance_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_absp_impact_absence_idx ON ent_absp_impact (etablissement_id, absence_id, debut);
CREATE INDEX IF NOT EXISTS ent_absp_impact_jour_idx ON ent_absp_impact (etablissement_id, debut);
CREATE INDEX IF NOT EXISTS ent_absp_impact_etat_idx ON ent_absp_impact (etablissement_id, etat, debut);

CREATE TABLE IF NOT EXISTS ent_absp_reponse (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    impact_id UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN (
        'remplacement', 'autre_cours', 'deplacement', 'continuite', 'annulation', 'suppleance',
        'sans_objet', 'echange', 'avance')),
    remplacant_agent_id UUID,
    encadrant_agent_id UUID,
    -- Lien vers la modification de l'EDT (`ent_edt_exception.id` aujourd'hui, modification du 10 ensuite).
    modification_edt_ref UUID,
    etat TEXT NOT NULL DEFAULT 'active' CHECK (etat IN ('active', 'levee')),
    posee_par_user_id UUID NOT NULL,
    posee_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    levee_par_user_id UUID,
    levee_le TIMESTAMPTZ,
    CONSTRAINT ent_absp_reponse_levee_chk CHECK ((etat = 'levee') = (levee_le IS NOT NULL)),
    CONSTRAINT ent_absp_reponse_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_reponse_impact_fk FOREIGN KEY (etablissement_id, impact_id)
        REFERENCES ent_absp_impact (etablissement_id, id),
    CONSTRAINT ent_absp_reponse_remplacant_fk FOREIGN KEY (etablissement_id, remplacant_agent_id)
        REFERENCES ent_agent (etablissement_id, id)
);
-- Une réponse active par impact.
CREATE UNIQUE INDEX IF NOT EXISTS ent_absp_reponse_active_uq
    ON ent_absp_reponse (etablissement_id, impact_id) WHERE etat = 'active';

DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_absp_impact_reponse_fk') THEN
        ALTER TABLE ent_absp_impact ADD CONSTRAINT ent_absp_impact_reponse_fk
            FOREIGN KEY (etablissement_id, reponse_active_id) REFERENCES ent_absp_reponse (etablissement_id, id);
    END IF;
END $$;

SELECT ent_rls_poser(ARRAY['ent_absp_impact', 'ent_absp_reponse']);
