-- D-036 (lot 2) — `ent_etablissement_profil.jours_ouvres` (JSONB) devient une table typée : une
-- ligne par jour ouvré de la semaine type, avec ses deux demi-journées (matin, après-midi).
-- Le JSON `{lun:[matin,apres_midi], …}` échangé avec l'API est reconstruit à la lecture par
-- `ent_etablissement_jours_ouvres`.
--
-- Reprise idempotente : seuls les profils SANS ligne sont repris (valeurs inconnues ignorées ; à
-- défaut de ligne valide, la semaine par défaut de l'ancienne colonne). L'ancienne colonne reste
-- en LECTURE SEULE (déclencheur) jusqu'au lot de retrait.

CREATE TABLE IF NOT EXISTS ent_etablissement_jour_ouvre (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    jour TEXT NOT NULL CHECK (jour IN ('lun', 'mar', 'mer', 'jeu', 'ven', 'sam', 'dim')),
    matin BOOLEAN NOT NULL,
    apres_midi BOOLEAN NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_etablissement_jour_ouvre_demi_chk CHECK (matin OR apres_midi),
    CONSTRAINT ent_etablissement_jour_ouvre_uq UNIQUE (etablissement_id, jour),
    CONSTRAINT ent_etablissement_jour_ouvre_profil_fk FOREIGN KEY (etablissement_id)
        REFERENCES ent_etablissement_profil (etablissement_id) ON DELETE CASCADE
);

-- Reprise : jours connus, demi-journées connues (au moins une).
INSERT INTO ent_etablissement_jour_ouvre (id, etablissement_id, jour, matin, apres_midi)
SELECT gen_random_uuid(), p.etablissement_id, j.cle,
       coalesce(j.valeur ? 'matin', false), coalesce(j.valeur ? 'apres_midi', false)
  FROM ent_etablissement_profil p,
       LATERAL jsonb_each(CASE WHEN jsonb_typeof(p.jours_ouvres) = 'object' THEN p.jours_ouvres ELSE '{}'::jsonb END)
           AS j(cle, valeur)
 WHERE j.cle IN ('lun', 'mar', 'mer', 'jeu', 'ven', 'sam', 'dim')
   AND jsonb_typeof(j.valeur) = 'array'
   AND (j.valeur ? 'matin' OR j.valeur ? 'apres_midi')
   AND NOT EXISTS (SELECT 1 FROM ent_etablissement_jour_ouvre x WHERE x.etablissement_id = p.etablissement_id)
ON CONFLICT DO NOTHING;

-- Profils sans aucune ligne valide : semaine par défaut (celle de l'ancienne colonne).
INSERT INTO ent_etablissement_jour_ouvre (id, etablissement_id, jour, matin, apres_midi)
SELECT gen_random_uuid(), p.etablissement_id, d.jour, true, d.apres_midi
  FROM ent_etablissement_profil p,
       (VALUES ('lun', true), ('mar', true), ('mer', false), ('jeu', true), ('ven', true)) AS d(jour, apres_midi)
 WHERE NOT EXISTS (SELECT 1 FROM ent_etablissement_jour_ouvre x WHERE x.etablissement_id = p.etablissement_id)
ON CONFLICT DO NOTHING;

-- Semaine type au format d'échange de l'API ; semaine par défaut si l'établissement n'a rien.
CREATE OR REPLACE FUNCTION ent_etablissement_jours_ouvres(etab uuid) RETURNS jsonb
LANGUAGE sql STABLE AS $$
    SELECT coalesce(
        (SELECT jsonb_object_agg(j.jour, to_jsonb(array_remove(
                    ARRAY[CASE WHEN j.matin THEN 'matin' END, CASE WHEN j.apres_midi THEN 'apres_midi' END],
                    NULL)))
           FROM ent_etablissement_jour_ouvre j WHERE j.etablissement_id = etab),
        '{"lun":["matin","apres_midi"],"mar":["matin","apres_midi"],"mer":["matin"],"jeu":["matin","apres_midi"],"ven":["matin","apres_midi"]}'::jsonb)
$$;

-- Ancienne colonne : lecture seule (seule la valeur par défaut est acceptée).
CREATE OR REPLACE FUNCTION ent_profil_jours_ouvres_lecture_seule() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.jours_ouvres IS DISTINCT FROM
           '{"lun":["matin","apres_midi"],"mar":["matin","apres_midi"],"mer":["matin"],"jeu":["matin","apres_midi"],"ven":["matin","apres_midi"]}'::jsonb
       AND (TG_OP = 'INSERT' OR NEW.jours_ouvres IS DISTINCT FROM OLD.jours_ouvres) THEN
        RAISE EXCEPTION 'ent_etablissement_profil.jours_ouvres est en lecture seule (D-036) : utiliser ent_etablissement_jour_ouvre'
            USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_profil_jours_ouvres_lecture_seule ON ent_etablissement_profil;
CREATE TRIGGER ent_profil_jours_ouvres_lecture_seule
    BEFORE INSERT OR UPDATE OF jours_ouvres ON ent_etablissement_profil
    FOR EACH ROW EXECUTE FUNCTION ent_profil_jours_ouvres_lecture_seule();

SELECT ent_rls_poser(ARRAY['ent_etablissement_jour_ouvre']);
