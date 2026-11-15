-- 447 — Socle 10 (emploi du temps), lot T4 : journal des lectures « où est cet élève maintenant »
-- (docs/socles/10_EMPLOI_DU_TEMPS.md §14.3, §21.3, R-E27 ; `edt.localiser`).
--
-- Une lecture de localisation est un accès à une donnée sensible : elle est **journalisée** (qui, quel élève,
-- pour quel usage, quelle réponse), en ajout seul. La réponse n'est jamais le lieu : seulement sa nature
-- (en cours, pas de cours, indisponible), pour qu'un journal relu ne devienne pas lui-même une trace de
-- localisation. RLS d'établissement en 454. Additive, rejouable (E8). Aucune colonne JSON ni tableau.

CREATE TABLE IF NOT EXISTS ent_edt_localisation_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    lecteur_user_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    usage TEXT NOT NULL CHECK (usage IN ('remise', 'urgence', 'recherche', 'infirmerie')),
    reponse TEXT NOT NULL CHECK (reponse IN ('en_cours', 'pas_de_cours', 'indisponible')),
    le TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_edt_localisation_journal_eleve_idx
    ON ent_edt_localisation_journal (etablissement_id, eleve_id, le DESC);
CREATE INDEX IF NOT EXISTS ent_edt_localisation_journal_lecteur_idx
    ON ent_edt_localisation_journal (etablissement_id, lecteur_user_id, le DESC);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_localisation_journal_eleve_fk') THEN
        ALTER TABLE ent_edt_localisation_journal ADD CONSTRAINT ent_edt_localisation_journal_eleve_fk
            FOREIGN KEY (eleve_id) REFERENCES ent_eleve (id) ON DELETE RESTRICT;
    END IF;
END
$$;

CREATE OR REPLACE FUNCTION ent_edt_localisation_journal_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'ent_edt_localisation_journal est en ajout seul (% interdit)', TG_OP USING ERRCODE = '23514';
END
$$;

DROP TRIGGER IF EXISTS ent_edt_localisation_journal_ajout_seul_tg ON ent_edt_localisation_journal;
CREATE TRIGGER ent_edt_localisation_journal_ajout_seul_tg BEFORE UPDATE OR DELETE ON ent_edt_localisation_journal
    FOR EACH ROW EXECUTE FUNCTION ent_edt_localisation_journal_ajout_seul();
