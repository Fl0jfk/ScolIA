-- Socle 2, lot S2-7 : grille de saisie des notes (EVA-02), publication (EVA-03),
-- règles de calcul par établissement, coefficients de matière, corrections après
-- fermeture (EVA-14). Migration ADDITIVE : aucune donnée existante modifiée.

-- Résultat non numérique d'une évaluation : absent, dispensé, non noté.
-- `valeur` reste NOT NULL (lecteurs existants) : elle vaut 0 et n'est jamais lue
-- quand `statut` <> 'note' (les lecteurs historiques filtrent `statut = 'note'`).
ALTER TABLE ent_note ADD COLUMN IF NOT EXISTS statut TEXT NOT NULL DEFAULT 'note';
ALTER TABLE ent_note ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ;
ALTER TABLE ent_note ADD COLUMN IF NOT EXISTS updated_by_user_id UUID REFERENCES ent_user (id);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_note_statut_chk') THEN
        ALTER TABLE ent_note ADD CONSTRAINT ent_note_statut_chk
            CHECK (statut IN ('note', 'absent', 'dispense', 'non_note'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_note_valeur_statut_chk') THEN
        ALTER TABLE ent_note ADD CONSTRAINT ent_note_valeur_statut_chk
            CHECK (statut = 'note' OR valeur = 0);
    END IF;
END
$$;

-- Évaluation rattachée à un service d'enseignement (prof × matière × classe/groupe).
-- `publication_explicite` : les notes ne sont visibles des familles qu'après
-- publication (`publie_le`). Les devoirs historiques gardent `false` (inchangés).
ALTER TABLE ent_devoir ADD COLUMN IF NOT EXISTS service_id UUID REFERENCES ent_service_enseignement (id);
ALTER TABLE ent_devoir ADD COLUMN IF NOT EXISTS groupe_id UUID REFERENCES ent_groupe (id);
ALTER TABLE ent_devoir ADD COLUMN IF NOT EXISTS publication_explicite BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_devoir ADD COLUMN IF NOT EXISTS publie_le TIMESTAMPTZ;
ALTER TABLE ent_devoir ADD COLUMN IF NOT EXISTS publie_par_user_id UUID REFERENCES ent_user (id);
CREATE INDEX IF NOT EXISTS ent_devoir_service_idx
    ON ent_devoir (etablissement_id, service_id, periode_id) WHERE service_id IS NOT NULL;

-- Règles de calcul de l'établissement (une ligne au plus ; défauts sinon).
CREATE TABLE IF NOT EXISTS ent_eva_regle (
    etablissement_id UUID PRIMARY KEY,
    -- Arrondi d'AFFICHAGE des moyennes (le calcul est exact, sans arrondi intermédiaire).
    arrondi TEXT NOT NULL DEFAULT 'centieme',
    -- Un « absent » compte-t-il 0 ? Défaut : non (exclu comme dispensé et non noté).
    absent_compte_zero BOOLEAN NOT NULL DEFAULT false,
    updated_by_user_id UUID REFERENCES ent_user (id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eva_regle_arrondi_chk CHECK (arrondi IN ('centieme', 'dixieme', 'demi_point'))
);

-- Coefficient d'une matière dans la moyenne générale d'une classe (défaut 1).
CREATE TABLE IF NOT EXISTS ent_eva_coef_matiere (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    classe_id UUID NOT NULL REFERENCES ent_classe (id),
    matiere_id UUID NOT NULL REFERENCES ent_matiere (id),
    coef DOUBLE PRECISION NOT NULL,
    updated_by_user_id UUID REFERENCES ent_user (id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eva_coef_matiere_uq UNIQUE (etablissement_id, classe_id, matiere_id),
    CONSTRAINT ent_eva_coef_matiere_coef_chk CHECK (coef > 0 AND coef <= 100)
);

-- Journal des corrections d'une note après fermeture de la période (EVA-14).
CREATE TABLE IF NOT EXISTS ent_note_correction (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    note_id UUID NOT NULL REFERENCES ent_note (id),
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    periode_id UUID NOT NULL REFERENCES ent_note_periode (id),
    ancien_statut TEXT NOT NULL,
    ancienne_valeur DOUBLE PRECISION NOT NULL,
    nouveau_statut TEXT NOT NULL,
    nouvelle_valeur DOUBLE PRECISION NOT NULL,
    motif TEXT NOT NULL,
    corrige_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    corrige_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_note_correction_motif_chk CHECK (length(btrim(motif)) >= 3)
);
CREATE INDEX IF NOT EXISTS ent_note_correction_eleve_idx
    ON ent_note_correction (etablissement_id, eleve_id, periode_id);
