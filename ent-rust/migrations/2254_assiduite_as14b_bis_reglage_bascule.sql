-- 2254 — Socle 12, lot AS14b-bis (tableau de bord de la vie scolaire en panneaux).
-- RÉGLAGE D'ÉTABLISSEMENT : l'heure de bascule du tableau de bord (heure locale de l'établissement). Avant cette
-- heure le panneau des absents du jour est placé en premier et en grand ; après, les exclus de cours et les
-- appels non faits. 11 h par défaut (ligne absente = 11:00 ; le code applique le défaut, rien n'est semé).
-- Pourquoi pas dans `ent_asd_regle` : la règle d'assiduité est DATÉE et figée par un mois arrêté (R-AS15) ; une
-- préférence d'affichage ne doit ni créer une version de règle ni être gelée. Réglage du même ordre, une ligne par
-- établissement, modifiable à tout moment.
-- Additive, rejouable (E8), sans colonne JSON, sans BYTEA ; RLS par `ent_rls_poser`.

CREATE TABLE IF NOT EXISTS ent_asd_tableau_reglage (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    heure_bascule TIME NOT NULL DEFAULT '11:00' CHECK (heure_bascule BETWEEN '06:00' AND '18:00'),
    modifie_par UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_tableau_reglage_etab_uq UNIQUE (etablissement_id)
);

SELECT ent_rls_poser(ARRAY['ent_asd_tableau_reglage']);
