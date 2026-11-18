-- Socle 21, lot V1 (suite) — 464 : résultats d'évaluation, historique en ajout seul (D6), aucune
-- suppression physique (D7), journal des corrections après l'arrêt, résultats externes de reprise.
-- Additif, rejouable. Aucune colonne JSON, aucun BYTEA.
--
-- `eva_resultat` succède à `ent_note` (008, 022, 180). Un résultat est un fait daté : il se REMPLACE
-- (l'ancienne valeur reste dans `eva_resultat_historique`) ou s'ANNULE (statut `annule`, motif), jamais
-- ne se supprime (R-V6). Les gardes sont en base : aucun écrivain, présent ou futur, ne peut les contourner.

CREATE TABLE IF NOT EXISTS eva_resultat (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    evaluation_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    statut TEXT NOT NULL CHECK (statut IN
        ('note', 'absent', 'a_rattraper', 'dispense', 'non_rendu', 'non_note', 'annule')),
    -- Décimal exact (jamais DOUBLE PRECISION) ; présent si et seulement si le statut est `note`.
    valeur NUMERIC(7, 3) CHECK (valeur IS NULL OR valeur >= 0),
    -- Marque interne « aménagé » (jamais imprimée sur un document à un tiers, R-V16).
    amenage BOOLEAN NOT NULL DEFAULT false,
    -- Mot au dossier de l'élève (publié avec la note) ; note privée du professeur (communicable, R-V41).
    commentaire_eleve TEXT CHECK (commentaire_eleve IS NULL OR char_length(commentaire_eleve) <= 1000),
    note_privee TEXT CHECK (note_privee IS NULL OR char_length(note_privee) <= 2000),
    rattrapage_le DATE,
    source TEXT NOT NULL DEFAULT 'saisie'
        CHECK (source IN ('saisie', 'appel', 'dispense', 'import', 'hors_ligne', 'reprise')),
    -- La dernière écriture était une correction après l'arrêt. Son motif voyage dans la ligne elle-même
    -- (`motif_correction`, jamais dans une variable de session) : il va au journal et ne survit pas à une
    -- écriture ordinaire (le déclencheur le remet à NULL).
    derniere_ecriture_correction BOOLEAN NOT NULL DEFAULT false,
    motif_correction TEXT CHECK (motif_correction IS NULL OR char_length(btrim(motif_correction)) >= 3),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    saisi_par UUID,
    saisi_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_resultat_valeur_chk CHECK ((statut = 'note') = (valeur IS NOT NULL)),
    CONSTRAINT eva_resultat_rattrapage_chk CHECK (rattrapage_le IS NULL OR statut IN ('absent', 'a_rattraper', 'non_rendu')),
    CONSTRAINT eva_resultat_eval_fk FOREIGN KEY (evaluation_id, etablissement_id)
        REFERENCES eva_evaluation (id, etablissement_id),
    CONSTRAINT eva_resultat_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id),
    -- D8 : un seul résultat par (évaluation, élève).
    CONSTRAINT eva_resultat_uq UNIQUE (evaluation_id, eleve_id),
    CONSTRAINT eva_resultat_id_etab_uq UNIQUE (id, etablissement_id)
);
CREATE INDEX IF NOT EXISTS eva_resultat_eleve_idx ON eva_resultat (etablissement_id, eleve_id);
CREATE INDEX IF NOT EXISTS eva_resultat_eval_idx ON eva_resultat (etablissement_id, evaluation_id);

-- Historique en ajout seul : une ligne par écriture (création ou changement de statut ou de valeur).
CREATE TABLE IF NOT EXISTS eva_resultat_historique (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    resultat_id UUID NOT NULL,
    evaluation_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    statut_avant TEXT,
    valeur_avant NUMERIC(7, 3),
    statut_apres TEXT NOT NULL,
    valeur_apres NUMERIC(7, 3),
    -- F5 : le reste de la ligne est journalisé aussi (avant puis après).
    commentaire_avant TEXT,
    commentaire_apres TEXT,
    amenage_avant BOOLEAN,
    amenage_apres BOOLEAN,
    rattrapage_le_avant DATE,
    rattrapage_le_apres DATE,
    note_privee_avant TEXT,
    note_privee_apres TEXT,
    -- Correction après l'arrêt de la période de l'élève : motif obligatoire.
    correction BOOLEAN NOT NULL,
    motif TEXT,
    source TEXT NOT NULL,
    auteur UUID,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_resultat_histo_fk FOREIGN KEY (resultat_id, etablissement_id)
        REFERENCES eva_resultat (id, etablissement_id),
    CONSTRAINT eva_resultat_histo_correction_chk CHECK (NOT correction OR (motif IS NOT NULL AND length(btrim(motif)) >= 3)),
    CONSTRAINT eva_resultat_histo_uq UNIQUE (resultat_id, version)
);
CREATE INDEX IF NOT EXISTS eva_resultat_histo_eleve_idx ON eva_resultat_historique (etablissement_id, eleve_id, le);

-- Les gardes sont SANS dérogation de session : ni `SET`, ni variable de transaction ne les contourne.
-- La reprise de l'existant (467) est la seule écriture qui échappe à ce déclencheur : c'est une fonction
-- SECURITY DEFINER (propriétaire des tables) qui désactive ces deux déclencheurs le temps de sa transaction,
-- comme `ent_fin_purger_etablissement` (522), et écrit elle-même le journal complet.
CREATE OR REPLACE FUNCTION ent_trg_eva_resultat() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    ev RECORD;
    per RECORD;
    v_corr boolean := false;
    v_motif text := nullif(btrim(coalesce(NEW.motif_correction, '')), '');
    v_arretee boolean;
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'eva_resultat : suppression physique interdite, remplacer la valeur ou annuler (D7, R-V6)'
            USING ERRCODE = '23514';
    END IF;
    -- `reprise` n'est écrite que par la fonction de reprise (qui désactive ce déclencheur) : jamais ici.
    IF NEW.source = 'reprise' THEN
        RAISE EXCEPTION 'eva_resultat : la source reprise est réservée à la reprise de l''existant' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'UPDATE' AND NEW.source IS DISTINCT FROM OLD.source THEN
        RAISE EXCEPTION 'eva_resultat : la source d''un résultat ne change pas' USING ERRCODE = '23514';
    END IF;
    SELECT e.bareme, e.hors_bareme_autorise, e.etat, e.mode, e.date_eval, e.cours_id
      INTO ev
      FROM eva_evaluation e
     WHERE e.id = NEW.evaluation_id AND e.etablissement_id = NEW.etablissement_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'eva_resultat : évaluation introuvable' USING ERRCODE = '23503';
    END IF;
    IF TG_OP = 'UPDATE' AND (NEW.evaluation_id IS DISTINCT FROM OLD.evaluation_id
                             OR NEW.eleve_id IS DISTINCT FROM OLD.eleve_id) THEN
        RAISE EXCEPTION 'eva_resultat : l''évaluation et l''élève ne changent pas' USING ERRCODE = '23514';
    END IF;
    IF ev.mode <> 'note' THEN
        RAISE EXCEPTION 'eva_resultat : évaluation de positionnement, pas de résultat chiffré (socle 22)' USING ERRCODE = '23514';
    END IF;
    IF ev.etat = 'annulee' THEN
        RAISE EXCEPTION 'eva_resultat : évaluation annulée' USING ERRCODE = '23514';
    END IF;
    IF NEW.statut = 'note' AND NEW.valeur > ev.bareme AND NOT ev.hors_bareme_autorise THEN
        RAISE EXCEPTION 'eva_resultat : % sur % : vérifiez le barème', NEW.valeur, ev.bareme USING ERRCODE = '23514';
    END IF;
    -- Public : un élève hors du public du cours à la date n'a pas de résultat (non concerné, R-V2).
    IF NOT EXISTS (SELECT 1 FROM ent_public_cours_jour(NEW.etablissement_id, ev.date_eval, ev.cours_id) pu
                    WHERE pu.eleve_id = NEW.eleve_id) THEN
        RAISE EXCEPTION 'eva_resultat : élève non concerné par cette évaluation à sa date' USING ERRCODE = '23514';
    END IF;
    -- Période de L'ÉLÈVE (R-V29) : classe de rattachement à la date de l'évaluation. Un élève sans classe ou
    -- sans période à cette date est VERROUILLÉ (M8) : seule une correction motivée écrit.
    SELECT * INTO per FROM eva_periode_eleve_jour(NEW.etablissement_id, NEW.eleve_id, ev.date_eval);
    v_arretee := NOT FOUND OR per.etat <> 'ouverte';
    IF v_arretee THEN
        -- Rattrapage autorisé (E4) : en vigueur à ce jour, et seulement sur une ligne absente ou restée sans
        -- note (absent, à rattraper, non rendu). Une note déjà saisie ne s'écrase pas par ce biais.
        IF eva_rattrapage_en_vigueur(NEW.etablissement_id, NEW.evaluation_id, NEW.eleve_id)
           AND (TG_OP = 'INSERT' OR OLD.statut IN ('absent', 'a_rattraper', 'non_rendu')) THEN
            v_corr := false;
        ELSE
            -- Après l'arrêt : seulement une correction motivée (la direction), jamais une saisie ordinaire.
            IF v_motif IS NULL OR length(v_motif) < 3 THEN
                RAISE EXCEPTION 'eva_resultat : notes arrêtées pour cet élève, correction motivée requise (R-V10)'
                    USING ERRCODE = '23514';
            END IF;
            v_corr := true;
        END IF;
    END IF;
    NEW.derniere_ecriture_correction := v_corr;
    NEW.motif_correction := CASE WHEN v_corr THEN v_motif END;
    IF TG_OP = 'UPDATE' THEN
        NEW.version := OLD.version + 1;
        NEW.saisi_le := now();
        -- Aucune écriture sans effet : la ligne reste telle quelle, la version aussi (rien n'est journalisé).
        IF NEW.statut IS NOT DISTINCT FROM OLD.statut AND NEW.valeur IS NOT DISTINCT FROM OLD.valeur
           AND NEW.rattrapage_le IS NOT DISTINCT FROM OLD.rattrapage_le
           AND NEW.amenage IS NOT DISTINCT FROM OLD.amenage
           AND NEW.commentaire_eleve IS NOT DISTINCT FROM OLD.commentaire_eleve
           AND NEW.note_privee IS NOT DISTINCT FROM OLD.note_privee THEN
            RETURN OLD;
        END IF;
    END IF;
    RETURN NEW;
END
$$;

-- Le journal s'écrit TOUJOURS, sans condition ni dérogation : une ligne par écriture qui change quelque chose.
CREATE OR REPLACE FUNCTION ent_trg_eva_resultat_historique() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.version = OLD.version THEN
        RETURN NULL;
    END IF;
    INSERT INTO eva_resultat_historique (
        id, etablissement_id, resultat_id, evaluation_id, eleve_id, version, statut_avant, valeur_avant,
        statut_apres, valeur_apres, commentaire_avant, commentaire_apres, amenage_avant, amenage_apres,
        rattrapage_le_avant, rattrapage_le_apres, note_privee_avant, note_privee_apres,
        correction, motif, source, auteur)
    VALUES (
        gen_random_uuid(), NEW.etablissement_id, NEW.id, NEW.evaluation_id, NEW.eleve_id, NEW.version,
        CASE WHEN TG_OP = 'UPDATE' THEN OLD.statut END, CASE WHEN TG_OP = 'UPDATE' THEN OLD.valeur END,
        NEW.statut, NEW.valeur,
        CASE WHEN TG_OP = 'UPDATE' THEN OLD.commentaire_eleve END, NEW.commentaire_eleve,
        CASE WHEN TG_OP = 'UPDATE' THEN OLD.amenage END, NEW.amenage,
        CASE WHEN TG_OP = 'UPDATE' THEN OLD.rattrapage_le END, NEW.rattrapage_le,
        CASE WHEN TG_OP = 'UPDATE' THEN OLD.note_privee END, NEW.note_privee,
        NEW.derniere_ecriture_correction, NEW.motif_correction, NEW.source, NEW.saisi_par);
    RETURN NULL;
END
$$;

DROP TRIGGER IF EXISTS eva_resultat_garde ON eva_resultat;
CREATE TRIGGER eva_resultat_garde BEFORE INSERT OR UPDATE OR DELETE ON eva_resultat
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_resultat();
DROP TRIGGER IF EXISTS eva_resultat_histo ON eva_resultat;
CREATE TRIGGER eva_resultat_histo AFTER INSERT OR UPDATE ON eva_resultat
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_resultat_historique();
DROP TRIGGER IF EXISTS eva_resultat_histo_ajout_seul ON eva_resultat_historique;
CREATE TRIGGER eva_resultat_histo_ajout_seul BEFORE UPDATE OR DELETE ON eva_resultat_historique
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();
-- TRUNCATE (521, 650) : `TRUNCATE eva_evaluation CASCADE` ou `TRUNCATE ent_eleve CASCADE` viderait sinon ces tables.
DROP TRIGGER IF EXISTS eva_resultat_vidage ON eva_resultat;
CREATE TRIGGER eva_resultat_vidage BEFORE TRUNCATE ON eva_resultat
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_resultat_histo_vidage ON eva_resultat_historique;
CREATE TRIGGER eva_resultat_histo_vidage BEFORE TRUNCATE ON eva_resultat_historique
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

-- Détail par exercice d'un barème détaillé : la somme vaut la note (§8.9).
CREATE TABLE IF NOT EXISTS eva_resultat_item (
    resultat_id UUID NOT NULL,
    item_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    valeur NUMERIC(7, 3) NOT NULL CHECK (valeur >= 0),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (resultat_id, item_id),
    CONSTRAINT eva_resultat_item_resultat_fk FOREIGN KEY (resultat_id, etablissement_id)
        REFERENCES eva_resultat (id, etablissement_id),
    CONSTRAINT eva_resultat_item_item_fk FOREIGN KEY (item_id, etablissement_id)
        REFERENCES eva_bareme_item (id, etablissement_id)
);

-- ── Publication : ce que la famille et l'élève peuvent lire (R-V7) ─────────────────────────
-- Un résultat n'est visible que si son évaluation est publiée. La vue ne porte ni la note privée du
-- professeur, ni le motif, ni l'auteur. `security_invoker` : la RLS de l'appelant s'applique.
CREATE OR REPLACE VIEW eva_resultat_publie WITH (security_invoker = true) AS
SELECT r.id, r.etablissement_id, r.evaluation_id, r.eleve_id, r.statut, r.valeur,
       r.commentaire_eleve, r.rattrapage_le, e.cours_id, e.titre, e.date_eval, e.bareme, e.coef,
       e.comptage, e.etat AS etat_evaluation, e.publiee_le
  FROM eva_resultat r
  JOIN eva_evaluation e ON e.id = r.evaluation_id AND e.etablissement_id = r.etablissement_id
 WHERE e.publiee_le IS NOT NULL AND e.etat IN ('ouverte', 'annulee');

-- ── Moyennes statuées (« en attente » : R-02, CE-04, CE-05) ───────────────────────────────
CREATE TABLE IF NOT EXISTS eva_moyenne_statut (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    cours_id UUID,
    matiere_id UUID,
    periode_id UUID NOT NULL,
    statut TEXT NOT NULL CHECK (statut IN ('en_attente', 'non_significative')),
    motif_interne TEXT CHECK (motif_interne IS NULL OR char_length(motif_interne) <= 500),
    par UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_moyenne_statut_cible_chk CHECK ((cours_id IS NULL) <> (matiere_id IS NULL)),
    CONSTRAINT eva_moyenne_statut_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id),
    CONSTRAINT eva_moyenne_statut_cours_fk FOREIGN KEY (cours_id, etablissement_id)
        REFERENCES ent_cours (id, etablissement_id),
    CONSTRAINT eva_moyenne_statut_matiere_fk FOREIGN KEY (matiere_id, etablissement_id)
        REFERENCES ent_matiere (id, etablissement_id),
    CONSTRAINT eva_moyenne_statut_periode_fk FOREIGN KEY (periode_id, etablissement_id)
        REFERENCES ent_periode (id, etablissement_id)
);
CREATE UNIQUE INDEX IF NOT EXISTS eva_moyenne_statut_uq ON eva_moyenne_statut (
    etablissement_id, eleve_id, periode_id, coalesce(cours_id, matiere_id));

SELECT ent_rls_poser(ARRAY[
    'eva_resultat', 'eva_resultat_historique', 'eva_resultat_item', 'eva_moyenne_statut'
]);
