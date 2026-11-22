-- Socle 20, lot C8 — migration 2407 : visa figé, accès d'inspection borné et journalisé, exports figés, clôture et archive
-- annuelle du cahier de textes.
-- Réf. : 20_CAHIER_DE_TEXTES.md §14, R-C17 à R-C19, R-C28. Numéro libre de la plage 2400–2419 (2407 était réservée à C10,
-- dont la place est reprise par le premier numéro libre de la plage à l'écriture de C10).
-- Additive et rejouable. Aucune colonne JSON, tableau ni BYTEA (D-036).
--
-- Modèle de menace :
--  * un visa est figé (référence immuable des versions vues) : ni UPDATE ni DELETE ni TRUNCATE ;
--  * l'accès d'inspection ne se modifie que par prolongation ou révocation, chacune tracée dans un historique en ajout seul ;
--  * un export est un texte figé dont l'empreinte SHA-256 est recalculée par la base (CHECK) : impossible de la falsifier ;
--  * la clôture du cahier est un journal en ajout seul (clôture / réouverture motivée) : jamais effacé.

-- ── Visa, observations, réponses : figés ──────────────────────────────────────────────────────────────────────────────
CREATE OR REPLACE FUNCTION ent_cdt_sans_truncate() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION '% : TRUNCATE interdit (ajout seul, R-C17)', TG_TABLE_NAME USING ERRCODE = '42501';
END
$$ LANGUAGE plpgsql;

-- Un visa ne se réécrit pas : date, viseur, période et cible sont figés (R-C17). Le texte de l'observation éventuelle vit
-- dans `ent_cdt_observation`, jamais modifiable non plus.
CREATE OR REPLACE FUNCTION ent_cdt_visa_fige() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'ent_cdt_visa : un visa est figé (aucun UPDATE ni DELETE ; R-C17)' USING ERRCODE = '23514';
END
$$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS ent_cdt_visa_fige_trg ON ent_cdt_visa;
CREATE TRIGGER ent_cdt_visa_fige_trg BEFORE UPDATE OR DELETE ON ent_cdt_visa
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_visa_fige();
DROP TRIGGER IF EXISTS ent_cdt_observation_ajout_seul ON ent_cdt_observation;
CREATE TRIGGER ent_cdt_observation_ajout_seul BEFORE UPDATE OR DELETE ON ent_cdt_observation
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_ajout_seul();
DROP TRIGGER IF EXISTS ent_cdt_observation_reponse_ajout_seul ON ent_cdt_observation_reponse;
CREATE TRIGGER ent_cdt_observation_reponse_ajout_seul BEFORE UPDATE OR DELETE ON ent_cdt_observation_reponse
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_ajout_seul();

-- Le visa d'une cible « cours » ou « agent » garde son instantané de ce qui a été vu : nombre d'éléments figés.
ALTER TABLE ent_cdt_visa ADD COLUMN IF NOT EXISTS nb_elements INTEGER;

-- ── Accès d'inspection ───────────────────────────────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS ent_cdt_acces_inspection (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    beneficiaire_user_id UUID NOT NULL REFERENCES ent_user (id),
    -- Professeur concerné (rendez-vous de carrière) : informé à l'octroi, il lit le journal de consultation.
    agent_cible_id UUID,
    du_cahier DATE NOT NULL,
    au_cahier DATE NOT NULL,
    valable_du TIMESTAMPTZ NOT NULL,
    valable_au TIMESTAMPTZ NOT NULL,
    motif TEXT NOT NULL CHECK (motif IN ('rdv_carriere', 'inspection', 'autre')),
    accorde_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    revoque_le TIMESTAMPTZ,
    revoque_par_user_id UUID REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_acces_inspection_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_acces_inspection_cahier_chk CHECK (du_cahier <= au_cahier AND au_cahier - du_cahier <= 400),
    -- Durée bornée : 90 jours au plus, prolongations comprises (défaut 30 jours côté application).
    CONSTRAINT ent_cdt_acces_inspection_duree_chk CHECK (valable_au > valable_du AND valable_au - valable_du <= interval '90 days'),
    CONSTRAINT ent_cdt_acces_inspection_revoque_chk CHECK ((revoque_le IS NULL) = (revoque_par_user_id IS NULL)),
    -- M1 : nul ne s'octroie à lui-même un accès d'inspection (un pouvoir = une action).
    CONSTRAINT ent_cdt_acces_inspection_pas_soi_chk CHECK (beneficiaire_user_id <> accorde_par_user_id),
    CONSTRAINT ent_cdt_acces_inspection_agent_fk FOREIGN KEY (etablissement_id, agent_cible_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_cdt_acces_inspection_benef_idx ON ent_cdt_acces_inspection (etablissement_id, beneficiaire_user_id);

CREATE TABLE IF NOT EXISTS ent_cdt_acces_inspection_cours (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    acces_id UUID NOT NULL,
    cours_id UUID NOT NULL,
    PRIMARY KEY (acces_id, cours_id),
    CONSTRAINT ent_cdt_acces_inspection_cours_acces_fk FOREIGN KEY (etablissement_id, acces_id)
        REFERENCES ent_cdt_acces_inspection (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_acces_inspection_cours_cours_fk FOREIGN KEY (cours_id, etablissement_id)
        REFERENCES ent_cours (id, etablissement_id) ON DELETE RESTRICT
);
DROP TRIGGER IF EXISTS ent_cdt_acces_inspection_cours_ajout_seul ON ent_cdt_acces_inspection_cours;
CREATE TRIGGER ent_cdt_acces_inspection_cours_ajout_seul BEFORE UPDATE OR DELETE ON ent_cdt_acces_inspection_cours
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_ajout_seul();

-- Seules la fin de validité (jamais raccourcie par UPDATE : une révocation passe par `revoque_le`) et la révocation changent.
CREATE OR REPLACE FUNCTION ent_cdt_acces_inspection_garde() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_cdt_acces_inspection : suppression interdite (révoquer)' USING ERRCODE = '23514';
    END IF;
    IF NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.beneficiaire_user_id IS DISTINCT FROM OLD.beneficiaire_user_id
       OR NEW.agent_cible_id IS DISTINCT FROM OLD.agent_cible_id
       OR NEW.du_cahier IS DISTINCT FROM OLD.du_cahier OR NEW.au_cahier IS DISTINCT FROM OLD.au_cahier
       OR NEW.valable_du IS DISTINCT FROM OLD.valable_du OR NEW.motif IS DISTINCT FROM OLD.motif
       OR NEW.accorde_par_user_id IS DISTINCT FROM OLD.accorde_par_user_id
       OR NEW.cree_le IS DISTINCT FROM OLD.cree_le THEN
        RAISE EXCEPTION 'ent_cdt_acces_inspection : seuls la fin de validité et la révocation changent' USING ERRCODE = '23514';
    END IF;
    IF NEW.valable_au < OLD.valable_au THEN
        RAISE EXCEPTION 'ent_cdt_acces_inspection : la validité ne raccourcit que par révocation' USING ERRCODE = '23514';
    END IF;
    IF OLD.revoque_le IS NOT NULL AND (NEW.revoque_le IS DISTINCT FROM OLD.revoque_le
                                       OR NEW.revoque_par_user_id IS DISTINCT FROM OLD.revoque_par_user_id
                                       OR NEW.valable_au IS DISTINCT FROM OLD.valable_au) THEN
        RAISE EXCEPTION 'ent_cdt_acces_inspection : un accès révoqué ne se prolonge pas (en accorder un autre)' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS ent_cdt_acces_inspection_garde_trg ON ent_cdt_acces_inspection;
CREATE TRIGGER ent_cdt_acces_inspection_garde_trg BEFORE UPDATE OR DELETE ON ent_cdt_acces_inspection
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_acces_inspection_garde();

CREATE TABLE IF NOT EXISTS ent_cdt_acces_inspection_historique (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    acces_id UUID NOT NULL,
    action TEXT NOT NULL CHECK (action IN ('accorde', 'prolonge', 'revoque')),
    valable_au TIMESTAMPTZ,
    par_user_id UUID NOT NULL REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_acces_inspection_hist_fk FOREIGN KEY (etablissement_id, acces_id)
        REFERENCES ent_cdt_acces_inspection (etablissement_id, id) ON DELETE RESTRICT
);
DROP TRIGGER IF EXISTS ent_cdt_acces_inspection_historique_ajout_seul ON ent_cdt_acces_inspection_historique;
CREATE TRIGGER ent_cdt_acces_inspection_historique_ajout_seul BEFORE UPDATE OR DELETE ON ent_cdt_acces_inspection_historique
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_ajout_seul();

-- Le journal de consultation (2410, ajout seul) saura de quel accès et de quel cours vient chaque lecture d'inspection.
ALTER TABLE ent_cdt_consultation ADD COLUMN IF NOT EXISTS acces_id UUID;
ALTER TABLE ent_cdt_consultation ADD COLUMN IF NOT EXISTS cours_id UUID;
CREATE INDEX IF NOT EXISTS ent_cdt_consultation_acces_idx ON ent_cdt_consultation (etablissement_id, acces_id) WHERE acces_id IS NOT NULL;

-- ── Exports figés ────────────────────────────────────────────────────────────────────────────────────────────────────
-- Un export est un texte daté, sans nom d'élève, dont l'empreinte est contrôlée par la base elle-même.
CREATE TABLE IF NOT EXISTS ent_cdt_export (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nature TEXT NOT NULL CHECK (nature IN ('litige', 'inspection', 'archive', 'professeur', 'autre')),
    cours_id UUID,
    agent_id UUID,
    annee_id UUID,
    du DATE NOT NULL,
    au DATE NOT NULL,
    titre TEXT NOT NULL CHECK (length(btrim(titre)) BETWEEN 1 AND 200),
    contenu TEXT NOT NULL CHECK (length(contenu) <= 4000000),
    empreinte CHAR(64) NOT NULL,
    nb_entrees INTEGER NOT NULL CHECK (nb_entrees >= 0),
    nb_travaux INTEGER NOT NULL CHECK (nb_travaux >= 0),
    motif TEXT CHECK (motif IS NULL OR length(motif) <= 500),
    produit_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_cdt_export_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_export_periode_chk CHECK (du <= au),
    -- L'empreinte est recalculée par la base : un export ne peut pas porter l'empreinte d'un autre contenu.
    CONSTRAINT ent_cdt_export_empreinte_chk CHECK (empreinte = encode(sha256(convert_to(contenu, 'UTF8')), 'hex')),
    CONSTRAINT ent_cdt_export_cours_fk FOREIGN KEY (cours_id, etablissement_id)
        REFERENCES ent_cours (id, etablissement_id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_export_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_cdt_export_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_cdt_export_etab_idx ON ent_cdt_export (etablissement_id, cree_le DESC);
-- M2 : l'export personnel d'un professeur est conservé une seule fois par (auteur, année, empreinte).
CREATE UNIQUE INDEX IF NOT EXISTS ent_cdt_export_prof_uq
    ON ent_cdt_export (etablissement_id, produit_par_user_id, annee_id, empreinte) WHERE nature = 'professeur';
DROP TRIGGER IF EXISTS ent_cdt_export_ajout_seul ON ent_cdt_export;
CREATE TRIGGER ent_cdt_export_ajout_seul BEFORE UPDATE OR DELETE ON ent_cdt_export
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_ajout_seul();

-- ── Archive annuelle : l'export remplace l'objet du magasin tant que le PDF/A du socle 7 n'existe pas ───────────────────
ALTER TABLE ent_cdt_archive ADD COLUMN IF NOT EXISTS export_id UUID;
ALTER TABLE ent_cdt_archive ALTER COLUMN objet_id DROP NOT NULL;
ALTER TABLE ent_cdt_archive DROP CONSTRAINT IF EXISTS ent_cdt_archive_source_chk;
ALTER TABLE ent_cdt_archive ADD CONSTRAINT ent_cdt_archive_source_chk CHECK (objet_id IS NOT NULL OR export_id IS NOT NULL);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_cdt_archive_export_fk') THEN
        ALTER TABLE ent_cdt_archive ADD CONSTRAINT ent_cdt_archive_export_fk
            FOREIGN KEY (etablissement_id, export_id) REFERENCES ent_cdt_export (etablissement_id, id) ON DELETE RESTRICT;
    END IF;
END
$$;
-- La garde de 2411 est remplacée (2411 intacte) : l'export fait désormais partie de ce qui est figé.
CREATE OR REPLACE FUNCTION ent_cdt_archive_garde() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_cdt_archive : suppression interdite (decision)' USING ERRCODE = '23514';
    END IF;
    IF NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id OR NEW.annee_id IS DISTINCT FROM OLD.annee_id
       OR NEW.cours_id IS DISTINCT FROM OLD.cours_id OR NEW.objet_id IS DISTINCT FROM OLD.objet_id
       OR NEW.export_id IS DISTINCT FROM OLD.export_id
       OR NEW.empreinte IS DISTINCT FROM OLD.empreinte OR NEW.produite_le IS DISTINCT FROM OLD.produite_le
       OR NEW.version IS DISTINCT FROM OLD.version OR NEW.echeance_archive IS DISTINCT FROM OLD.echeance_archive THEN
        RAISE EXCEPTION 'ent_cdt_archive : archive immuable (seule la décision change)' USING ERRCODE = '23514';
    END IF;
    IF OLD.decision <> 'a_decider' AND (NEW.decision IS DISTINCT FROM OLD.decision
                                        OR NEW.decide_par_user_id IS DISTINCT FROM OLD.decide_par_user_id
                                        OR NEW.decide_le IS DISTINCT FROM OLD.decide_le) THEN
        RAISE EXCEPTION 'ent_cdt_archive : décision déjà prise (auteur et date figés)' USING ERRCODE = '23514';
    END IF;
    IF OLD.decision = 'a_decider' AND NEW.decision <> 'a_decider' AND NEW.decide_par_user_id IS NULL THEN
        RAISE EXCEPTION 'ent_cdt_archive : une décision porte son auteur' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

-- F2 : l'empreinte d'une archive est celle de l'export qu'elle désigne (même cours, même année, même contenu figé) : la
-- base refuse une archive dont le lien d'empreinte ne tient pas.
CREATE OR REPLACE FUNCTION ent_cdt_archive_lien() RETURNS trigger AS $$
BEGIN
    IF NEW.export_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM ent_cdt_export x
         WHERE x.etablissement_id = NEW.etablissement_id AND x.id = NEW.export_id AND x.nature = 'archive'
           AND x.empreinte = NEW.empreinte AND x.cours_id IS NOT DISTINCT FROM NEW.cours_id
           AND x.annee_id IS NOT DISTINCT FROM NEW.annee_id) THEN
        RAISE EXCEPTION 'ent_cdt_archive : l''empreinte, le cours et l''année doivent être ceux de l''export' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS ent_cdt_archive_lien_trg ON ent_cdt_archive;
CREATE TRIGGER ent_cdt_archive_lien_trg BEFORE INSERT ON ent_cdt_archive
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_archive_lien();

-- ── Clôture du cahier : journal en ajout seul ────────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS ent_cdt_cloture (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    annee_id UUID NOT NULL,
    -- Réouverture en deux temps (M1, double validation) : `reouverture_demandee` (première personne de la direction), puis
    -- `reouverture` (seconde personne, jamais le demandeur). Seules `cloture` et `reouverture` changent l'état figé.
    action TEXT NOT NULL CHECK (action IN ('cloture', 'reouverture_demandee', 'reouverture')),
    -- Motif de 20 caractères au moins (R-C19, CC-63) ; la réouverture reprend celui de la demande qu'elle valide.
    motif TEXT CHECK (motif IS NULL OR length(btrim(motif)) BETWEEN 20 AND 500),
    nb_archives INTEGER NOT NULL DEFAULT 0 CHECK (nb_archives >= 0),
    -- `par_user_id` : qui clôt, qui demande la réouverture, ou qui la valide.
    par_user_id UUID NOT NULL REFERENCES ent_user (id),
    -- Réouverture : le demandeur de la demande validée (la base refuse qu'il valide lui-même).
    demandeur_user_id UUID REFERENCES ent_user (id),
    -- Horloge réelle : l'ordre du journal suit l'ordre des validations, pas l'heure de début des transactions.
    cree_le TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    CONSTRAINT ent_cdt_cloture_motif_chk CHECK (action = 'cloture' OR motif IS NOT NULL),
    CONSTRAINT ent_cdt_cloture_double_chk CHECK (action <> 'reouverture' OR (demandeur_user_id IS NOT NULL AND demandeur_user_id <> par_user_id)),
    CONSTRAINT ent_cdt_cloture_annee_fk FOREIGN KEY (annee_id, etablissement_id)
        REFERENCES ent_annee_scolaire (id, etablissement_id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_cdt_cloture_annee_idx ON ent_cdt_cloture (etablissement_id, annee_id, cree_le DESC);
DROP TRIGGER IF EXISTS ent_cdt_cloture_ajout_seul ON ent_cdt_cloture;
CREATE TRIGGER ent_cdt_cloture_ajout_seul BEFORE UPDATE OR DELETE ON ent_cdt_cloture
    FOR EACH ROW EXECUTE FUNCTION ent_cdt_ajout_seul();

-- Le cahier d'une année est figé quand la dernière ligne du journal est une clôture, ou quand l'année est close (socle 5).
CREATE OR REPLACE FUNCTION ent_cdt_cahier_fige(p_etab UUID, p_annee UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE AS $$
    SELECT COALESCE((SELECT c.action = 'cloture' FROM ent_cdt_cloture c
                      WHERE c.etablissement_id = p_etab AND c.annee_id = p_annee AND c.action IN ('cloture', 'reouverture')
                      ORDER BY c.cree_le DESC, c.id DESC LIMIT 1), false)
$$;

-- ── E1 / M5 : une année figée l'est EN BASE, quel que soit le chemin d'écriture ──────────────────────────────────────────
-- Entrées, travaux, contributions et leurs versions (et les notions d'une entrée) ne s'insèrent ni ne se modifient quand le
-- cahier de l'année est figé : seule la réouverture (journal `ent_cdt_cloture`) désarme la garde. La garde prend un verrou
-- partagé sur la ligne de l'année ; la clôture et la réouverture la prennent en exclusif (`FOR UPDATE`) : une écriture et
-- une clôture ne se croisent donc jamais (l'écriture attend la fin de la clôture et la voit, la clôture attend les écritures
-- en cours et les archive). Lève 23514.
CREATE OR REPLACE FUNCTION ent_cdt_ecriture_gardee() RETURNS trigger AS $$
DECLARE
    v_etab UUID := NEW.etablissement_id;
    v_annees UUID[];
    v_annee UUID;
    v_fige BOOLEAN;
BEGIN
    IF TG_TABLE_NAME IN ('ent_cdt_entree', 'ent_cdt_travail') THEN
        v_annees := ARRAY[NEW.annee_id];
        IF TG_OP = 'UPDATE' AND OLD.annee_id IS DISTINCT FROM NEW.annee_id THEN
            v_annees := v_annees || OLD.annee_id;
        END IF;
    ELSIF TG_TABLE_NAME IN ('ent_cdt_entree_version', 'ent_cdt_entree_notion', 'ent_cdt_contribution') THEN
        SELECT ARRAY[e.annee_id] INTO v_annees FROM ent_cdt_entree e
         WHERE e.etablissement_id = v_etab AND e.id = NEW.entree_id;
    ELSIF TG_TABLE_NAME = 'ent_cdt_travail_version' THEN
        SELECT ARRAY[t.annee_id] INTO v_annees FROM ent_cdt_travail t
         WHERE t.etablissement_id = v_etab AND t.id = NEW.travail_id;
    ELSIF TG_TABLE_NAME = 'ent_cdt_contribution_version' THEN
        SELECT ARRAY[e.annee_id] INTO v_annees FROM ent_cdt_contribution c
          JOIN ent_cdt_entree e ON e.etablissement_id = c.etablissement_id AND e.id = c.entree_id
         WHERE c.etablissement_id = v_etab AND c.id = NEW.contribution_id;
    END IF;
    FOREACH v_annee IN ARRAY COALESCE(v_annees, ARRAY[]::uuid[])
    LOOP
        PERFORM 1 FROM ent_annee_scolaire a WHERE a.etablissement_id = v_etab AND a.id = v_annee FOR SHARE;
        SELECT COALESCE((SELECT c.action = 'cloture' FROM ent_cdt_cloture c
                          WHERE c.etablissement_id = v_etab AND c.annee_id = v_annee AND c.action IN ('cloture', 'reouverture')
                          ORDER BY c.cree_le DESC, c.id DESC LIMIT 1), false) INTO v_fige;
        IF v_fige THEN
            RAISE EXCEPTION 'cahier de textes : l''année est figée (clôturée), aucune écriture sans réouverture tracée (R-C28)'
                USING ERRCODE = '23514';
        END IF;
    END LOOP;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;
DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_cdt_entree', 'ent_cdt_entree_version', 'ent_cdt_entree_notion', 'ent_cdt_travail',
                             'ent_cdt_travail_version', 'ent_cdt_contribution', 'ent_cdt_contribution_version']
    LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', t || '_annee_figee', t);
        EXECUTE format('CREATE TRIGGER %I BEFORE INSERT OR UPDATE ON %I FOR EACH ROW EXECUTE FUNCTION ent_cdt_ecriture_gardee()',
                       t || '_annee_figee', t);
    END LOOP;
END
$$;

-- ── Gardes TRUNCATE (ajout seul) et RLS ──────────────────────────────────────────────────────────────────────────────
DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_cdt_visa', 'ent_cdt_visa_element', 'ent_cdt_observation', 'ent_cdt_observation_reponse',
                             'ent_cdt_consultation', 'ent_cdt_acces_inspection_cours', 'ent_cdt_acces_inspection_historique',
                             'ent_cdt_export', 'ent_cdt_cloture', 'ent_cdt_archive',
                             -- F1 : l'accès d'inspection n'est pas en ajout seul (prolongation, révocation) mais ne se vide pas.
                             'ent_cdt_acces_inspection']
    LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', t || '_sans_truncate', t);
        EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT '
                       'EXECUTE FUNCTION ent_cdt_sans_truncate()', t || '_sans_truncate', t);
    END LOOP;
END
$$;

SELECT ent_rls_poser(ARRAY['ent_cdt_acces_inspection', 'ent_cdt_acces_inspection_cours', 'ent_cdt_acces_inspection_historique',
                           'ent_cdt_export', 'ent_cdt_cloture']);
