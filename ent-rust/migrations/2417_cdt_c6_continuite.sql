-- Socle 20, lot C6 — migration 2417 : continuité. Réf. : 20_CAHIER_DE_TEXTES.md §8.4, §12, CC-11 à CC-30, CC-69, CDT-147 à 150,
-- CDT-202 à 209, CDT-255, CDT-256, CDT-258. Additif et rejouable ; aucune migration existante n'est modifiée ; aucune colonne JSON.
--
-- Le socle 10 n'émet pas encore ses événements de séance (`edt.seance_annulee`…) sur `ENT` : les propositions de report se
-- calculent **sur l'état** des séances (statut `annulee`), de façon idempotente (une seule proposition par séance et par objet) ;
-- le consommateur d'événements du socle 10 appellera la même fonction (`cdt_reports_synchroniser`) quand il existera.

-- 1. Propositions de report : motif du refus (facultatif), cours concerné, une seule proposition par séance et par objet.
ALTER TABLE ent_cdt_report ADD COLUMN IF NOT EXISTS motif TEXT CHECK (motif IS NULL OR length(motif) <= 500);
ALTER TABLE ent_cdt_report ADD COLUMN IF NOT EXISTS cours_id UUID;
CREATE UNIQUE INDEX IF NOT EXISTS ent_cdt_report_objet_uq ON ent_cdt_report (etablissement_id, origine_seance_id, objet, objet_id);
CREATE INDEX IF NOT EXISTS ent_cdt_report_cours_idx ON ent_cdt_report (etablissement_id, cours_id, etat);

-- 2. Séance suivante d'un cours (non annulée), après une séance donnée : cible d'un report. Fenêtre de 150 jours.
CREATE OR REPLACE FUNCTION ent_cdt_prochaine_seance(p_etab UUID, p_cours UUID, p_seance UUID)
RETURNS UUID LANGUAGE sql STABLE AS $$
    SELECT x.id
      FROM ent_edt_seance o
      JOIN ent_edt_seance x ON x.etablissement_id = o.etablissement_id
                           AND x.jour BETWEEN o.jour AND o.jour + 150
                           AND (x.service_id IS NOT DISTINCT FROM o.service_id OR x.classe_id IS NOT DISTINCT FROM o.classe_id)
     WHERE o.etablissement_id = p_etab AND o.id = p_seance
       AND x.id <> o.id AND x.statut <> 'annulee'
       AND (x.jour, x.heure_debut) > (o.jour, o.heure_debut)
       AND ent_cdt_cours_de_seance(p_etab, x.id) = p_cours
     ORDER BY x.jour, x.heure_debut, x.id
     LIMIT 1
$$;

-- 3. Continuité d'un élève : absence longue (PP, CPE) ou exclusion temporaire (socle 13). Aucun motif n'est stocké ; la nature
-- ne se montre jamais aux professeurs (le bandeau dit seulement « élève en continuité à distance »).
ALTER TABLE ent_cdt_continuite ADD COLUMN IF NOT EXISTS nature TEXT NOT NULL DEFAULT 'absence_longue'
    CHECK (nature IN ('absence_longue', 'exclusion'));
CREATE INDEX IF NOT EXISTS ent_cdt_continuite_actives_idx ON ent_cdt_continuite (etablissement_id, eleve_id, du) WHERE close_par_user_id IS NULL;

-- 4. Travail du professeur absent, travail à distance (fermeture), travail d'un poste vacant : un contexte du travail et, pour une
-- séance précise, le lien vers cette séance (« visible à l'heure prévue »).
ALTER TABLE ent_cdt_travail ADD COLUMN IF NOT EXISTS contexte TEXT NOT NULL DEFAULT 'normal'
    CHECK (contexte IN ('normal', 'absence_professeur', 'poste_vacant', 'distance'));
ALTER TABLE ent_cdt_travail ADD COLUMN IF NOT EXISTS seance_speciale_id UUID;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_cdt_travail_seance_speciale_fk') THEN
        ALTER TABLE ent_cdt_travail ADD CONSTRAINT ent_cdt_travail_seance_speciale_fk
            FOREIGN KEY (etablissement_id, seance_speciale_id) REFERENCES ent_edt_seance (etablissement_id, id) ON DELETE RESTRICT;
    END IF;
END
$$;
CREATE INDEX IF NOT EXISTS ent_cdt_travail_contexte_idx ON ent_cdt_travail (etablissement_id, cours_id, contexte) WHERE contexte <> 'normal';

-- 5. Copies en attente à l'ouverture d'une suppléance (CDT-256, CC-69) : une décision par cours et par fois, jamais de silence.
CREATE TABLE IF NOT EXISTS ent_cdt_copies_decision (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    cours_id UUID NOT NULL,
    decision TEXT NOT NULL CHECK (decision IN ('je_corrige', 'titulaire_corrigera', 'rendues_sans_correction')),
    nb_copies INTEGER NOT NULL CHECK (nb_copies >= 0),
    decide_par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_copies_decision_cours_fk FOREIGN KEY (cours_id, etablissement_id)
        REFERENCES ent_cours (id, etablissement_id) ON DELETE RESTRICT
);
DROP TRIGGER IF EXISTS ent_cdt_copies_decision_ajout_seul ON ent_cdt_copies_decision;
CREATE TRIGGER ent_cdt_copies_decision_ajout_seul BEFORE UPDATE OR DELETE ON ent_cdt_copies_decision
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_ajout_seul();
SELECT ent_rls_poser(ARRAY['ent_cdt_copies_decision']);

-- 6. Port « aménagements utiles » du socle 35, **en simulation** tant que le socle 35 n'existe pas : une seule table et une seule
-- fonction Rust (`cdt_amenagements_utiles`) à remplacer. Elle ne porte que des **faits utiles** d'un catalogue fermé (jamais un
-- diagnostic, un intitulé de plan ni un motif) : c'est tout ce que le cahier est autorisé à connaître (§13.1).
CREATE TABLE IF NOT EXISTS ent_cdt_amenagement_simule (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    code TEXT NOT NULL CHECK (code IN ('temps_majore', 'version_allegee', 'police_agrandie', 'document_audio',
                                       'lecture_consigne', 'ordinateur', 'pas_de_copie_tableau')),
    valeur_pct SMALLINT CHECK (valeur_pct IS NULL OR valeur_pct BETWEEN 1 AND 100),
    du DATE NOT NULL,
    au DATE,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_amenagement_simule_dates_chk CHECK (au IS NULL OR au >= du),
    CONSTRAINT ent_cdt_amenagement_simule_pct_chk CHECK (code = 'temps_majore' OR valeur_pct IS NULL),
    CONSTRAINT ent_cdt_amenagement_simule_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_cdt_amenagement_simule_idx ON ent_cdt_amenagement_simule (etablissement_id, eleve_id, du);
SELECT ent_rls_poser(ARRAY['ent_cdt_amenagement_simule']);
