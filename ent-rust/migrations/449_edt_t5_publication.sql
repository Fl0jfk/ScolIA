-- 449 — Socle 10 (emploi du temps), lot T5 : publication des versions (docs/socles/10_EMPLOI_DU_TEMPS.md §14.1,
-- §21.2, EDT-150 à EDT-156).
--
-- Une version passe de `brouillon` à `publiee` en DEUX TEMPS (défaut) :
--   1. « annoncée » aux personnels (`annoncee_le`) : la version est figée (ses placements ne se modifient plus), sa date
--      de prise d'effet (`valable_du`) et sa date de diffusion (`diffusion_familles_le`) sont posées ; l'emploi du temps
--      RÉEL (les séances) n'a pas encore changé : les familles et les élèves ne voient rien ;
--   2. diffusée : une seule transaction bascule le réel (version précédente tronquée à la veille, nouvelle version
--      `publiee`, séances matérialisées, événement) ; `publiee_le` est l'instant de cette bascule.
-- La publication immédiate fait les deux dans la même transaction. Le retour arrière restaure la version remplacée
-- (`remplace_*`) ou republie une COPIE de la version précédente à une date future (le passé ne change pas).
-- * `ent_edt_version_journal` : historique en ajout seul des faits de publication (qui, quand, effet, chiffres) ;
-- * `ent_edt_individuelle` et ses lignes : version INDIVIDUELLE d'un élève (inclusions/exclusions datées du socle 5,
--   `ent_cours_membre`) sans republier l'établissement.
-- Additive, rejouable (E8). E1, E2, E3, E6 ; aucun JSON ni tableau ; RLS en 455.

ALTER TABLE ent_edt_version ADD COLUMN IF NOT EXISTS annoncee_le TIMESTAMPTZ;
ALTER TABLE ent_edt_version ADD COLUMN IF NOT EXISTS annoncee_par UUID;
ALTER TABLE ent_edt_version ADD COLUMN IF NOT EXISTS empreinte_controles TEXT;
ALTER TABLE ent_edt_version ADD COLUMN IF NOT EXISTS remplace_version_id UUID;
ALTER TABLE ent_edt_version ADD COLUMN IF NOT EXISTS remplace_valable_au_avant DATE;
ALTER TABLE ent_edt_version ADD COLUMN IF NOT EXISTS remplace_etat_avant TEXT;
ALTER TABLE ent_edt_version ADD COLUMN IF NOT EXISTS copie_de_version_id UUID;
ALTER TABLE ent_edt_version ADD COLUMN IF NOT EXISTS retiree_le TIMESTAMPTZ;
ALTER TABLE ent_edt_version ADD COLUMN IF NOT EXISTS retiree_par UUID;

-- Les versions déjà publiées (reprise) ont été « annoncées » quand elles ont été publiées.
UPDATE ent_edt_version SET annoncee_le = publiee_le WHERE etat IN ('publiee', 'remplacee') AND annoncee_le IS NULL;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_version_remplace_fk') THEN
        ALTER TABLE ent_edt_version ADD CONSTRAINT ent_edt_version_remplace_fk
            FOREIGN KEY (remplace_version_id, etablissement_id) REFERENCES ent_edt_version (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_version_copie_fk') THEN
        ALTER TABLE ent_edt_version ADD CONSTRAINT ent_edt_version_copie_fk
            FOREIGN KEY (copie_de_version_id, etablissement_id) REFERENCES ent_edt_version (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_version_remplace_etat_chk') THEN
        ALTER TABLE ent_edt_version ADD CONSTRAINT ent_edt_version_remplace_etat_chk
            CHECK (remplace_etat_avant IS NULL OR remplace_etat_avant IN ('publiee', 'remplacee'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_version_annonce_chk') THEN
        ALTER TABLE ent_edt_version ADD CONSTRAINT ent_edt_version_annonce_chk
            CHECK ((annoncee_le IS NULL) = (annoncee_par IS NULL) OR etat IN ('publiee', 'remplacee'));
    END IF;
END
$$;

CREATE INDEX IF NOT EXISTS ent_edt_version_annoncee_idx
    ON ent_edt_version (etablissement_id, diffusion_familles_le) WHERE etat = 'brouillon' AND annoncee_le IS NOT NULL;

-- Historique des faits de publication, en ajout seul.
CREATE TABLE IF NOT EXISTS ent_edt_version_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    version_id UUID NOT NULL,
    fait TEXT NOT NULL CHECK (fait IN
        ('annoncee', 'diffusee', 'annonce_retiree', 'diffusion_reglee', 'retour_arriere', 'copie_retour')),
    date_effet DATE,
    diffusion_le TIMESTAMPTZ,
    nb_creees INTEGER NOT NULL DEFAULT 0 CHECK (nb_creees >= 0),
    nb_retirees INTEGER NOT NULL DEFAULT 0 CHECK (nb_retirees >= 0),
    nb_conservees INTEGER NOT NULL DEFAULT 0 CHECK (nb_conservees >= 0),
    nb_modifiees INTEGER NOT NULL DEFAULT 0 CHECK (nb_modifiees >= 0),
    nb_modifications_conservees INTEGER NOT NULL DEFAULT 0 CHECK (nb_modifications_conservees >= 0),
    nb_avertissements INTEGER NOT NULL DEFAULT 0 CHECK (nb_avertissements >= 0),
    -- Prise d'effet le jour même (forçage de la direction) : motif obligatoire.
    forcage_motif TEXT CHECK (forcage_motif IS NULL OR char_length(btrim(forcage_motif)) BETWEEN 1 AND 300),
    decalee BOOLEAN NOT NULL DEFAULT false,
    par UUID NOT NULL,
    le TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_edt_version_journal_version_idx ON ent_edt_version_journal (etablissement_id, version_id, le);
CREATE INDEX IF NOT EXISTS ent_edt_version_journal_annee_idx ON ent_edt_version_journal (etablissement_id, annee_id, le DESC);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_version_journal_version_fk') THEN
        ALTER TABLE ent_edt_version_journal ADD CONSTRAINT ent_edt_version_journal_version_fk
            FOREIGN KEY (version_id, etablissement_id) REFERENCES ent_edt_version (id, etablissement_id);
    END IF;
END
$$;

CREATE OR REPLACE FUNCTION ent_edt_version_journal_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'ent_edt_version_journal est en ajout seul (% interdit)', TG_OP USING ERRCODE = '23514';
END
$$;

DROP TRIGGER IF EXISTS ent_edt_version_journal_ajout_seul_tg ON ent_edt_version_journal;
CREATE TRIGGER ent_edt_version_journal_ajout_seul_tg BEFORE UPDATE OR DELETE ON ent_edt_version_journal
    FOR EACH ROW EXECUTE FUNCTION ent_edt_version_journal_ajout_seul();

-- Version individuelle d'un élève (EDT-156) : le fait de publier ; l'effet est porté par `ent_cours_membre`
-- (inclusions et exclusions datées du socle 5), qui reste LA source du public d'un cours.
CREATE TABLE IF NOT EXISTS ent_edt_individuelle (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    motif_code TEXT NOT NULL CHECK (motif_code IN
        ('ulis', 'upe2a', 'decloisonnement', 'avance', 'parcours_amenage', 'scolarite_partagee',
         'echange', 'accueil_temporaire', 'dispense_cours', 'autre')),
    a_partir_du DATE NOT NULL,
    au DATE,
    cle TEXT CHECK (cle IS NULL OR char_length(cle) BETWEEN 1 AND 80),
    cree_par UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    annulee_le TIMESTAMPTZ,
    annulee_par UUID,
    CONSTRAINT ent_edt_individuelle_dates_chk CHECK (au IS NULL OR au >= a_partir_du),
    CONSTRAINT ent_edt_individuelle_annulee_chk CHECK ((annulee_le IS NULL) = (annulee_par IS NULL))
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_individuelle_id_etab_uq ON ent_edt_individuelle (id, etablissement_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_individuelle_cle_uq ON ent_edt_individuelle (etablissement_id, cle) WHERE cle IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_edt_individuelle_eleve_idx ON ent_edt_individuelle (etablissement_id, eleve_id, a_partir_du);

CREATE TABLE IF NOT EXISTS ent_edt_individuelle_ligne (
    individuelle_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    cours_id UUID NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN ('inclusion', 'exclusion')),
    membre_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (individuelle_id, cours_id, nature)
);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_individuelle_eleve_fk') THEN
        ALTER TABLE ent_edt_individuelle ADD CONSTRAINT ent_edt_individuelle_eleve_fk
            FOREIGN KEY (eleve_id, etablissement_id) REFERENCES ent_eleve (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_individuelle_ligne_individuelle_fk') THEN
        ALTER TABLE ent_edt_individuelle_ligne ADD CONSTRAINT ent_edt_individuelle_ligne_individuelle_fk
            FOREIGN KEY (individuelle_id, etablissement_id) REFERENCES ent_edt_individuelle (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_individuelle_ligne_cours_fk') THEN
        ALTER TABLE ent_edt_individuelle_ligne ADD CONSTRAINT ent_edt_individuelle_ligne_cours_fk
            FOREIGN KEY (cours_id, etablissement_id) REFERENCES ent_cours (id, etablissement_id);
    END IF;
END
$$;
