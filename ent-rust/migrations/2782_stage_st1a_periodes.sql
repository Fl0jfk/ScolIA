-- 2782 — Socle 39 (stages), lot ST1 (a) : périodes de stage et leurs publics (STG-120 à STG-124).
-- Réf. : docs/socles/39_STAGES.md §9.2, §19.3, §25. Additive, rejouable, sans JSON (D-036).
-- Une période appartient à une année scolaire du socle 5 et à un dispositif ; elle vise une classe,
-- un groupe ou un élève (trois colonnes à clé étrangère, jamais un identifiant sans contrainte).
-- La période de nature « entreprise » du calendrier du socle 5 sera liée au lot ST1 (b).

-- `ent_groupe` n'avait pas de clé (id, établissement) : nécessaire aux clés composées (E2).
CREATE UNIQUE INDEX IF NOT EXISTS ent_groupe_id_etab_st_uq ON ent_groupe (id, etablissement_id);

CREATE TABLE IF NOT EXISTS ent_stage_periode (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    dispositif_code TEXT NOT NULL REFERENCES ref_stage_dispositif (code),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    du DATE NOT NULL,
    au DATE NOT NULL,
    type TEXT NOT NULL DEFAULT 'collective' CHECK (type IN ('collective', 'individuelle')),
    -- Motif obligatoire pour une période individuelle (rattrapage, décalage) : STG-123.
    motif TEXT CHECK (motif IS NULL OR length(motif) <= 300),
    etat TEXT NOT NULL DEFAULT 'prevue' CHECK (etat IN ('prevue', 'publiee', 'close')),
    publiee_le TIMESTAMPTZ,
    close_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_personne_id UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    CONSTRAINT ent_stage_periode_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_stage_periode_dates_chk CHECK (au >= du),
    CONSTRAINT ent_stage_periode_motif_chk CHECK (type = 'collective' OR length(btrim(coalesce(motif, ''))) > 0),
    CONSTRAINT ent_stage_periode_etat_chk CHECK (
        (etat = 'prevue' AND publiee_le IS NULL AND close_le IS NULL)
        OR (etat = 'publiee' AND publiee_le IS NOT NULL AND close_le IS NULL)
        OR (etat = 'close' AND close_le IS NOT NULL)),
    CONSTRAINT ent_stage_periode_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id) ON DELETE RESTRICT,
    CONSTRAINT ent_stage_periode_cree_par_fk FOREIGN KEY (etablissement_id, cree_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_stage_periode_annee_idx
    ON ent_stage_periode (etablissement_id, annee_id, du);

CREATE TABLE IF NOT EXISTS ent_stage_periode_public (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    periode_id UUID NOT NULL,
    classe_id UUID,
    groupe_id UUID,
    eleve_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Exactement une cible.
    CONSTRAINT ent_stage_periode_public_cible_chk CHECK (
        (classe_id IS NOT NULL)::int + (groupe_id IS NOT NULL)::int + (eleve_id IS NOT NULL)::int = 1),
    CONSTRAINT ent_stage_periode_public_periode_fk FOREIGN KEY (etablissement_id, periode_id)
        REFERENCES ent_stage_periode (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_stage_periode_public_classe_fk FOREIGN KEY (classe_id, etablissement_id)
        REFERENCES ent_classe (id, etablissement_id) ON DELETE RESTRICT,
    CONSTRAINT ent_stage_periode_public_groupe_fk FOREIGN KEY (groupe_id, etablissement_id)
        REFERENCES ent_groupe (id, etablissement_id) ON DELETE RESTRICT,
    CONSTRAINT ent_stage_periode_public_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_stage_periode_public_classe_uq
    ON ent_stage_periode_public (periode_id, classe_id) WHERE classe_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_stage_periode_public_groupe_uq
    ON ent_stage_periode_public (periode_id, groupe_id) WHERE groupe_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_stage_periode_public_eleve_uq
    ON ent_stage_periode_public (periode_id, eleve_id) WHERE eleve_id IS NOT NULL;

SELECT ent_rls_poser(ARRAY['ent_stage_periode', 'ent_stage_periode_public']);
