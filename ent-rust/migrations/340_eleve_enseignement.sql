-- Socle 5, lot S5 (`05_STRUCTURE.md` §7.2 n° 1, §9.9, §9.10, STR-43, STR-62, R-S16, Q-S2) :
-- enseignements SUIVIS par l'élève, distincts de ses groupes.
--
-- L'élève *suit* un enseignement (son choix : exports, vœux, facturation d'une option payante) ;
-- il est *membre* d'un groupe (l'organisation concrète). Le lien est un CONTRÔLE
-- (`ent_enseignement_choix_sans_groupe`), jamais une écriture silencieuse.
--
-- Cycle de vie (tenu par la base) :
--   choisi ──► accepte ──► quitte (date, nature R-S16, motif)
--     ├──────► refuse   (admission refusée)       refuse, retire, quitte : terminaux
--     └──────► retire   (choix retiré avant décision)
--   Une ligne ne se supprime pas ; un enseignement quitté garde sa nature et ses dates.
--   Les deux tables d'options historiques (`ent_eleve_option`, `ent_eleve_option_annee`) sont
--   REPRISES ici (aucune option perdue : ce qui ne peut l'être est rangé dans `_a_traiter`) et
--   un pont à sens unique continue de refléter leurs écritures. Rien n'est créé en dehors de
--   l'établissement ni sans trace.
-- Additif et rejouable.

CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_id_etab_s5_uq ON ent_eleve (id, etablissement_id);

CREATE TABLE IF NOT EXISTS ent_eleve_enseignement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    offre_id UUID NOT NULL,
    etat TEXT NOT NULL DEFAULT 'choisi' CHECK (etat IN ('choisi', 'accepte', 'refuse', 'retire', 'quitte')),
    choisi_le DATE NOT NULL DEFAULT current_date,
    -- Date d'effet de l'acceptation (début de l'enseignement suivi).
    debut_le DATE,
    -- Dernier jour suivi (inclus) pour un enseignement quitté.
    quitte_le DATE,
    -- R-S16 : l'enseignement quitté garde sa nature.
    fin_nature TEXT CHECK (fin_nature IS NULL OR fin_nature IN
        ('changement_en_cours_annee', 'abandon_fin_premiere', 'fermeture_offre', 'depart', 'correction')),
    motif TEXT CHECK (motif IS NULL OR char_length(motif) <= 300),
    -- Avis rendu pour une admission sur avis, test ou dossier.
    avis TEXT CHECK (avis IS NULL OR avis IN ('favorable', 'defavorable', 'reserve')),
    avis_le TIMESTAMPTZ,
    avis_par UUID,
    source TEXT NOT NULL DEFAULT 'saisie' CHECK (source IN ('saisie', 'reprise_option', 'famille', 'inscription')),
    choisi_par UUID,
    decide_par UUID,
    modifie_par UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eleve_enseignement_fin_chk CHECK (
        (etat = 'quitte' AND fin_nature IS NOT NULL AND quitte_le IS NOT NULL AND debut_le IS NOT NULL
            AND quitte_le >= debut_le AND motif IS NOT NULL)
        OR (etat <> 'quitte' AND fin_nature IS NULL AND quitte_le IS NULL)),
    CONSTRAINT ent_eleve_enseignement_debut_chk CHECK (
        (etat IN ('accepte', 'quitte') AND debut_le IS NOT NULL) OR etat NOT IN ('accepte', 'quitte')),
    CONSTRAINT ent_eleve_enseignement_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id),
    CONSTRAINT ent_eleve_enseignement_offre_fk FOREIGN KEY (offre_id, etablissement_id)
        REFERENCES ent_offre_enseignement (id, etablissement_id),
    CONSTRAINT ent_eleve_enseignement_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id)
);
-- Un élève suit une offre une seule fois à la fois ; l'historique (quitté, refusé) s'accumule.
CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_enseignement_actif_uq
    ON ent_eleve_enseignement (eleve_id, offre_id) WHERE etat IN ('choisi', 'accepte');
CREATE INDEX IF NOT EXISTS ent_eleve_enseignement_eleve_idx ON ent_eleve_enseignement (etablissement_id, eleve_id, annee_id);
CREATE INDEX IF NOT EXISTS ent_eleve_enseignement_offre_idx ON ent_eleve_enseignement (etablissement_id, offre_id, etat);

-- Historique en ajout seul des changements d'état.
CREATE TABLE IF NOT EXISTS ent_eleve_enseignement_histo (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    enseignement_id UUID NOT NULL REFERENCES ent_eleve_enseignement (id),
    de_etat TEXT,
    vers_etat TEXT NOT NULL,
    effet_le DATE,
    fin_nature TEXT,
    motif TEXT,
    par UUID,
    le TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_eleve_enseignement_histo_idx ON ent_eleve_enseignement_histo (etablissement_id, enseignement_id, le);

CREATE OR REPLACE FUNCTION ent_trg_histo_ajout_seul() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'historique en ajout seul' USING ERRCODE = '42501';
END $$;
DROP TRIGGER IF EXISTS ent_eleve_enseignement_histo_ajout_seul_trg ON ent_eleve_enseignement_histo;
CREATE TRIGGER ent_eleve_enseignement_histo_ajout_seul_trg BEFORE UPDATE OR DELETE ON ent_eleve_enseignement_histo
    FOR EACH ROW EXECUTE FUNCTION ent_trg_histo_ajout_seul();

-- ───────────────────────── Gardes de la table principale ─────────────────────────
CREATE OR REPLACE FUNCTION ent_trg_eleve_enseignement() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v_annee UUID; v_adm TEXT; v_close BOOLEAN; v_sortie BOOLEAN;
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'enseignement suivi : jamais de suppression (quitter, retirer ou corriger)' USING ERRCODE = '42501';
    END IF;
    IF TG_OP = 'UPDATE' THEN
        -- Fusion de deux fiches : seul `eleve_id` bouge.
        IF ent_fusion_en_cours() AND (to_jsonb(NEW) - 'eleve_id' - 'modifie_le') = (to_jsonb(OLD) - 'eleve_id' - 'modifie_le') THEN
            RETURN NEW;
        END IF;
        IF (NEW.id, NEW.etablissement_id, NEW.annee_id, NEW.eleve_id, NEW.offre_id, NEW.choisi_le, NEW.source)
           IS DISTINCT FROM (OLD.id, OLD.etablissement_id, OLD.annee_id, OLD.eleve_id, OLD.offre_id, OLD.choisi_le, OLD.source) THEN
            RAISE EXCEPTION 'enseignement_immuable: élève, offre, année, date de choix et source ne changent pas' USING ERRCODE = '23514';
        END IF;
        IF OLD.etat IN ('refuse', 'retire', 'quitte') THEN
            RAISE EXCEPTION 'enseignement_termine: un enseignement refusé, retiré ou quitté ne change plus' USING ERRCODE = '23514';
        END IF;
        IF NOT ((OLD.etat = 'choisi' AND NEW.etat IN ('choisi', 'accepte', 'refuse', 'retire'))
                OR (OLD.etat = 'accepte' AND NEW.etat IN ('accepte', 'quitte'))) THEN
            RAISE EXCEPTION 'enseignement_transition: % vers %', OLD.etat, NEW.etat USING ERRCODE = '23514';
        END IF;
        IF OLD.etat = 'accepte' AND NEW.etat = 'accepte' THEN
            RAISE EXCEPTION 'enseignement_transition: un enseignement accepté ne change que pour être quitté' USING ERRCODE = '23514';
        END IF;
        IF OLD.etat = 'accepte' AND NEW.debut_le IS DISTINCT FROM OLD.debut_le THEN
            RAISE EXCEPTION 'enseignement_transition: la date de début ne change plus' USING ERRCODE = '23514';
        END IF;
    END IF;

    SELECT annee_id, admission INTO v_annee, v_adm FROM ent_offre_enseignement o
     WHERE o.id = NEW.offre_id AND o.etablissement_id = NEW.etablissement_id;
    IF v_annee IS DISTINCT FROM NEW.annee_id THEN
        RAISE EXCEPTION 'enseignement_incoherent: offre d''une autre année' USING ERRCODE = '23514';
    END IF;
    IF NEW.source <> 'reprise_option' THEN
        v_close := ent_offre_annee_close(NEW.etablissement_id, NEW.annee_id);
        IF v_close THEN
            RAISE EXCEPTION 'enseignement_annee_cloturee' USING ERRCODE = '23514';
        END IF;
        IF TG_OP = 'INSERT' AND EXISTS (SELECT 1 FROM ent_eleve e WHERE e.id = NEW.eleve_id AND e.fusionne_dans IS NOT NULL) THEN
            RAISE EXCEPTION 'enseignement_incoherent: fiche fusionnée' USING ERRCODE = '23514';
        END IF;
        -- Admission conditionnée : pas d'acceptation sans avis favorable.
        IF NEW.etat = 'accepte' AND v_adm <> 'libre' AND NEW.avis IS DISTINCT FROM 'favorable' THEN
            RAISE EXCEPTION 'enseignement_admission: avis favorable requis (%)', v_adm USING ERRCODE = '23514';
        END IF;
        IF NEW.etat = 'refuse' AND v_adm = 'libre' THEN
            RAISE EXCEPTION 'enseignement_admission: une offre libre ne se refuse pas' USING ERRCODE = '23514';
        END IF;
    END IF;
    NEW.modifie_le := now();
    RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS ent_eleve_enseignement_garde_trg ON ent_eleve_enseignement;
CREATE TRIGGER ent_eleve_enseignement_garde_trg BEFORE INSERT OR UPDATE OR DELETE ON ent_eleve_enseignement
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eleve_enseignement();

-- Historique : une ligne à chaque création et à chaque changement d'état ou d'avis.
CREATE OR REPLACE FUNCTION ent_trg_eleve_enseignement_histo() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.etat = OLD.etat AND NEW.avis IS NOT DISTINCT FROM OLD.avis THEN
        RETURN NULL;
    END IF;
    INSERT INTO ent_eleve_enseignement_histo
        (id, etablissement_id, enseignement_id, de_etat, vers_etat, effet_le, fin_nature, motif, par)
    VALUES (gen_random_uuid(), NEW.etablissement_id, NEW.id,
            CASE WHEN TG_OP = 'UPDATE' THEN OLD.etat END, NEW.etat,
            coalesce(NEW.quitte_le, NEW.debut_le, NEW.choisi_le), NEW.fin_nature, NEW.motif,
            coalesce(NEW.modifie_par, NEW.decide_par, NEW.choisi_par));
    RETURN NULL;
END $$;
DROP TRIGGER IF EXISTS ent_eleve_enseignement_histo_trg ON ent_eleve_enseignement;
CREATE TRIGGER ent_eleve_enseignement_histo_trg AFTER INSERT OR UPDATE ON ent_eleve_enseignement
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eleve_enseignement_histo();

-- ───────────────────────── Reprise des options historiques ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_eleve_enseignement_reprise (
    source TEXT NOT NULL CHECK (source IN ('option', 'option_annee')),
    option_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    enseignement_id UUID NOT NULL REFERENCES ent_eleve_enseignement (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (source, option_id)
);
CREATE TABLE IF NOT EXISTS ent_eleve_enseignement_a_traiter (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    source TEXT NOT NULL,
    option_id UUID NOT NULL,
    cause TEXT NOT NULL,
    detecte_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    traite_le TIMESTAMPTZ,
    UNIQUE (source, option_id, cause)
);

DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_eleve_enseignement', 'ent_eleve_enseignement_histo',
                             'ent_eleve_enseignement_reprise', 'ent_eleve_enseignement_a_traiter']
    LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('DROP POLICY IF EXISTS ent_rls_etablissement ON %I', t);
        EXECUTE format($p$CREATE POLICY ent_rls_etablissement ON %I FOR ALL
            USING (etablissement_id = (SELECT ent_rls_etablissement())
                OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
            WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
                OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))$p$, t);
    END LOOP;
END $$;

-- Offre de l'année portant ce code, créée seulement par la reprise (jamais pour un code vide) ;
-- le groupe homonyme de l'année, s'il existe, lui est associé.
CREATE OR REPLACE FUNCTION ent_offre_assurer_depuis_option(p_etab UUID, p_annee UUID, p_code TEXT, p_libelle TEXT, p_nature TEXT)
RETURNS UUID LANGUAGE plpgsql AS $$
DECLARE v_id UUID; v_code TEXT := left(btrim(p_code), 40); v_groupe UUID; v_matiere UUID;
BEGIN
    SELECT id INTO v_id FROM ent_offre_enseignement WHERE etablissement_id = p_etab AND annee_id = p_annee AND code = v_code;
    IF v_id IS NULL THEN
        SELECT m.id INTO v_matiere FROM ent_matiere m
         WHERE m.etablissement_id = p_etab AND upper(m.code) = upper(v_code) AND m.archivee_le IS NULL LIMIT 1;
        v_id := gen_random_uuid();
        INSERT INTO ent_offre_enseignement (id, etablissement_id, annee_id, code, libelle, matiere_id, modalite, source)
        VALUES (v_id, p_etab, p_annee, v_code,
                left(coalesce(nullif(btrim(p_libelle), ''), v_code), 160), v_matiere,
                CASE WHEN p_nature = 'academique' THEN 'option_facultative' ELSE 'autre' END, 'reprise_option');
    END IF;
    SELECT g.id INTO v_groupe FROM ent_groupe g WHERE g.etablissement_id = p_etab AND g.annee_id = p_annee AND g.code = btrim(p_code);
    IF v_groupe IS NOT NULL AND NOT EXISTS (SELECT 1 FROM ent_offre_groupe og WHERE og.groupe_id = v_groupe) THEN
        INSERT INTO ent_offre_groupe (id, etablissement_id, offre_id, groupe_id)
        VALUES (gen_random_uuid(), p_etab, v_id, v_groupe) ON CONFLICT DO NOTHING;
    END IF;
    RETURN v_id;
END $$;

CREATE OR REPLACE FUNCTION ent_eleve_enseignement_a_traiter_ajouter(p_etab UUID, p_eleve UUID, p_source TEXT, p_option UUID, p_cause TEXT)
RETURNS VOID LANGUAGE sql AS $$
    INSERT INTO ent_eleve_enseignement_a_traiter (etablissement_id, eleve_id, source, option_id, cause)
    VALUES (p_etab, p_eleve, p_source, p_option, p_cause) ON CONFLICT DO NOTHING
$$;

-- Option héritée (dates) → enseignement suivi. Renvoie l'identifiant de l'enseignement ou NULL.
CREATE OR REPLACE FUNCTION ent_eleve_enseignement_depuis_option(o ent_eleve_option) RETURNS UUID
LANGUAGE plpgsql AS $$
DECLARE v_an ent_annee_scolaire; v_offre UUID; v_ens UUID; v_fin DATE;
BEGIN
    SELECT enseignement_id INTO v_ens FROM ent_eleve_enseignement_reprise WHERE source = 'option' AND option_id = o.id;
    SELECT * INTO v_an FROM ent_annee_scolaire a
     WHERE a.etablissement_id = o.etablissement_id AND o.date_debut BETWEEN a.date_debut AND a.date_fin
     ORDER BY a.courante DESC, a.date_debut DESC LIMIT 1;
    IF v_ens IS NULL THEN
        IF btrim(coalesce(o.code, '')) = '' THEN
            PERFORM ent_eleve_enseignement_a_traiter_ajouter(o.etablissement_id, o.eleve_id, 'option', o.id, 'code_vide');
            RETURN NULL;
        END IF;
        IF v_an.id IS NULL THEN
            PERFORM ent_eleve_enseignement_a_traiter_ajouter(o.etablissement_id, o.eleve_id, 'option', o.id, 'annee_inconnue');
            RETURN NULL;
        END IF;
        v_offre := ent_offre_assurer_depuis_option(o.etablissement_id, v_an.id, o.code, o.libelle, o.nature);
        SELECT id INTO v_ens FROM ent_eleve_enseignement
         WHERE eleve_id = o.eleve_id AND offre_id = v_offre AND etat IN ('choisi', 'accepte');
        IF v_ens IS NULL THEN
            v_ens := gen_random_uuid();
            INSERT INTO ent_eleve_enseignement (id, etablissement_id, annee_id, eleve_id, offre_id, etat, choisi_le, debut_le, source)
            VALUES (v_ens, o.etablissement_id, v_an.id, o.eleve_id, v_offre, 'accepte', o.date_debut, o.date_debut, 'reprise_option');
        END IF;
        INSERT INTO ent_eleve_enseignement_reprise (source, option_id, etablissement_id, enseignement_id)
        VALUES ('option', o.id, o.etablissement_id, v_ens) ON CONFLICT DO NOTHING;
    END IF;
    -- Une fin saisie avant la fin de l'année clôt l'enseignement (la fin d'année reste « en cours »).
    SELECT * INTO v_an FROM ent_annee_scolaire a
     WHERE a.id = (SELECT annee_id FROM ent_eleve_enseignement WHERE id = v_ens);
    IF o.date_fin IS NOT NULL AND o.date_fin < v_an.date_fin THEN
        v_fin := greatest(o.date_fin, o.date_debut);
        UPDATE ent_eleve_enseignement
           SET etat = 'quitte', quitte_le = greatest(v_fin, debut_le), fin_nature = 'changement_en_cours_annee',
               motif = 'Reprise : fin saisie sur l''option historique'
         WHERE id = v_ens AND etat = 'accepte';
    END IF;
    RETURN v_ens;
END $$;

-- Option par année (propositions N+1) → enseignement suivi ; suit les changements de statut.
CREATE OR REPLACE FUNCTION ent_eleve_enseignement_depuis_option_annee(o ent_eleve_option_annee) RETURNS UUID
LANGUAGE plpgsql AS $$
DECLARE v_an ent_annee_scolaire; v_offre UUID; v_ens UUID; v_etat TEXT;
BEGIN
    SELECT enseignement_id INTO v_ens FROM ent_eleve_enseignement_reprise WHERE source = 'option_annee' AND option_id = o.id;
    IF v_ens IS NULL THEN
        IF btrim(coalesce(o.code, '')) = '' THEN
            PERFORM ent_eleve_enseignement_a_traiter_ajouter(o.etablissement_id, o.eleve_id, 'option_annee', o.id, 'code_vide');
            RETURN NULL;
        END IF;
        SELECT * INTO v_an FROM ent_annee_scolaire a WHERE a.etablissement_id = o.etablissement_id AND a.libelle = o.annee_scolaire;
        IF v_an.id IS NULL THEN
            PERFORM ent_eleve_enseignement_a_traiter_ajouter(o.etablissement_id, o.eleve_id, 'option_annee', o.id, 'annee_inconnue');
            RETURN NULL;
        END IF;
        v_offre := ent_offre_assurer_depuis_option(o.etablissement_id, v_an.id, o.code, o.libelle, o.nature);
        SELECT id INTO v_ens FROM ent_eleve_enseignement
         WHERE eleve_id = o.eleve_id AND offre_id = v_offre AND etat IN ('choisi', 'accepte');
        IF v_ens IS NULL THEN
            v_etat := CASE o.statut WHEN 'retenue' THEN 'accepte' WHEN 'retiree' THEN 'retire' ELSE 'choisi' END;
            v_ens := gen_random_uuid();
            INSERT INTO ent_eleve_enseignement (id, etablissement_id, annee_id, eleve_id, offre_id, etat, choisi_le, debut_le, source)
            VALUES (v_ens, o.etablissement_id, v_an.id, o.eleve_id, v_offre, v_etat,
                    o.created_at::date, CASE WHEN v_etat = 'accepte' THEN v_an.date_debut END, 'reprise_option');
        END IF;
        INSERT INTO ent_eleve_enseignement_reprise (source, option_id, etablissement_id, enseignement_id)
        VALUES ('option_annee', o.id, o.etablissement_id, v_ens) ON CONFLICT DO NOTHING;
        RETURN v_ens;
    END IF;
    -- Mise à jour d'une ligne déjà reprise : on suit le statut.
    SELECT * INTO v_an FROM ent_annee_scolaire a WHERE a.id = (SELECT annee_id FROM ent_eleve_enseignement WHERE id = v_ens);
    IF o.statut = 'retenue' THEN
        UPDATE ent_eleve_enseignement SET etat = 'accepte', debut_le = v_an.date_debut WHERE id = v_ens AND etat = 'choisi';
    ELSIF o.statut = 'retiree' THEN
        UPDATE ent_eleve_enseignement SET etat = 'retire' WHERE id = v_ens AND etat = 'choisi';
        UPDATE ent_eleve_enseignement
           SET etat = 'quitte', quitte_le = greatest(debut_le, least(current_date, v_an.date_fin)),
               fin_nature = 'changement_en_cours_annee', motif = 'Reprise : option retirée sur l''historique'
         WHERE id = v_ens AND etat = 'accepte';
    END IF;
    RETURN v_ens;
END $$;

-- Pont pour les écritures à venir sur les anciennes tables : ne casse jamais l'écriture
-- historique (en cas d'erreur, la ligne est rangée dans `_a_traiter`, rien n'est perdu).
CREATE OR REPLACE FUNCTION ent_eleve_enseignement_pont() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF ent_fusion_en_cours() THEN
        RETURN NULL;
    END IF;
    BEGIN
        IF TG_TABLE_NAME = 'ent_eleve_option' THEN
            PERFORM ent_eleve_enseignement_depuis_option(NEW);
        ELSE
            PERFORM ent_eleve_enseignement_depuis_option_annee(NEW);
        END IF;
    EXCEPTION WHEN OTHERS THEN
        PERFORM ent_eleve_enseignement_a_traiter_ajouter(NEW.etablissement_id, NEW.eleve_id,
            CASE WHEN TG_TABLE_NAME = 'ent_eleve_option' THEN 'option' ELSE 'option_annee' END, NEW.id,
            'erreur_reprise');
    END;
    RETURN NULL;
END $$;
DROP TRIGGER IF EXISTS ent_eleve_option_enseignement_pont ON ent_eleve_option;
CREATE TRIGGER ent_eleve_option_enseignement_pont AFTER INSERT OR UPDATE ON ent_eleve_option
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_enseignement_pont();
DROP TRIGGER IF EXISTS ent_eleve_option_annee_enseignement_pont ON ent_eleve_option_annee;
CREATE TRIGGER ent_eleve_option_annee_enseignement_pont AFTER INSERT OR UPDATE ON ent_eleve_option_annee
    FOR EACH ROW EXECUTE FUNCTION ent_eleve_enseignement_pont();

-- Reprise idempotente ; renvoie le nombre d'options reprises ou rangées à traiter.
CREATE OR REPLACE FUNCTION ent_eleve_enseignement_reprendre() RETURNS BIGINT
LANGUAGE plpgsql AS $$
DECLARE n BIGINT := 0; o ent_eleve_option; a ent_eleve_option_annee;
BEGIN
    FOR o IN SELECT * FROM ent_eleve_option x
             WHERE NOT EXISTS (SELECT 1 FROM ent_eleve_enseignement_reprise r WHERE r.source = 'option' AND r.option_id = x.id)
               AND NOT EXISTS (SELECT 1 FROM ent_eleve_enseignement_a_traiter t WHERE t.source = 'option' AND t.option_id = x.id)
             ORDER BY x.created_at, x.id
    LOOP
        BEGIN
            PERFORM ent_eleve_enseignement_depuis_option(o);
        EXCEPTION WHEN OTHERS THEN
            PERFORM ent_eleve_enseignement_a_traiter_ajouter(o.etablissement_id, o.eleve_id, 'option', o.id, 'erreur_reprise');
        END;
        n := n + 1;
    END LOOP;
    FOR a IN SELECT * FROM ent_eleve_option_annee x
             WHERE NOT EXISTS (SELECT 1 FROM ent_eleve_enseignement_reprise r WHERE r.source = 'option_annee' AND r.option_id = x.id)
               AND NOT EXISTS (SELECT 1 FROM ent_eleve_enseignement_a_traiter t WHERE t.source = 'option_annee' AND t.option_id = x.id)
             ORDER BY x.created_at, x.id
    LOOP
        BEGIN
            PERFORM ent_eleve_enseignement_depuis_option_annee(a);
        EXCEPTION WHEN OTHERS THEN
            PERFORM ent_eleve_enseignement_a_traiter_ajouter(a.etablissement_id, a.eleve_id, 'option_annee', a.id, 'erreur_reprise');
        END;
        n := n + 1;
    END LOOP;
    RETURN n;
END $$;

SELECT ent_eleve_enseignement_reprendre();

-- Comparaison avant / après : toute option héritée est reprise OU rangée à traiter.
CREATE OR REPLACE FUNCTION ent_eleve_enseignement_comparaison(p_etab UUID)
RETURNS TABLE (heritees BIGINT, reprises BIGINT, a_traiter BIGINT) LANGUAGE sql STABLE AS $$
    SELECT
        (SELECT count(*) FROM ent_eleve_option WHERE etablissement_id = p_etab)
          + (SELECT count(*) FROM ent_eleve_option_annee WHERE etablissement_id = p_etab),
        (SELECT count(*) FROM ent_eleve_enseignement_reprise WHERE etablissement_id = p_etab),
        (SELECT count(*) FROM ent_eleve_enseignement_a_traiter WHERE etablissement_id = p_etab AND traite_le IS NULL)
$$;

-- ───────────────────────── Contrôle « choix sans groupe » (STR-68) ─────────────────────────
-- Suivi à la date j : accepté depuis `debut_le`, ou quitté mais pas encore au-delà de `quitte_le`.
CREATE OR REPLACE FUNCTION ent_enseignement_suivi_le(e ent_eleve_enseignement, j DATE) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
    SELECT e.etat IN ('accepte', 'quitte') AND e.debut_le <= j AND (e.quitte_le IS NULL OR j <= e.quitte_le)
$$;

-- Incohérences entre ce que l'élève SUIT et les groupes dont il est MEMBRE, pour les offres
-- organisées par groupe :
--   sans_groupe       : suit l'enseignement, dans aucun groupe de cette offre ;
--   plusieurs_groupes : suit l'enseignement, dans plusieurs groupes de cette offre ;
--   groupe_sans_choix : membre d'un groupe de l'offre sans la suivre.
-- Les offres sans aucun groupe associé sont signalées séparément (`offre_sans_groupe`), une fois.
CREATE OR REPLACE FUNCTION ent_enseignement_incoherences(p_etab UUID, p_annee UUID, p_jour DATE)
RETURNS TABLE (cause TEXT, eleve_id UUID, offre_id UUID, enseignement_id UUID, nb_groupes INTEGER)
LANGUAGE sql STABLE AS $$
    WITH suivis AS (
        SELECT e.id, e.eleve_id, e.offre_id
          FROM ent_eleve_enseignement e
          JOIN ent_offre_enseignement o ON o.id = e.offre_id AND o.etablissement_id = e.etablissement_id
          JOIN ent_eleve el ON el.id = e.eleve_id AND el.etablissement_id = e.etablissement_id
         WHERE e.etablissement_id = p_etab AND e.annee_id = p_annee AND o.public_type = 'groupe'
           AND el.fusionne_dans IS NULL AND ent_enseignement_suivi_le(e, p_jour)
    ), membres AS (
        SELECT a.eleve_id, og.offre_id, count(DISTINCT a.groupe_id)::int AS nb
          FROM ent_affectation a
          JOIN ent_offre_groupe og ON og.groupe_id = a.groupe_id AND og.etablissement_id = a.etablissement_id
         WHERE a.etablissement_id = p_etab AND a.annee_id = p_annee AND a.groupe_id IS NOT NULL
           AND a.date_debut <= p_jour AND (a.date_fin IS NULL OR a.date_fin >= p_jour)
         GROUP BY a.eleve_id, og.offre_id
    )
    SELECT 'sans_groupe'::text, s.eleve_id, s.offre_id, s.id, 0
      FROM suivis s
      JOIN ent_offre_groupe og0 ON og0.offre_id = s.offre_id AND og0.etablissement_id = p_etab
      LEFT JOIN membres m ON m.eleve_id = s.eleve_id AND m.offre_id = s.offre_id
     WHERE m.eleve_id IS NULL
     GROUP BY s.eleve_id, s.offre_id, s.id
    UNION ALL
    SELECT 'plusieurs_groupes', s.eleve_id, s.offre_id, s.id, m.nb
      FROM suivis s JOIN membres m ON m.eleve_id = s.eleve_id AND m.offre_id = s.offre_id WHERE m.nb > 1
    UNION ALL
    SELECT 'groupe_sans_choix', m.eleve_id, m.offre_id, NULL::uuid, m.nb
      FROM membres m
      JOIN ent_eleve el ON el.id = m.eleve_id AND el.etablissement_id = p_etab AND el.fusionne_dans IS NULL
     WHERE NOT EXISTS (SELECT 1 FROM suivis s WHERE s.eleve_id = m.eleve_id AND s.offre_id = m.offre_id)
    UNION ALL
    SELECT 'offre_sans_groupe', NULL::uuid, o.id, NULL::uuid, 0
      FROM ent_offre_enseignement o
     WHERE o.etablissement_id = p_etab AND o.annee_id = p_annee AND o.public_type = 'groupe' AND o.ferme_le IS NULL
       AND NOT EXISTS (SELECT 1 FROM ent_offre_groupe og WHERE og.offre_id = o.id)
       AND EXISTS (SELECT 1 FROM suivis s WHERE s.offre_id = o.id)
$$;
