-- Socle 21, lot V8 « fin de période » (`docs/socles/21_EVALUATIONS.md` §9, §17.8, §17.9, EVA-178 à EVA-193 ;
-- livraison : `docs/socles/21_EVALUATIONS_V8_LIVRAISON.md`). Plage d'extension 701–719 du socle 21 ; posée en FIN du
-- tableau de `migrate.rs` (règle 38). Additive et rejouable ; aucune migration versée n'est modifiée (règle 25).
--
-- La machine d'états de période, les rattrapages autorisés et l'instantané d'arrêt existent depuis 463 (V1) ; les
-- demandes de correction depuis 466. Ce lot y ajoute :
--   * les appréciations retenues par chaque version de l'instantané (« appréciations à cet instant », §9.1) ;
--   * ce qui manquait aux demandes de correction : la proposition, la version vue, la décision motivée et son effet
--     sur les moyennes (EVA-189, EVA-190), avec un garde « une décision, une fois » ;
--   * le journal des réouvertures motivées, classe par classe, avec leur durée (EVA-185) ;
--   * le journal des relances et des rappels J-7 / J-2 aux professeurs (EVA-179, EVA-239), une fois par échéance.
-- Aucune colonne JSON (D-036). Fonctions avec `search_path` fixé, relations qualifiées `public.`.

-- 1. Appréciations retenues par une version de l'instantané ---------------------------------------------------------------
-- Une ligne par appréciation VALIDÉE à l'instant de la version (sa version à elle) : l'empreinte de l'instantané en dépend,
-- et le conseil voit ce qui a changé depuis. Immuable, comme l'instantané.
CREATE TABLE IF NOT EXISTS eva_instantane_appreciation (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    instantane_id UUID NOT NULL,
    appreciation_id UUID NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_instantane_appreciation_inst_fk FOREIGN KEY (instantane_id, etablissement_id)
        REFERENCES public.eva_instantane (id, etablissement_id),
    CONSTRAINT eva_instantane_appreciation_app_fk FOREIGN KEY (appreciation_id, etablissement_id)
        REFERENCES public.eva_appreciation (id, etablissement_id),
    CONSTRAINT eva_instantane_appreciation_uq UNIQUE (instantane_id, appreciation_id)
);
DROP TRIGGER IF EXISTS eva_instantane_appreciation_immuable ON eva_instantane_appreciation;
CREATE TRIGGER eva_instantane_appreciation_immuable BEFORE UPDATE OR DELETE ON eva_instantane_appreciation
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_instantane_appreciation_vidage ON eva_instantane_appreciation;
CREATE TRIGGER eva_instantane_appreciation_vidage BEFORE TRUNCATE ON eva_instantane_appreciation
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();
CREATE INDEX IF NOT EXISTS eva_instantane_ligne_inst_idx ON eva_instantane_ligne (etablissement_id, instantane_id);

-- 2. Demandes de correction après l'arrêt (EVA-189, EVA-190) ----------------------------------------------------------------
-- Colonnes nullables : une ligne antérieure (466) n'en a pas. La proposition est celle du professeur ; la décision est
-- celle de la direction, motivée si elle refuse ; l'effet (millièmes) est celui que la direction a vu en décidant.
ALTER TABLE eva_correction_demande ADD COLUMN IF NOT EXISTS version_vue INTEGER CHECK (version_vue IS NULL OR version_vue >= 1);
ALTER TABLE eva_correction_demande ADD COLUMN IF NOT EXISTS statut_propose TEXT
    CHECK (statut_propose IS NULL OR statut_propose IN
        ('note', 'absent', 'a_rattraper', 'dispense', 'non_rendu', 'non_note'));
ALTER TABLE eva_correction_demande ADD COLUMN IF NOT EXISTS valeur_proposee NUMERIC(7, 3)
    CHECK (valeur_proposee IS NULL OR valeur_proposee >= 0);
ALTER TABLE eva_correction_demande ADD COLUMN IF NOT EXISTS decision_motif TEXT
    CHECK (decision_motif IS NULL OR char_length(btrim(decision_motif)) BETWEEN 3 AND 1000);
ALTER TABLE eva_correction_demande ADD COLUMN IF NOT EXISTS matiere_avant_m BIGINT;
ALTER TABLE eva_correction_demande ADD COLUMN IF NOT EXISTS matiere_apres_m BIGINT;
ALTER TABLE eva_correction_demande ADD COLUMN IF NOT EXISTS generale_avant_m BIGINT;
ALTER TABLE eva_correction_demande ADD COLUMN IF NOT EXISTS generale_apres_m BIGINT;
ALTER TABLE eva_correction_demande ADD COLUMN IF NOT EXISTS version_ecrite INTEGER CHECK (version_ecrite IS NULL OR version_ecrite >= 1);
ALTER TABLE eva_correction_demande ADD COLUMN IF NOT EXISTS retiree_le TIMESTAMPTZ;
-- Une demande en attente par résultat (une seconde attend la décision de la première).
CREATE UNIQUE INDEX IF NOT EXISTS eva_correction_demande_ouverte_uq
    ON eva_correction_demande (etablissement_id, resultat_id) WHERE etat = 'en_attente' AND resultat_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS eva_correction_demande_etat_idx ON eva_correction_demande (etablissement_id, etat, cree_le);

-- Garde : ni suppression, ni réécriture de la cible, de l'auteur, du motif, de la proposition ou de la date ; une seule
-- sortie de l'état « en attente » (décision avec auteur et date, ou retrait avec date et sans décideur).
CREATE OR REPLACE FUNCTION eva_correction_demande_garde() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'eva_correction_demande : une demande ne se supprime pas' USING ERRCODE = '23514';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.resultat_id IS DISTINCT FROM OLD.resultat_id OR NEW.appreciation_id IS DISTINCT FROM OLD.appreciation_id
       OR NEW.demandeur IS DISTINCT FROM OLD.demandeur OR NEW.motif IS DISTINCT FROM OLD.motif
       OR NEW.cree_le IS DISTINCT FROM OLD.cree_le OR NEW.version_vue IS DISTINCT FROM OLD.version_vue
       OR NEW.statut_propose IS DISTINCT FROM OLD.statut_propose
       OR NEW.valeur_proposee IS DISTINCT FROM OLD.valeur_proposee THEN
        RAISE EXCEPTION 'eva_correction_demande : la demande ne se réécrit pas' USING ERRCODE = '23514';
    END IF;
    IF OLD.etat <> 'en_attente' THEN
        RAISE EXCEPTION 'eva_correction_demande : demande déjà traitée' USING ERRCODE = '23514';
    END IF;
    IF NEW.etat = 'en_attente' THEN
        RAISE EXCEPTION 'eva_correction_demande : une mise à jour sort de l''état « en attente »' USING ERRCODE = '23514';
    END IF;
    IF NEW.etat = 'retiree' AND (NEW.retiree_le IS NULL OR NEW.decide_par IS NOT NULL) THEN
        RAISE EXCEPTION 'eva_correction_demande : un retrait porte sa date et aucun décideur' USING ERRCODE = '23514';
    END IF;
    IF NEW.etat = 'refusee' AND NEW.decision_motif IS NULL THEN
        RAISE EXCEPTION 'eva_correction_demande : un refus est motivé' USING ERRCODE = '23514';
    END IF;
    IF NEW.etat = 'acceptee' AND NEW.version_ecrite IS NULL THEN
        RAISE EXCEPTION 'eva_correction_demande : une acceptation porte la version écrite' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS eva_correction_demande_garde_trg ON eva_correction_demande;
CREATE TRIGGER eva_correction_demande_garde_trg BEFORE UPDATE OR DELETE ON eva_correction_demande
    FOR EACH ROW EXECUTE FUNCTION eva_correction_demande_garde();
DROP TRIGGER IF EXISTS eva_correction_demande_vidage_trg ON eva_correction_demande;
CREATE TRIGGER eva_correction_demande_vidage_trg BEFORE TRUNCATE ON eva_correction_demande
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

-- 3. Réouvertures motivées (EVA-185) ---------------------------------------------------------------------------------------
-- Une ligne par classe rouverte ; un même geste (plusieurs classes) partage son `lot_id`. La durée est une date de fin
-- (jour de l'établissement, inclus) ; au lendemain, le passage quotidien referme la classe (`refermee_le`, une fois).
CREATE TABLE IF NOT EXISTS eva_reouverture (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    lot_id UUID NOT NULL,
    periode_id UUID NOT NULL,
    classe_id UUID NOT NULL,
    etat_avant TEXT NOT NULL CHECK (etat_avant IN ('notes_arretees', 'appreciations_closes', 'conseil_tenu', 'close')),
    etat_apres TEXT NOT NULL CHECK (etat_apres IN ('ouverte', 'notes_arretees')),
    motif TEXT NOT NULL CHECK (char_length(btrim(motif)) BETWEEN 10 AND 500),
    jusqu_au DATE NOT NULL,
    par UUID NOT NULL,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    refermee_le TIMESTAMPTZ,
    -- `auto` : fin de la durée (passage quotidien) ; `geste` : la direction a arrêté de nouveau avant le terme.
    refermee_origine TEXT CHECK (refermee_origine IS NULL OR refermee_origine IN ('auto', 'geste')),
    CONSTRAINT eva_reouverture_etat_fk FOREIGN KEY (etablissement_id, periode_id, classe_id)
        REFERENCES public.eva_periode_etat (etablissement_id, periode_id, classe_id),
    CONSTRAINT eva_reouverture_fermeture_chk CHECK ((refermee_le IS NULL) = (refermee_origine IS NULL))
);
CREATE INDEX IF NOT EXISTS eva_reouverture_classe_idx ON eva_reouverture (etablissement_id, periode_id, classe_id, le);
-- Une réouverture en cours au plus par (période, classe).
CREATE UNIQUE INDEX IF NOT EXISTS eva_reouverture_en_cours_uq
    ON eva_reouverture (etablissement_id, periode_id, classe_id) WHERE refermee_le IS NULL;

CREATE OR REPLACE FUNCTION eva_reouverture_garde() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'eva_reouverture : journal, aucune suppression' USING ERRCODE = '23514';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.lot_id IS DISTINCT FROM OLD.lot_id OR NEW.periode_id IS DISTINCT FROM OLD.periode_id
       OR NEW.classe_id IS DISTINCT FROM OLD.classe_id OR NEW.etat_avant IS DISTINCT FROM OLD.etat_avant
       OR NEW.etat_apres IS DISTINCT FROM OLD.etat_apres OR NEW.motif IS DISTINCT FROM OLD.motif
       OR NEW.jusqu_au IS DISTINCT FROM OLD.jusqu_au OR NEW.par IS DISTINCT FROM OLD.par OR NEW.le IS DISTINCT FROM OLD.le THEN
        RAISE EXCEPTION 'eva_reouverture : seule la fermeture se pose' USING ERRCODE = '23514';
    END IF;
    IF OLD.refermee_le IS NOT NULL THEN
        RAISE EXCEPTION 'eva_reouverture : réouverture déjà refermée' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS eva_reouverture_garde_trg ON eva_reouverture;
CREATE TRIGGER eva_reouverture_garde_trg BEFORE UPDATE OR DELETE ON eva_reouverture
    FOR EACH ROW EXECUTE FUNCTION eva_reouverture_garde();
DROP TRIGGER IF EXISTS eva_reouverture_vidage_trg ON eva_reouverture;
CREATE TRIGGER eva_reouverture_vidage_trg BEFORE TRUNCATE ON eva_reouverture
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

-- 4. Relances et rappels aux professeurs (EVA-179, EVA-239, §9.1 « J-7 et J-2 ») ------------------------------------------
-- Ajout seul. Les nombres décrivent ce qui manquait au professeur pour CETTE classe à l'instant de la relance ; aucun nom
-- d'élève. Un rappel automatique part une fois par (période, classe, professeur, échéance) ; une relance manuelle une
-- fois par jour.
CREATE TABLE IF NOT EXISTS eva_relance (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    periode_id UUID NOT NULL,
    classe_id UUID NOT NULL,
    prof_user_id UUID NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN ('manuelle', 'j7', 'j2')),
    jour DATE NOT NULL,
    -- Date d'arrêt visée par un rappel automatique.
    echeance DATE,
    non_publiees INTEGER NOT NULL CHECK (non_publiees >= 0),
    cases_vides INTEGER NOT NULL CHECK (cases_vides >= 0),
    cours_sans_evaluation INTEGER NOT NULL CHECK (cours_sans_evaluation >= 0),
    message TEXT CHECK (message IS NULL OR char_length(btrim(message)) BETWEEN 3 AND 1000),
    par UUID,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_relance_classe_fk FOREIGN KEY (classe_id, etablissement_id)
        REFERENCES public.ent_classe (id, etablissement_id),
    CONSTRAINT eva_relance_periode_fk FOREIGN KEY (periode_id, etablissement_id)
        REFERENCES public.ent_periode (id, etablissement_id),
    CONSTRAINT eva_relance_nature_chk CHECK ((nature = 'manuelle') = (par IS NOT NULL)
                                             AND (nature = 'manuelle' OR echeance IS NOT NULL))
);
CREATE UNIQUE INDEX IF NOT EXISTS eva_relance_jour_uq
    ON eva_relance (etablissement_id, periode_id, classe_id, prof_user_id, nature, jour) WHERE nature = 'manuelle';
CREATE UNIQUE INDEX IF NOT EXISTS eva_relance_echeance_uq
    ON eva_relance (etablissement_id, periode_id, classe_id, prof_user_id, nature, echeance) WHERE nature <> 'manuelle';
DROP TRIGGER IF EXISTS eva_relance_ajout_seul ON eva_relance;
CREATE TRIGGER eva_relance_ajout_seul BEFORE UPDATE OR DELETE ON eva_relance
    FOR EACH ROW EXECUTE FUNCTION ent_trg_eva_ajout_seul();
DROP TRIGGER IF EXISTS eva_relance_vidage ON eva_relance;
CREATE TRIGGER eva_relance_vidage BEFORE TRUNCATE ON eva_relance
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_eva_ajout_seul();

-- 5. Élèves d'une classe pour une période --------------------------------------------------------------------------------
-- Ceux dont la DERNIÈRE affectation de classe qui recoupe la période est cette classe (même règle que le moteur pour les
-- coefficients : un élève parti en cours de période reste dans la classe qu'il a quittée en dernier).
CREATE OR REPLACE FUNCTION eva_eleves_classe_periode(p_etab uuid, p_periode uuid, p_classe uuid)
RETURNS TABLE (eleve_id uuid)
LANGUAGE sql STABLE
SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT q.eleve_id FROM (
        SELECT DISTINCT ON (a.eleve_id) a.eleve_id, a.classe_id
          FROM public.ent_affectation a
          JOIN public.ent_periode p ON p.id = p_periode AND p.etablissement_id = a.etablissement_id
         WHERE a.etablissement_id = p_etab AND a.classe_id IS NOT NULL
           AND a.date_debut <= p.date_fin AND (a.date_fin IS NULL OR a.date_fin >= p.date_debut)
           AND a.eleve_id IN (SELECT a2.eleve_id FROM public.ent_affectation a2
                               WHERE a2.etablissement_id = p_etab AND a2.classe_id = p_classe)
         ORDER BY a.eleve_id, a.date_debut DESC
    ) q
     WHERE q.classe_id = p_classe
$$;

SELECT ent_rls_poser(ARRAY['eva_instantane_appreciation', 'eva_reouverture', 'eva_relance']);
