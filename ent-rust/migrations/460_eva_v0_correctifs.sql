-- 460 — Socle 21 (évaluations), lot V0 « correctifs urgents » (plage de repli 460–479 de
-- docs/socles/MIGRATIONS.md : la règle Q-B2 n'est pas adoptée). Migration idempotente.
--   D8  : dédoublonnage des résultats (devoir, élève) + contrainte d'unicité ;
--   D10 : `absent_compte_zero` neutralisé (une absence n'est jamais un zéro) ;
--   R-V5 : statut de résultat « à rattraper » (distinct, jamais converti en note) ;
--   D2  : journal en ajout seul des changements de statut d'une période ;
--   D11 : mentions « avertissement_* » du conseil reclassées en « mise_en_garde_* » ;
--   D9  : RLS permissive (migration 100) sur les tables d'évaluation, de conseil, de bulletin
--         et de compétences (la RLS stricte est au lot V13).

-- ── D8 : doublons de résultats ─────────────────────────────────────────────────
-- Les lignes en trop sont archivées (rien ne s'efface) puis retirées. On garde la ligne qui
-- porte une correction journalisée, sinon la plus récente.
CREATE TABLE IF NOT EXISTS ent_note_doublon_archive (
    LIKE ent_note INCLUDING DEFAULTS,
    note_conservee_id UUID NOT NULL,
    archive_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (id)
);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'ent_note_doublon_archive'::regclass AND contype = 'p') THEN
        ALTER TABLE ent_note_doublon_archive ADD PRIMARY KEY (id);
    END IF;
END
$$;

WITH classees AS (
    SELECT n.id,
           first_value(n.id) OVER w AS garde
      FROM ent_note n
      LEFT JOIN (SELECT DISTINCT note_id FROM ent_note_correction) c ON c.note_id = n.id
     WHERE n.devoir_id IS NOT NULL
    WINDOW w AS (PARTITION BY n.etablissement_id, n.devoir_id, n.eleve_id
                 ORDER BY (c.note_id IS NOT NULL) DESC, n.created_at DESC, n.id DESC)
), en_trop AS (
    SELECT id, garde FROM classees WHERE id <> garde
), archivees AS (
    INSERT INTO ent_note_doublon_archive
    SELECT n.*, e.garde, now()
      FROM ent_note n JOIN en_trop e ON e.id = n.id
    RETURNING id
)
DELETE FROM ent_note WHERE id IN (SELECT id FROM archivees);

CREATE UNIQUE INDEX IF NOT EXISTS ent_note_devoir_eleve_uq
    ON ent_note (etablissement_id, devoir_id, eleve_id) WHERE devoir_id IS NOT NULL;

-- ── R-V5 : statut « à rattraper » ──────────────────────────────────────────────
ALTER TABLE ent_note DROP CONSTRAINT IF EXISTS ent_note_statut_chk;
ALTER TABLE ent_note ADD CONSTRAINT ent_note_statut_chk
    CHECK (statut IN ('note', 'absent', 'a_rattraper', 'dispense', 'non_note'));

-- ── D10 : une absence n'est jamais un zéro ─────────────────────────────────────
UPDATE ent_eva_regle SET absent_compte_zero = false WHERE absent_compte_zero;
ALTER TABLE ent_eva_regle DROP CONSTRAINT IF EXISTS ent_eva_regle_absent_zero_chk;
ALTER TABLE ent_eva_regle ADD CONSTRAINT ent_eva_regle_absent_zero_chk CHECK (absent_compte_zero = false);

-- ── D2 : journal des changements de statut d'une période (ajout seul) ──────────
CREATE TABLE IF NOT EXISTS ent_note_periode_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    periode_id UUID NOT NULL REFERENCES ent_note_periode (id),
    statut_avant TEXT NOT NULL,
    statut_apres TEXT NOT NULL,
    motif TEXT,
    acteur_user_id UUID NOT NULL REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_note_periode_journal_motif_chk CHECK (motif IS NULL OR length(btrim(motif)) >= 3)
);
CREATE INDEX IF NOT EXISTS ent_note_periode_journal_idx
    ON ent_note_periode_journal (etablissement_id, periode_id, cree_le DESC);

CREATE OR REPLACE FUNCTION ent_trg_periode_journal_ajout_seul() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'journal de période : ajout seul (modification et suppression interdites)'
        USING ERRCODE = '23514';
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_periode_journal_ajout_seul ON ent_note_periode_journal;
CREATE TRIGGER ent_periode_journal_ajout_seul BEFORE UPDATE OR DELETE ON ent_note_periode_journal
    FOR EACH ROW EXECUTE FUNCTION ent_trg_periode_journal_ajout_seul();

-- ── D11 : l'avertissement est une sanction disciplinaire, pas une mention du conseil ──
ALTER TABLE ent_conseil_classe_dossier DROP CONSTRAINT IF EXISTS ent_conseil_dossier_mention_chk;
UPDATE ent_conseil_classe_dossier SET mention = 'mise_en_garde_travail' WHERE mention = 'avertissement_travail';
UPDATE ent_conseil_classe_dossier SET mention = 'mise_en_garde_conduite' WHERE mention = 'avertissement_conduite';
ALTER TABLE ent_conseil_classe_dossier ADD CONSTRAINT ent_conseil_dossier_mention_chk
    CHECK (mention IS NULL OR mention IN (
        'felicitations', 'complement', 'encouragements',
        'mise_en_garde_travail', 'mise_en_garde_conduite', 'aucune'));

-- ── D9 : RLS permissive (même politique que les migrations 100 et 206) ──────────
DO $$
DECLARE
    t text;
    tables text[] := ARRAY[
        'ent_note_periode', 'ent_devoir', 'ent_note', 'ent_note_appreciation', 'ent_note_correction',
        'ent_eva_regle', 'ent_eva_coef_matiere', 'ent_eva_export',
        'ent_conseil_classe', 'ent_conseil_classe_dossier', 'ent_orientation_avis',
        'ent_bilan_cycle', 'ent_bilan_cycle_composante', 'ent_bulletin',
        'ent_competence_item', 'ent_competence_evaluation',
        'ent_note_periode_journal', 'ent_note_doublon_archive'
    ];
BEGIN
    FOREACH t IN ARRAY tables LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('DROP POLICY IF EXISTS ent_rls_etablissement ON %I', t);
        EXECUTE format(
            'CREATE POLICY ent_rls_etablissement ON %I FOR ALL '
            'USING (etablissement_id = (SELECT ent_rls_etablissement()) '
            '  OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict()))) '
            'WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement()) '
            '  OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))',
            t);
    END LOOP;
END
$$;
