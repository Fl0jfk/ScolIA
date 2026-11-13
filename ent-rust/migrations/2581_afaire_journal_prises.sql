-- 2581 — Socle 29, lot P2 : journal des transitions (ajout seul), prises en charge, états
-- personnels. Réf. : 29_PORTAILS.md §22.1, §29.4. Migration additive et rejouable.
--
-- * `ent_afaire_journal` : AJOUT SEUL (déclencheur). Aucune valeur libre : états et motif en codes
--   fermés ; aucun contenu métier (R-PO5) ; conservation 12 mois (§15.5, à valider par le DPO).
-- * `ent_afaire_prise` : une prise en charge active par tâche (la première gagne, CPO-36).
-- * `ent_afaire_etat_personnel` : état propre à UN compte, jamais lu pour autrui (R-PO7). La
--   « lecture » n'est pas une preuve : simple booléen « nouveau », effacé à la clôture (§29.4).
--   La lecture est restreinte à `compte_id` par le code (toutes les requêtes filtrent sur le compte
--   de la session) ; la RLS ne connaît que l'établissement (la restriction par compte en base est
--   prévue au lot P15, RLS stricte, 2598).

CREATE TABLE IF NOT EXISTS ent_afaire_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    afaire_id UUID NOT NULL,
    de_etat TEXT CHECK (de_etat IS NULL OR de_etat IN (
        'ouverte', 'prise', 'en_attente', 'faite', 'sans_objet', 'expiree', 'annulee')),
    vers_etat TEXT NOT NULL CHECK (vers_etat IN (
        'ouverte', 'prise', 'en_attente', 'faite', 'sans_objet', 'expiree', 'annulee')),
    par_compte_id UUID,
    par_socle SMALLINT NOT NULL CHECK (par_socle BETWEEN 0 AND 99),
    motif_code TEXT CHECK (motif_code IS NULL OR length(motif_code) <= 40),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_afaire_journal_afaire_fk FOREIGN KEY (etablissement_id, afaire_id)
        REFERENCES ent_afaire (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_afaire_journal_afaire_idx
    ON ent_afaire_journal (etablissement_id, afaire_id, le);

CREATE OR REPLACE FUNCTION ent_afaire_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'table en ajout seul : % interdit sur %', TG_OP, TG_TABLE_NAME
        USING ERRCODE = '23000';
END
$$;
DROP TRIGGER IF EXISTS ent_afaire_journal_immuable ON ent_afaire_journal;
CREATE TRIGGER ent_afaire_journal_immuable
    BEFORE UPDATE OR DELETE ON ent_afaire_journal
    FOR EACH ROW EXECUTE FUNCTION ent_afaire_ajout_seul();

CREATE TABLE IF NOT EXISTS ent_afaire_prise (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    afaire_id UUID NOT NULL,
    compte_id UUID NOT NULL REFERENCES ent_user (id),
    -- Files à deux niveaux (§29.4) : préparer / décider.
    niveau TEXT NOT NULL DEFAULT 'decider' CHECK (niveau IN ('preparer', 'decider')),
    depuis TIMESTAMPTZ NOT NULL DEFAULT now(),
    jusqu_a TIMESTAMPTZ NOT NULL,
    relachee_le TIMESTAMPTZ,
    relachee_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_afaire_prise_afaire_fk FOREIGN KEY (etablissement_id, afaire_id)
        REFERENCES ent_afaire (etablissement_id, id),
    CONSTRAINT ent_afaire_prise_relache_chk CHECK ((relachee_le IS NULL) = (relachee_par IS NULL)),
    CONSTRAINT ent_afaire_prise_duree_chk CHECK (jusqu_a > depuis)
);
-- Une seule prise active par tâche ET par niveau : la première gagne (CPO-36).
CREATE UNIQUE INDEX IF NOT EXISTS ent_afaire_prise_active_uq
    ON ent_afaire_prise (afaire_id, niveau) WHERE relachee_le IS NULL;

CREATE TABLE IF NOT EXISTS ent_afaire_etat_personnel (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    afaire_id UUID NOT NULL,
    compte_id UUID NOT NULL REFERENCES ent_user (id),
    nouveau BOOLEAN NOT NULL DEFAULT true,
    reportee_jusqu_au TIMESTAMPTZ,
    masquee_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, afaire_id, compte_id),
    CONSTRAINT ent_afaire_etat_personnel_afaire_fk FOREIGN KEY (etablissement_id, afaire_id)
        REFERENCES ent_afaire (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_afaire_etat_personnel_compte_idx
    ON ent_afaire_etat_personnel (etablissement_id, compte_id);

SELECT ent_rls_poser(ARRAY['ent_afaire_journal', 'ent_afaire_prise', 'ent_afaire_etat_personnel']);
