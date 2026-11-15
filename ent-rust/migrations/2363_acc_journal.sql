-- Socle 18, lot FO1 — migration 2363 : journal des accords (ajout seul)
-- (`docs/socles/18_FORMULAIRES.md` §9.2, §23.2 ; règles R-FC38, R-FC39).
--
-- Une ligne par acte : activation d'un objet, publication d'une version, **lecture détaillée** des décisions par le
-- secrétariat ou la direction (FD-09), reprise, et — **agrégée** — les vérifications de routine du port `verifier`
-- (« qui a vérifié quel objet pour combien de sujets », jamais une ligne par élève, minimisation).
-- Additif, rejouable, sans JSON, sans date UTC.

CREATE TABLE IF NOT EXISTS ent_acc_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    acte TEXT NOT NULL CHECK (acte IN (
        'objet_active', 'objet_cree', 'objet_desactive', 'version_redigee', 'version_publiee', 'version_relue',
        'decision', 'retrait', 'lecture_detaillee', 'verification', 'reprise')),
    objet_id UUID,
    version_id UUID,
    sujet_eleve_id UUID,
    sujet_personne_id UUID,
    acteur_user_id UUID,
    -- Nombre de sujets d'une vérification ou d'une lecture, de lignes d'une reprise.
    nb INTEGER CHECK (nb IS NULL OR nb >= 0),
    -- Code court (usage vérifié, portée de la lecture), jamais un texte libre ni une réponse.
    detail TEXT CHECK (detail IS NULL OR detail ~ '^[a-z0-9_.:-]{1,80}$'),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_acc_journal_objet_cfk FOREIGN KEY (etablissement_id, objet_id)
        REFERENCES ent_acc_objet (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_acc_journal_etab_idx ON ent_acc_journal (etablissement_id, le DESC);
CREATE INDEX IF NOT EXISTS ent_acc_journal_objet_idx ON ent_acc_journal (etablissement_id, objet_id, le DESC);

CREATE OR REPLACE FUNCTION ent_acc_journal_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF current_setting('ent.purge_etablissement', true) = 'on' THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'ent_acc_journal : journal en ajout seul' USING ERRCODE = '42501';
END $$;
DROP TRIGGER IF EXISTS ent_acc_journal_garde ON ent_acc_journal;
CREATE TRIGGER ent_acc_journal_garde BEFORE UPDATE OR DELETE ON ent_acc_journal
    FOR EACH ROW EXECUTE FUNCTION ent_acc_journal_garde();

SELECT ent_rls_poser(ARRAY['ent_acc_journal']);
