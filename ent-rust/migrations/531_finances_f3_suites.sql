-- 531 — Finances, suites du lot F3 (contre-revue de F3 : D-069, M-A à M-D et faibles 1 à 12).
--
-- Migrations 520 à 523 versées : elles ne changent pas. Cette migration (la fin du tableau, règle 38) :
--   * M-B : l'entrée en vigueur d'une grille ne peut être antérieure **ni égale** à celle d'une version déjà
--     appliquée (le choix suit la date puis le numéro : à date égale, le numéro l'emportait sur la décision) ;
--   * faible 7 : colonne `contributeurs` (ajout seul, en brouillon) : tout auteur d'une modification d'une
--     grille (tarifs ajoutés puis retirés, tranches, hausses) y reste, et un retour en brouillon y verse le
--     proposant : le second regard ne perd plus ses auteurs ; le vote refuse tout contributeur ;
--   * faible 3 : une version `close` (encore applicable jusqu'à la veille de la suivante) se publie ;
--   * faible 9 : l'année d'un tarif versionné ne change pas (celle de sa version) ;
--   * faible 4 : une ligne de l'historique des calculs d'une simulation ne se supprime plus qu'avec sa
--     simulation (clé étrangère `ON DELETE CASCADE` + déclencheur : DELETE direct refusé).
-- Toute fonction ci-dessous porte `SET search_path = pg_catalog, public, pg_temp` et des relations qualifiées
-- (règle 38). Les verrous d'établissement des services (M-C, faible 5) ne demandent aucun objet de base.

ALTER TABLE public.ent_fin_grille_version
    ADD COLUMN IF NOT EXISTS contributeurs UUID[] NOT NULL DEFAULT '{}'::uuid[];

CREATE OR REPLACE FUNCTION ent_trg_fin_grille_version() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    ok BOOLEAN;
    modifiees TEXT[];
    permises TEXT[];
    second BOOLEAN;
BEGIN
    IF TG_OP IN ('DELETE', 'TRUNCATE') THEN
        RAISE EXCEPTION 'ent_fin_grille_version : ni suppression ni vidage (R-F10)'
            USING ERRCODE = 'restrict_violation';
    END IF;
    IF TG_OP = 'INSERT' THEN
        IF NEW.statut <> 'brouillon'
           OR NEW.propose_le IS NOT NULL OR NEW.propose_par IS NOT NULL
           OR NEW.vote_le IS NOT NULL OR NEW.vote_reference IS NOT NULL OR NEW.vote_document_id IS NOT NULL
           OR NEW.vote_par IS NOT NULL OR NEW.en_vigueur_du IS NOT NULL OR NEW.en_vigueur_par IS NOT NULL
           OR NEW.close_le IS NOT NULL OR NEW.publiee_le IS NOT NULL
           OR cardinality(NEW.contributeurs) > 0 THEN
            RAISE EXCEPTION 'grille : une version naît brouillon, sans proposition, vote ni vigueur (R-F10)'
                USING ERRCODE = 'restrict_violation';
        END IF;
        RETURN NEW;
    END IF;
    -- UPDATE : colonnes réellement modifiées
    SELECT COALESCE(array_agg(n.key), ARRAY[]::TEXT[]) INTO modifiees
      FROM jsonb_each(to_jsonb(NEW)) n
     WHERE to_jsonb(OLD) -> n.key IS DISTINCT FROM n.value;
    IF modifiees && ARRAY['id', 'etablissement_id', 'annee_scolaire', 'numero', 'remplace_id', 'cree_par', 'created_at'] THEN
        RAISE EXCEPTION 'ent_fin_grille_version : identité figée (établissement, année, numéro, version remplacée, auteur)'
            USING ERRCODE = 'restrict_violation';
    END IF;
    IF NEW.statut IS DISTINCT FROM OLD.statut THEN
        ok := (OLD.statut, NEW.statut) IN (
            ('brouillon', 'proposee'), ('proposee', 'brouillon'), ('brouillon', 'abandonnee'),
            ('proposee', 'abandonnee'), ('proposee', 'votee'), ('votee', 'en_vigueur'),
            ('votee', 'close'), ('votee', 'supplantee'), ('en_vigueur', 'close'));
        IF NOT ok THEN
            RAISE EXCEPTION 'grille : transition % → % interdite', OLD.statut, NEW.statut
                USING ERRCODE = 'restrict_violation';
        END IF;
        permises := CASE
            WHEN NEW.statut = 'proposee' THEN ARRAY['statut', 'propose_le', 'propose_par']
            WHEN NEW.statut = 'brouillon' THEN ARRAY['statut', 'propose_le', 'propose_par']
            WHEN NEW.statut = 'abandonnee' THEN ARRAY['statut']
            WHEN NEW.statut = 'votee' THEN
                ARRAY['statut', 'vote_le', 'vote_reference', 'vote_document_id', 'vote_par']
            WHEN NEW.statut = 'en_vigueur' THEN ARRAY['statut', 'en_vigueur_du', 'en_vigueur_par']
            ELSE ARRAY['statut', 'close_le'] END;
        IF modifiees && ARRAY['contributeurs'] THEN
            RAISE EXCEPTION 'grille : les contributeurs ne se posent qu''en brouillon (second regard)'
                USING ERRCODE = 'restrict_violation';
        END IF;
        IF NOT (modifiees <@ permises) THEN
            RAISE EXCEPTION 'grille : la transition % → % ne change que % (R-F10)', OLD.statut, NEW.statut, permises
                USING ERRCODE = 'restrict_violation';
        END IF;
        -- ce qui est posé ne se repose pas : colonnes de la transition vides avant (sauf remise à zéro du retour)
        IF NEW.statut <> 'brouillon' THEN
            IF (NEW.statut = 'proposee' AND OLD.propose_le IS NOT NULL)
               OR (NEW.statut = 'votee' AND (OLD.vote_le IS NOT NULL OR OLD.vote_par IS NOT NULL))
               OR (NEW.statut = 'en_vigueur' AND (OLD.en_vigueur_du IS NOT NULL OR OLD.en_vigueur_par IS NOT NULL))
               OR (NEW.statut IN ('close', 'supplantee') AND OLD.close_le IS NOT NULL) THEN
                RAISE EXCEPTION 'grille : valeur déjà posée, figée (R-F10)' USING ERRCODE = 'restrict_violation';
            END IF;
        END IF;
        -- 531 (faible 7) : le retour en brouillon efface la proposition mais **garde son auteur** parmi les
        -- contributeurs (il ne votera pas la grille qu'il a proposée, règle 3)
        IF NEW.statut = 'brouillon' AND OLD.propose_par IS NOT NULL
           AND NOT (OLD.propose_par = ANY (OLD.contributeurs)) THEN
            NEW.contributeurs := array_append(OLD.contributeurs, OLD.propose_par);
        END IF;
        IF NEW.statut = 'proposee' AND (NEW.propose_le IS NULL OR NEW.propose_par IS NULL) THEN
            RAISE EXCEPTION 'grille : la proposition porte sa date et son auteur' USING ERRCODE = 'check_violation';
        END IF;
        IF NEW.statut = 'brouillon' AND (NEW.propose_le IS NOT NULL OR NEW.propose_par IS NOT NULL) THEN
            RAISE EXCEPTION 'grille : le retour en brouillon efface la proposition' USING ERRCODE = 'check_violation';
        END IF;
        IF NEW.statut IN ('close', 'supplantee') AND NEW.close_le IS NULL THEN
            RAISE EXCEPTION 'grille : la clôture porte sa date' USING ERRCODE = 'check_violation';
        END IF;
        IF NEW.statut = 'votee' THEN
            IF NEW.vote_le IS NULL OR NEW.vote_reference IS NULL OR NEW.vote_par IS NULL THEN
                RAISE EXCEPTION 'grille : le vote porte sa date, sa référence et son auteur'
                    USING ERRCODE = 'check_violation';
            END IF;
            -- second regard (règle 3) : le votant n'est ni le créateur, ni le proposant, ni l'auteur d'un tarif
            SELECT COALESCE((SELECT p.grille_second_regard FROM public.ent_fin_parametres p
                              WHERE p.etablissement_id = NEW.etablissement_id), true) INTO second;
            IF second AND (NEW.vote_par IS NOT DISTINCT FROM NEW.cree_par
                           OR NEW.vote_par IS NOT DISTINCT FROM NEW.propose_par
                           OR NEW.vote_par = ANY (NEW.contributeurs)
                           OR EXISTS (SELECT 1 FROM public.ent_fin_tarif t
                                       WHERE t.etablissement_id = NEW.etablissement_id
                                         AND t.grille_version_id = NEW.id
                                         AND (t.created_by = NEW.vote_par OR t.modifie_par = NEW.vote_par))) THEN
                RAISE EXCEPTION 'second regard : le votant est l''auteur de la grille (créateur, proposant ou auteur de tarif)'
                    USING ERRCODE = 'restrict_violation';
            END IF;
            -- pièce de la délibération : dans l'établissement (la table des documents est créée après la 523,
            -- une clé étrangère est impossible ici : contrôle à l'écriture)
            IF NEW.vote_document_id IS NOT NULL AND NOT EXISTS (
                SELECT 1 FROM public.ent_doc_document d
                 WHERE d.etablissement_id = NEW.etablissement_id AND d.id = NEW.vote_document_id) THEN
                RAISE EXCEPTION 'grille : pièce de la délibération inconnue dans l''établissement'
                    USING ERRCODE = 'foreign_key_violation';
            END IF;
        END IF;
        IF NEW.statut = 'en_vigueur' THEN
            IF NEW.en_vigueur_du IS NULL OR NEW.en_vigueur_par IS NULL THEN
                RAISE EXCEPTION 'grille : la mise en vigueur porte sa date et son auteur'
                    USING ERRCODE = 'check_violation';
            END IF;
            -- une grille ne tarifie qu'à partir de sa date : elle n'en précède aucune déjà en vigueur
            IF EXISTS (SELECT 1 FROM public.ent_fin_grille_version g
                        WHERE g.etablissement_id = NEW.etablissement_id AND g.annee_scolaire = NEW.annee_scolaire
                          AND g.id <> NEW.id AND g.statut IN ('en_vigueur', 'close')
                          AND g.en_vigueur_du >= NEW.en_vigueur_du) THEN
                RAISE EXCEPTION 'grille : entrée en vigueur antérieure ou égale à celle d''une version déjà appliquée'
                    USING ERRCODE = 'check_violation';
            END IF;
        END IF;
        RETURN NEW;
    END IF;
    -- hors transition
    IF OLD.statut = 'brouillon' THEN
        IF NOT (modifiees <@ ARRAY['libelle', 'motif', 'effet_retroactif', 'contributeurs']) THEN
            RAISE EXCEPTION 'grille : un brouillon ne change que son libellé, son motif, son effet rétroactif et ses contributeurs'
                USING ERRCODE = 'restrict_violation';
        END IF;
        -- les contributeurs ne s'ajoutent qu'en ajout : jamais retirés (preuve du second regard)
        IF NOT (NEW.contributeurs @> OLD.contributeurs) THEN
            RAISE EXCEPTION 'grille : un contributeur ne se retire pas du brouillon (second regard)'
                USING ERRCODE = 'restrict_violation';
        END IF;
        RETURN NEW;
    END IF;
    -- hors brouillon : seule la publication (posée une fois, version votée ou en vigueur)
    IF NOT (modifiees <@ ARRAY['publiee_le']) THEN
        RAISE EXCEPTION 'grille % : une version proposée, votée ou en vigueur ne se modifie plus (R-F10)', OLD.statut
            USING ERRCODE = 'restrict_violation';
    END IF;
    IF OLD.publiee_le IS NOT NULL THEN
        RAISE EXCEPTION 'grille : date de publication déjà posée' USING ERRCODE = 'restrict_violation';
    END IF;
    -- 531 (faible 3) : une version close reste applicable jusqu'à la veille de la suivante : elle se publie
    IF NOT (OLD.statut IN ('votee', 'en_vigueur') OR (OLD.statut = 'close' AND OLD.en_vigueur_du IS NOT NULL)) THEN
        RAISE EXCEPTION 'grille : seule une grille votée, en vigueur ou encore applicable (close) se publie'
            USING ERRCODE = 'restrict_violation';
    END IF;
    RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION ent_trg_fin_tarif_version() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v public.ent_fin_grille_version%ROWTYPE;
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.grille_version_id IS NOT NULL THEN
            -- FOR SHARE : une transition concurrente de la version attend la fin de cette écriture (F3)
            SELECT * INTO v FROM public.ent_fin_grille_version
             WHERE etablissement_id = NEW.etablissement_id AND id = NEW.grille_version_id FOR SHARE;
            IF v.statut IS DISTINCT FROM 'brouillon' THEN
                RAISE EXCEPTION 'tarif : la version de grille n''est plus un brouillon (R-F10)'
                    USING ERRCODE = 'restrict_violation';
            END IF;
            IF v.annee_scolaire <> NEW.annee_scolaire THEN
                RAISE EXCEPTION 'tarif : année différente de celle de la version' USING ERRCODE = 'check_violation';
            END IF;
        ELSIF EXISTS (SELECT 1 FROM public.ent_fin_grille_version g
                       WHERE g.etablissement_id = NEW.etablissement_id AND g.annee_scolaire = NEW.annee_scolaire
                         AND g.statut IN ('votee', 'en_vigueur')) THEN
            RAISE EXCEPTION 'tarif : l''année a une grille votée, passer par une version de grille (R-F10)'
                USING ERRCODE = 'restrict_violation';
        END IF;
        RETURN NEW;
    END IF;
    IF TG_OP = 'UPDATE' THEN
        IF NEW.grille_version_id IS DISTINCT FROM OLD.grille_version_id
           OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id THEN
            RAISE EXCEPTION 'tarif : version de grille et établissement figés' USING ERRCODE = 'restrict_violation';
        END IF;
        IF OLD.grille_version_id IS NULL THEN
            -- tarif historique (avant versions) : seule la désactivation reste possible
            IF (NEW.annee_scolaire, NEW.article_id, NEW.niveau, NEW.regime, NEW.qf_min_centimes,
                NEW.qf_max_centimes, NEW.montant_annuel_centimes, NEW.tarif_par_defaut)
               IS DISTINCT FROM (OLD.annee_scolaire, OLD.article_id, OLD.niveau, OLD.regime, OLD.qf_min_centimes,
                OLD.qf_max_centimes, OLD.montant_annuel_centimes, OLD.tarif_par_defaut) THEN
                RAISE EXCEPTION 'tarif : un tarif ne se modifie pas (désactiver puis créer, FD-19)'
                    USING ERRCODE = 'restrict_violation';
            END IF;
            RETURN NEW;
        END IF;
    END IF;
    -- UPDATE ou DELETE d'un tarif rattaché à une version : brouillon seulement
    SELECT * INTO v FROM public.ent_fin_grille_version
     WHERE etablissement_id = OLD.etablissement_id AND id = OLD.grille_version_id FOR SHARE;
    -- 531 (faible 9) : l'année d'un tarif versionné est celle de sa version, jamais autre
    IF TG_OP = 'UPDATE' AND OLD.grille_version_id IS NOT NULL
       AND NEW.annee_scolaire IS DISTINCT FROM v.annee_scolaire THEN
        RAISE EXCEPTION 'tarif : année différente de celle de la version' USING ERRCODE = 'check_violation';
    END IF;
    IF OLD.grille_version_id IS NOT NULL AND v.statut IS DISTINCT FROM 'brouillon' THEN
        RAISE EXCEPTION 'tarif : la grille % est figée (R-F10)', v.statut USING ERRCODE = 'restrict_violation';
    END IF;
    IF TG_OP = 'DELETE' AND OLD.grille_version_id IS NULL THEN
        RAISE EXCEPTION 'tarif : un tarif historique ne se supprime pas' USING ERRCODE = 'restrict_violation';
    END IF;
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END
$$;

-- ── Historique des calculs : suppression seulement avec la simulation (faible 4) ────────────────────
ALTER TABLE public.ent_fin_simulation_calcul DROP CONSTRAINT IF EXISTS ent_fin_simulation_calcul_fk;
ALTER TABLE public.ent_fin_simulation_calcul ADD CONSTRAINT ent_fin_simulation_calcul_fk
    FOREIGN KEY (etablissement_id, simulation_id)
    REFERENCES public.ent_fin_simulation (etablissement_id, id) ON DELETE CASCADE;

CREATE OR REPLACE FUNCTION ent_trg_fin_simulation_calcul_suppression() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    -- un DELETE direct est refusé ; la cascade de la suppression de la simulation passe (le parent n'existe plus)
    IF EXISTS (SELECT 1 FROM public.ent_fin_simulation s
                WHERE s.etablissement_id = OLD.etablissement_id AND s.id = OLD.simulation_id) THEN
        RAISE EXCEPTION 'ent_fin_simulation_calcul : un calcul historisé ne se supprime qu''avec sa simulation'
            USING ERRCODE = 'restrict_violation';
    END IF;
    RETURN OLD;
END
$$;
DROP TRIGGER IF EXISTS ent_fin_simulation_calcul_sans_suppression ON public.ent_fin_simulation_calcul;
CREATE TRIGGER ent_fin_simulation_calcul_sans_suppression BEFORE DELETE ON public.ent_fin_simulation_calcul
    FOR EACH ROW EXECUTE FUNCTION ent_trg_fin_simulation_calcul_suppression();
