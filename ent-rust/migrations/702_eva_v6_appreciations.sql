-- Socle 21, lot V6 « appréciations » (`docs/socles/21_EVALUATIONS.md` §12, §17.10, EVA-194 à EVA-206 hors IA ;
-- livraison : `docs/socles/21_EVALUATIONS_V6_LIVRAISON.md`). Plage d'extension 701–719 du socle 21 ; posée en FIN du
-- tableau de `migrate.rs` (règle 38). Additive et rejouable ; aucune migration versée n'est modifiée (règle 25).
--
-- Les appréciations elles-mêmes (`eva_appreciation`, son historique, la banque de phrases) existent depuis 465 (V1).
-- Ce lot y ajoute :
--   * la liste courte des mots interdits de l'établissement (S-117), lue par les contrôles de cohérence ;
--   * les remarques de relecture adressées aux auteurs par le professeur principal ou la direction (EVA-204) ;
--   * le journal des corrections faites par la direction après la fermeture de la fenêtre (motif obligatoire).
-- Aucune colonne JSON (D-036). Fonctions avec `search_path` fixé, relations qualifiées `public.`.

-- 1. Mots interdits de l'établissement (S-117 : liste courte, avertissement non bloquant) ------------------------------
CREATE TABLE IF NOT EXISTS eva_appreciation_mot_interdit (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    mot TEXT NOT NULL CHECK (char_length(btrim(mot)) BETWEEN 2 AND 60 AND mot = btrim(mot)),
    cree_par UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS eva_appreciation_mot_interdit_uq
    ON eva_appreciation_mot_interdit (etablissement_id, lower(mot));

-- 2. Remarques de relecture (EVA-204) -------------------------------------------------------------------------------------
-- Une remarque vise UNE version validée d'une appréciation ; elle est adressée à l'auteur de cette version et ne quitte
-- jamais l'établissement (aucune famille, aucun élève ne la lit). Elle ne se modifie pas : seul l'auteur visé la marque
-- « traitée », une fois.
CREATE TABLE IF NOT EXISTS eva_appreciation_remarque (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    appreciation_id UUID NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    destinataire UUID NOT NULL,
    texte TEXT NOT NULL CHECK (char_length(btrim(texte)) BETWEEN 3 AND 500),
    auteur UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    traitee_le TIMESTAMPTZ,
    CONSTRAINT eva_appreciation_remarque_fk FOREIGN KEY (appreciation_id, etablissement_id)
        REFERENCES public.eva_appreciation (id, etablissement_id),
    CONSTRAINT eva_appreciation_remarque_auteur_chk CHECK (auteur <> destinataire)
);
CREATE INDEX IF NOT EXISTS eva_appreciation_remarque_dest_idx
    ON eva_appreciation_remarque (etablissement_id, destinataire, traitee_le);
CREATE INDEX IF NOT EXISTS eva_appreciation_remarque_app_idx
    ON eva_appreciation_remarque (etablissement_id, appreciation_id);

CREATE OR REPLACE FUNCTION eva_appreciation_remarque_garde() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'eva_appreciation_remarque : une remarque ne se supprime pas' USING ERRCODE = '23514';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.appreciation_id IS DISTINCT FROM OLD.appreciation_id OR NEW.version IS DISTINCT FROM OLD.version
       OR NEW.destinataire IS DISTINCT FROM OLD.destinataire OR NEW.texte IS DISTINCT FROM OLD.texte
       OR NEW.auteur IS DISTINCT FROM OLD.auteur OR NEW.cree_le IS DISTINCT FROM OLD.cree_le THEN
        RAISE EXCEPTION 'eva_appreciation_remarque : une remarque ne se modifie pas' USING ERRCODE = '23514';
    END IF;
    IF OLD.traitee_le IS NOT NULL THEN
        RAISE EXCEPTION 'eva_appreciation_remarque : remarque déjà traitée' USING ERRCODE = '23514';
    END IF;
    IF NEW.traitee_le IS NULL THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS eva_appreciation_remarque_garde_trg ON eva_appreciation_remarque;
CREATE TRIGGER eva_appreciation_remarque_garde_trg BEFORE UPDATE OR DELETE ON eva_appreciation_remarque
    FOR EACH ROW EXECUTE FUNCTION eva_appreciation_remarque_garde();

-- 3. Corrections de la direction après la fenêtre (motif obligatoire, ajout seul) ----------------------------------------
-- La version écrite est celle que produit la correction dans `eva_appreciation_historique` (même numéro).
CREATE TABLE IF NOT EXISTS eva_appreciation_correction (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    appreciation_id UUID NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    etat_periode TEXT NOT NULL
        CHECK (etat_periode IN ('ouverte', 'notes_arretees', 'appreciations_closes', 'conseil_tenu', 'close')),
    motif TEXT NOT NULL CHECK (char_length(btrim(motif)) BETWEEN 10 AND 500),
    par UUID NOT NULL,
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT eva_appreciation_correction_fk FOREIGN KEY (appreciation_id, etablissement_id)
        REFERENCES public.eva_appreciation (id, etablissement_id),
    CONSTRAINT eva_appreciation_correction_uq UNIQUE (appreciation_id, version)
);

CREATE OR REPLACE FUNCTION eva_v6_ajout_seul() RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    RAISE EXCEPTION '% : journal en ajout seul', TG_TABLE_NAME USING ERRCODE = '23514';
END
$$;
DROP TRIGGER IF EXISTS eva_appreciation_correction_ajout_seul_trg ON eva_appreciation_correction;
CREATE TRIGGER eva_appreciation_correction_ajout_seul_trg BEFORE UPDATE OR DELETE ON eva_appreciation_correction
    FOR EACH ROW EXECUTE FUNCTION eva_v6_ajout_seul();
DROP TRIGGER IF EXISTS eva_appreciation_correction_vidage_trg ON eva_appreciation_correction;
CREATE TRIGGER eva_appreciation_correction_vidage_trg BEFORE TRUNCATE ON eva_appreciation_correction
    FOR EACH STATEMENT EXECUTE FUNCTION eva_v6_ajout_seul();
DROP TRIGGER IF EXISTS eva_appreciation_remarque_vidage_trg ON eva_appreciation_remarque;
CREATE TRIGGER eva_appreciation_remarque_vidage_trg BEFORE TRUNCATE ON eva_appreciation_remarque
    FOR EACH STATEMENT EXECUTE FUNCTION eva_v6_ajout_seul();

SELECT ent_rls_poser(ARRAY['eva_appreciation_mot_interdit', 'eva_appreciation_remarque', 'eva_appreciation_correction']);
