-- Messagerie, lot M6 (`docs/socles/16_MESSAGERIE.md` §13, §23 ; R-M19, R-M22 ; CM-61, CM-62, Q-M8) : « lu jusqu'à »
-- par participant. Plage 480–499 (repli de la messagerie) : 488 à 493 sont réservées à M5 et M6. Rejouable.
--
-- Une ligne par participant (personne, élève **ou boîte**) et par fil : le dernier message lu **par ce participant**.
-- Pour une boîte, la lecture est celle du **premier membre** qui ouvre (§13) : la ligne est celle de la boîte,
-- `lu_par_personne_id` garde le membre (visible du seul personnel de la boîte, jamais de la famille : R-M22 et
-- §21.3 point 4). Elle ne recule jamais (déclencheur). Aucun accusé n'est posé pour une lecture qui n'est pas celle
-- d'un destinataire (supervision, auteur, membre d'une autre boîte : R-M19) : le service ne crée simplement pas la ligne.

CREATE TABLE IF NOT EXISTS ent_msg_lecture (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    fil_id UUID NOT NULL,
    participant_id UUID NOT NULL,
    -- Dernier message lu, et son instant de création (clé d'ordre : un message plus ancien ne fait pas reculer).
    lu_jusqua_message_id UUID NOT NULL,
    lu_jusqua_message_le TIMESTAMPTZ NOT NULL,
    premiere_lecture_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    lu_par_personne_id UUID,
    PRIMARY KEY (fil_id, participant_id),
    CONSTRAINT ent_msg_lecture_participant_fk FOREIGN KEY (etablissement_id, participant_id)
        REFERENCES ent_msg_participant (etablissement_id, id),
    CONSTRAINT ent_msg_lecture_fil_fk FOREIGN KEY (etablissement_id, fil_id)
        REFERENCES ent_msg_fil (etablissement_id, id),
    CONSTRAINT ent_msg_lecture_message_fk FOREIGN KEY (etablissement_id, lu_jusqua_message_id)
        REFERENCES ent_msg (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_msg_lecture_fil_idx ON ent_msg_lecture (etablissement_id, fil_id);

-- « Lu jusqu'à » ne recule jamais : une mise à jour vers un message plus ancien est ignorée (la ligne reste).
CREATE OR REPLACE FUNCTION ent_msg_lecture_monotone() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.fil_id IS DISTINCT FROM OLD.fil_id
       OR NEW.participant_id IS DISTINCT FROM OLD.participant_id
       OR NEW.premiere_lecture_le IS DISTINCT FROM OLD.premiere_lecture_le THEN
        RAISE EXCEPTION 'ent_msg_lecture : identité et première lecture immuables' USING ERRCODE = 'check_violation';
    END IF;
    IF NEW.lu_jusqua_message_le < OLD.lu_jusqua_message_le THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_msg_lecture_monotone_trg ON ent_msg_lecture;
CREATE TRIGGER ent_msg_lecture_monotone_trg BEFORE UPDATE ON ent_msg_lecture
    FOR EACH ROW EXECUTE FUNCTION ent_msg_lecture_monotone();

-- Les participations **actives** d'un compte dans un fil : sa propre ligne (personne, élève) et celle des boîtes dont
-- il est membre à la date. Même résolution que `ent_msg_fil_lisible` (485) : la lecture reste décidée par cette
-- dernière (R-M2) ; cette fonction ne dit que « quelle ligne de participant est la mienne » pour poser l'avis.
CREATE OR REPLACE FUNCTION ent_msg_mes_participations(p_etab UUID, p_user UUID, p_fil UUID)
RETURNS TABLE (participant_id UUID, cote TEXT, boite_id UUID, personne_id UUID, eleve_id UUID)
LANGUAGE sql STABLE AS $$
    WITH moi AS (
        SELECT pe.id FROM ent_personne pe
         WHERE pe.etablissement_id = p_etab AND pe.user_id = p_user AND pe.fusionnee_dans IS NULL
        UNION
        SELECT s.id FROM ent_personne s
          JOIN ent_personne pe ON pe.id = s.fusionnee_dans AND pe.etablissement_id = s.etablissement_id
         WHERE pe.etablissement_id = p_etab AND pe.user_id = p_user
    ), mon_eleve AS (
        SELECT c.eleve_id FROM ent_eleve_compte c
         WHERE c.etablissement_id = p_etab AND c.user_id = p_user
    ), mes_boites AS (
        SELECT b.id FROM ent_msg_boite b
         WHERE b.etablissement_id = p_etab AND b.actif
           AND EXISTS (SELECT 1 FROM ent_msg_boite_membres(p_etab, b.id) bm WHERE bm.user_id = p_user)
    )
    SELECT p.id, p.cote, p.boite_id, p.personne_id, p.eleve_id
      FROM ent_msg_participant p
     WHERE p.etablissement_id = p_etab AND p.fil_id = p_fil
       AND p.au IS NULL AND p.suspendu_le IS NULL
       AND (p.personne_id IN (SELECT id FROM moi)
            OR p.eleve_id IN (SELECT eleve_id FROM mon_eleve)
            OR p.boite_id IN (SELECT id FROM mes_boites))
$$;

-- RLS permissive (même politique que 480, 482 et 485).
DO $$
BEGIN
    ALTER TABLE ent_msg_lecture ENABLE ROW LEVEL SECURITY;
    ALTER TABLE ent_msg_lecture FORCE ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS ent_rls_etablissement ON ent_msg_lecture;
    CREATE POLICY ent_rls_etablissement ON ent_msg_lecture FOR ALL
        USING (etablissement_id = (SELECT ent_rls_etablissement())
            OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
        WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
            OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));
END
$$;
