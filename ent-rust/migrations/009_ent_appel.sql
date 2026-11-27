-- Appel enseignant → vérité absences partagée (ent_absence).
-- Effectif du jour = dates d'effet (ent_eleve_periode), pas une fiche figée.

INSERT INTO ent_absence_motif (code, libelle) VALUES
    ('appel', 'Absent à l''appel')
ON CONFLICT (code) DO NOTHING;

CREATE TABLE IF NOT EXISTS ent_appel (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    classe TEXT NOT NULL,
    jour DATE NOT NULL,
    auteur_user_id UUID NOT NULL REFERENCES ent_user (id),
    statut TEXT NOT NULL DEFAULT 'ouvert',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    closed_at TIMESTAMPTZ,
    CONSTRAINT ent_appel_statut_chk CHECK (statut IN ('ouvert', 'clos')),
    CONSTRAINT ent_appel_classe_jour_uq UNIQUE (etablissement_id, classe, jour)
);

CREATE INDEX IF NOT EXISTS ent_appel_etab_jour_idx
    ON ent_appel (etablissement_id, jour);

CREATE TABLE IF NOT EXISTS ent_appel_ligne (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    appel_id UUID NOT NULL REFERENCES ent_appel (id) ON DELETE CASCADE,
    eleve_id UUID NOT NULL,
    etat TEXT NOT NULL DEFAULT 'present',
    CONSTRAINT ent_appel_ligne_etat_chk CHECK (etat IN ('present', 'absent', 'retard')),
    CONSTRAINT ent_appel_ligne_uq UNIQUE (appel_id, eleve_id)
);

CREATE INDEX IF NOT EXISTS ent_appel_ligne_eleve_idx
    ON ent_appel_ligne (etablissement_id, eleve_id);
