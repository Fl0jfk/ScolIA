-- 2243 — Socle 12, lot AS2 (situations connues). Réf. : 12_ASSIDUITE.md §8.2, §22.3, §29.3.
-- Tout ce qui dit à l'avance où est l'élève : déclaration, localisation, exclusion ponctuelle de
-- cours, retard annoncé, dispense de présence, sortie… Une situation PRÉ-REMPLIT l'appel et n'est
-- jamais une absence (R-AS4). Reprise idempotente des déclarations (131). Aucune colonne JSON.

CREATE TABLE IF NOT EXISTS ent_asd_situation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN (
        'declaration', 'dispense_presence', 'localisation', 'sortie', 'pfmp', 'examen', 'mission',
        'exclusion_inclusion', 'mobilite', 'autorisation', 'sortie_anticipee', 'retard_annonce',
        'exclusion_cours', 'prise_en_charge_exterieure')),
    debut_jour DATE NOT NULL,
    debut_heure TIME,
    fin_jour DATE NOT NULL,
    fin_heure TIME,
    heure_arrivee_prevue TIME,
    source TEXT NOT NULL CHECK (source IN (
        'famille', 'eleve_majeur', 'accueil', 'vie_scolaire', 'messagerie', 'lien_nominatif', 'socle', 'professeur')),
    origine_ref_type TEXT,
    origine_ref_id UUID,
    motif_id UUID,
    lieu_code TEXT CHECK (lieu_code IS NULL OR lieu_code IN (
        'infirmerie', 'vie_scolaire', 'cdi', 'entretien', 'autre_activite')),
    etat TEXT NOT NULL DEFAULT 'active' CHECK (etat IN ('active', 'close', 'annulee', 'venu')),
    declarant_personne_id UUID,
    declarant_nom TEXT CHECK (declarant_nom IS NULL OR length(btrim(declarant_nom)) BETWEEN 1 AND 200),
    declarant_qualite TEXT CHECK (declarant_qualite IS NULL OR length(declarant_qualite) <= 100),
    -- R-AS6 : une déclaration d'un appelant non reconnu pré-remplit mais ne qualifie rien.
    declarant_verifie BOOLEAN NOT NULL DEFAULT false,
    canal TEXT NOT NULL DEFAULT 'web' CHECK (canal IN ('web', 'mobile', 'telephone', 'visite', 'courriel', 'systeme')),
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_situation_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_asd_situation_jours_chk CHECK (fin_jour >= debut_jour),
    CONSTRAINT ent_asd_situation_heures_chk CHECK (
        fin_jour > debut_jour OR debut_heure IS NULL OR fin_heure IS NULL OR fin_heure > debut_heure),
    CONSTRAINT ent_asd_situation_lieu_chk CHECK (nature <> 'localisation' OR lieu_code IS NOT NULL),
    CONSTRAINT ent_asd_situation_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_asd_situation_motif_fk FOREIGN KEY (etablissement_id, motif_id)
        REFERENCES ent_asd_motif (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_asd_situation_eleve_idx ON ent_asd_situation (etablissement_id, eleve_id, debut_jour, fin_jour);
CREATE INDEX IF NOT EXISTS ent_asd_situation_jour_idx ON ent_asd_situation (etablissement_id, debut_jour, fin_jour) WHERE etat = 'active';
-- Anti-doublon d'une origine (reprise et double écriture des déclarations).
CREATE UNIQUE INDEX IF NOT EXISTS ent_asd_situation_origine_uq
    ON ent_asd_situation (etablissement_id, origine_ref_type, origine_ref_id) WHERE origine_ref_id IS NOT NULL;

-- Texte libre SÉPARÉ (R-AS22) : lu seulement par l'auteur et la vie scolaire ; ≤ 500 caractères.
CREATE TABLE IF NOT EXISTS ent_asd_situation_texte (
    situation_id UUID NOT NULL,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    auteur_personne_id UUID,
    auteur_user_id UUID,
    texte TEXT NOT NULL CHECK (length(btrim(texte)) BETWEEN 1 AND 500),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (situation_id, cree_le),
    CONSTRAINT ent_asd_situation_texte_fk FOREIGN KEY (etablissement_id, situation_id)
        REFERENCES ent_asd_situation (etablissement_id, id)
);

CREATE TABLE IF NOT EXISTS ent_asd_situation_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    situation_id UUID NOT NULL,
    etat_avant TEXT,
    etat_apres TEXT NOT NULL,
    auteur_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_situation_version_fk FOREIGN KEY (etablissement_id, situation_id)
        REFERENCES ent_asd_situation (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_asd_situation_version_idx ON ent_asd_situation_version (etablissement_id, situation_id, cree_le);
DROP TRIGGER IF EXISTS ent_asd_situation_version_ajout_seul ON ent_asd_situation_version;
CREATE TRIGGER ent_asd_situation_version_ajout_seul BEFORE UPDATE OR DELETE ON ent_asd_situation_version
    FOR EACH ROW EXECUTE FUNCTION ent_asd_ajout_seul();

-- ── Reprise des déclarations (131) : une situation par déclaration, écrite une seule fois ─────
-- Le motif de la déclaration (code) est rattaché au motif d'établissement quand il existe.
INSERT INTO ent_asd_situation (id, etablissement_id, eleve_id, nature, debut_jour, debut_heure, fin_jour, fin_heure,
        source, origine_ref_type, origine_ref_id, motif_id, etat, declarant_nom, declarant_qualite,
        declarant_verifie, canal, cree_par, cree_le)
SELECT md5('asd_situation:' || d.id::text)::uuid, d.etablissement_id, d.eleve_id, 'declaration',
       d.debut_jour, d.debut_heure, d.fin_jour, d.fin_heure,
       CASE d.source WHEN 'famille' THEN 'famille' ELSE 'accueil' END,
       'ent_absence_declaration', d.id,
       (SELECT m.id FROM ent_asd_motif m WHERE m.etablissement_id = d.etablissement_id AND m.code = d.motif),
       CASE d.statut WHEN 'annulee' THEN 'annulee' ELSE 'active' END,
       d.declarant_nom, d.declarant_qualite, (d.source = 'famille'),
       CASE d.source WHEN 'famille' THEN 'web' ELSE 'telephone' END,
       d.declarant_user_id, d.created_at
  FROM ent_absence_declaration d
  JOIN ent_eleve e ON e.id = d.eleve_id AND e.etablissement_id = d.etablissement_id
ON CONFLICT DO NOTHING;
INSERT INTO ent_asd_situation_texte (situation_id, etablissement_id, auteur_user_id, texte, cree_le)
SELECT s.id, s.etablissement_id, d.declarant_user_id, left(btrim(d.commentaire), 500), d.created_at
  FROM ent_absence_declaration d
  JOIN ent_asd_situation s ON s.etablissement_id = d.etablissement_id AND s.origine_ref_id = d.id
                          AND s.origine_ref_type = 'ent_absence_declaration'
 WHERE d.commentaire IS NOT NULL AND btrim(d.commentaire) <> ''
ON CONFLICT DO NOTHING;

SELECT ent_rls_poser(ARRAY['ent_asd_situation', 'ent_asd_situation_texte', 'ent_asd_situation_version']);
