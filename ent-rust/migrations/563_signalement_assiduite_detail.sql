-- D-036 — `ent_signalement_assiduite.dossier` (JSONB : élève, mois, seuil, détail des
-- demi-journées non justifiées) devient une table de détail typée. Élève, mois, décompte et seuil
-- sont déjà des colonnes (ou une jointure sur `ent_eleve`) ; reste le détail jour par jour.
-- Le JSON du « dossier » renvoyé par l'API est reconstruit à la lecture (format d'échange).
--
-- Reprise idempotente : seuls les signalements SANS ligne de détail sont repris ; le code n'écrit
-- plus que `{}` dans l'ancienne colonne, conservée en lecture seule (déclencheur) puis retirée
-- dans un lot ultérieur (lot D-036 « retrait des colonnes reprises »).

CREATE TABLE IF NOT EXISTS ent_signalement_assiduite_detail (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    signalement_id UUID NOT NULL REFERENCES ent_signalement_assiduite (id) ON DELETE CASCADE,
    jour DATE NOT NULL,
    periode TEXT NOT NULL CHECK (periode IN ('matin', 'apres_midi')),
    minutes INTEGER NOT NULL CHECK (minutes >= 0),
    PRIMARY KEY (signalement_id, jour, periode)
);
CREATE INDEX IF NOT EXISTS ent_signalement_assiduite_detail_etab_idx
    ON ent_signalement_assiduite_detail (etablissement_id, signalement_id);

INSERT INTO ent_signalement_assiduite_detail (etablissement_id, signalement_id, jour, periode, minutes)
SELECT s.etablissement_id, s.id, x.jour, x.periode, x.minutes
  FROM ent_signalement_assiduite s
  CROSS JOIN LATERAL (
      SELECT CASE WHEN e ->> 'jour' ~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}$' THEN (e ->> 'jour')::date END AS jour,
             e ->> 'periode' AS periode,
             CASE WHEN e ->> 'minutes' ~ '^[0-9]{1,6}$' THEN (e ->> 'minutes')::integer END AS minutes
        FROM jsonb_array_elements(
                 CASE WHEN jsonb_typeof(s.dossier -> 'demi_journees') = 'array'
                      THEN s.dossier -> 'demi_journees' ELSE '[]'::jsonb END) AS e
  ) x
 WHERE x.jour IS NOT NULL AND x.periode IN ('matin', 'apres_midi') AND x.minutes IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM ent_signalement_assiduite_detail d WHERE d.signalement_id = s.id)
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION ent_signalement_dossier_lecture_seule() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.dossier IS DISTINCT FROM '{}'::jsonb
       AND (TG_OP = 'INSERT' OR NEW.dossier IS DISTINCT FROM OLD.dossier) THEN
        RAISE EXCEPTION 'ent_signalement_assiduite.dossier est en lecture seule (D-036) : utiliser ent_signalement_assiduite_detail'
            USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_signalement_dossier_lecture_seule ON ent_signalement_assiduite;
CREATE TRIGGER ent_signalement_dossier_lecture_seule
    BEFORE INSERT OR UPDATE OF dossier ON ent_signalement_assiduite
    FOR EACH ROW EXECUTE FUNCTION ent_signalement_dossier_lecture_seule();

ALTER TABLE ent_signalement_assiduite_detail ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_signalement_assiduite_detail FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_signalement_assiduite_detail;
CREATE POLICY ent_rls_etablissement ON ent_signalement_assiduite_detail FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));
