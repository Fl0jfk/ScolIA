-- 2249 — Socle 12, lot AS5 : courriers postaux de la vie scolaire (relance sans réponse, RA-04, R-AS18).
-- Un courrier est PRÉPARÉ par la vie scolaire, DÉCIDÉ (validé) par une personne habilitée, puis noté
-- « posté » : ScolIA n'en envoie aucun d'elle-même (R-AS18). Aucune adresse n'est copiée ici : le courrier
-- vise une personne (responsable) et l'adresse est lue à l'édition, jamais celle de l'autre parent quand
-- elle est protégée (socle 4). La trame et le PDF relèvent du socle 7 (documents) : non faits dans ce lot.
-- Additive, rejouable (E8), sans colonne JSON (D-036) ; RLS par `ent_rls_poser`.

CREATE TABLE IF NOT EXISTS ent_asd_courrier (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    absence_id UUID,
    personne_id UUID,
    nature TEXT NOT NULL CHECK (nature IN ('relance_absence', 'avis_absence', 'autre')),
    statut TEXT NOT NULL DEFAULT 'prepare' CHECK (statut IN ('prepare', 'valide', 'poste', 'abandonne')),
    prepare_par UUID NOT NULL,
    prepare_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    valide_par UUID,
    valide_le TIMESTAMPTZ,
    poste_par UUID,
    poste_le TIMESTAMPTZ,
    abandonne_par UUID,
    abandonne_le TIMESTAMPTZ,
    abandon_code TEXT CHECK (abandon_code IS NULL OR abandon_code IN ('motif_recu', 'erreur', 'famille_jointe', 'decision_direction')),
    CONSTRAINT ent_asd_courrier_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_asd_courrier_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_asd_courrier_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_asd_absence (etablissement_id, id),
    CONSTRAINT ent_asd_courrier_personne_fk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id),
    CONSTRAINT ent_asd_courrier_statut_chk CHECK (
        (statut = 'prepare' AND valide_par IS NULL AND poste_par IS NULL AND abandonne_par IS NULL)
        OR (statut = 'valide' AND valide_par IS NOT NULL AND valide_le IS NOT NULL AND poste_par IS NULL AND abandonne_par IS NULL)
        OR (statut = 'poste' AND valide_par IS NOT NULL AND poste_par IS NOT NULL AND poste_le IS NOT NULL AND abandonne_par IS NULL)
        OR (statut = 'abandonne' AND abandonne_par IS NOT NULL AND abandonne_le IS NOT NULL AND abandon_code IS NOT NULL AND poste_par IS NULL))
);
-- Un seul courrier en cours (préparé ou validé) par épisode et par nature.
CREATE UNIQUE INDEX IF NOT EXISTS ent_asd_courrier_en_cours_uq
    ON ent_asd_courrier (etablissement_id, absence_id, nature) WHERE absence_id IS NOT NULL AND statut IN ('prepare', 'valide');
CREATE INDEX IF NOT EXISTS ent_asd_courrier_eleve_idx ON ent_asd_courrier (etablissement_id, eleve_id, prepare_le DESC);

-- Un courrier ne se supprime pas et ses champs d'origine sont figés ; seuls les jalons avancent.
CREATE OR REPLACE FUNCTION ent_asd_courrier_garde() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_asd_courrier ne se supprime pas (abandon tracé)' USING ERRCODE = '23514';
    END IF;
    IF NEW.id <> OLD.id OR NEW.etablissement_id <> OLD.etablissement_id OR NEW.eleve_id <> OLD.eleve_id
       OR NEW.absence_id IS DISTINCT FROM OLD.absence_id OR NEW.personne_id IS DISTINCT FROM OLD.personne_id
       OR NEW.nature <> OLD.nature OR NEW.prepare_par <> OLD.prepare_par OR NEW.prepare_le <> OLD.prepare_le THEN
        RAISE EXCEPTION 'les champs d''origine d''un courrier sont figés' USING ERRCODE = '23514';
    END IF;
    IF OLD.statut IN ('poste', 'abandonne') THEN
        RAISE EXCEPTION 'un courrier posté ou abandonné ne change plus' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_asd_courrier_garde_trg ON ent_asd_courrier;
CREATE TRIGGER ent_asd_courrier_garde_trg BEFORE UPDATE OR DELETE ON ent_asd_courrier
    FOR EACH ROW EXECUTE FUNCTION ent_asd_courrier_garde();

SELECT ent_rls_poser(ARRAY['ent_asd_courrier']);
