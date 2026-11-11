-- D-036 (lot 2) — `ent_personne_fusion.instantane` (JSONB) est RELU à l'annulation de la fusion
-- pour restaurer l'état de la personne absorbée : c'est de la mémoire de l'application, pas un
-- instantané d'audit figé. Il devient :
--   * trois colonnes typées sur `ent_personne_fusion` (compte transféré, agent déplacé, fusion liée
--     à une fusion d'agents) ;
--   * `ent_personne_fusion_objet` : liens élève-responsable et contacts d'urgence DÉPLACÉS (identifiants ;
--     référence polymorphe sans clé étrangère, les lignes déplacées pouvant disparaître ensuite) ;
--   * `ent_personne_fusion_lien_supprime` et `ent_personne_fusion_contact_supprime` : copie typée,
--     colonne pour colonne, des lignes en doublon SUPPRIMÉES à la fusion (nécessaires pour les
--     recréer à l'identique). Un test de garde (`tests_d036`) échoue si leurs colonnes divergent
--     de celles de `ent_eleve_lien` / `ent_eleve_contact_urgence`.
-- Reprise idempotente (`detail_normalise`) ; ancienne colonne facultative et en LECTURE SEULE.

ALTER TABLE ent_personne_fusion ADD COLUMN IF NOT EXISTS compte_transfere UUID;
ALTER TABLE ent_personne_fusion ADD COLUMN IF NOT EXISTS agent_deplace UUID;
ALTER TABLE ent_personne_fusion ADD COLUMN IF NOT EXISTS agents_fusionnes BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_personne_fusion ADD COLUMN IF NOT EXISTS detail_normalise BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_personne_fusion ALTER COLUMN instantane DROP NOT NULL;
SELECT ent_d036_contrainte('ent_personne_fusion', 'ent_personne_fusion_etab_id_uq', 'UNIQUE (etablissement_id, id)');

CREATE TABLE IF NOT EXISTS ent_personne_fusion_objet (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    fusion_id UUID NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN ('lien', 'contact_urgence')),
    objet_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_personne_fusion_objet_uq UNIQUE (fusion_id, nature, objet_id),
    CONSTRAINT ent_personne_fusion_objet_fusion_fk FOREIGN KEY (etablissement_id, fusion_id)
        REFERENCES ent_personne_fusion (etablissement_id, id) DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX IF NOT EXISTS ent_personne_fusion_objet_etab_idx
    ON ent_personne_fusion_objet (etablissement_id, fusion_id);

CREATE TABLE IF NOT EXISTS ent_personne_fusion_lien_supprime (
    fusion_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    user_id UUID,
    foyer_id UUID,
    lien TEXT NOT NULL,
    peut_voir_notes BOOLEAN NOT NULL,
    peut_voir_vs BOOLEAN NOT NULL,
    peut_justifier_absence BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    peut_autoriser_sorties BOOLEAN NOT NULL,
    est_payeur BOOLEAN NOT NULL,
    part_paiement SMALLINT,
    personne_id UUID NOT NULL,
    autorite_parentale BOOLEAN NOT NULL,
    garde TEXT,
    date_debut DATE,
    date_fin DATE,
    suspendu_le TIMESTAMPTZ,
    suspendu_par UUID,
    suspension_motif TEXT,
    suspension_reference TEXT,
    updated_at TIMESTAMPTZ NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (fusion_id, id),
    CONSTRAINT ent_personne_fusion_lien_supprime_fusion_fk FOREIGN KEY (etablissement_id, fusion_id)
        REFERENCES ent_personne_fusion (etablissement_id, id) DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX IF NOT EXISTS ent_personne_fusion_lien_supprime_etab_idx
    ON ent_personne_fusion_lien_supprime (etablissement_id, fusion_id);

CREATE TABLE IF NOT EXISTS ent_personne_fusion_contact_supprime (
    fusion_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    personne_id UUID NOT NULL,
    relation TEXT NOT NULL,
    ordre SMALLINT NOT NULL,
    peut_recuperer BOOLEAN NOT NULL,
    statut TEXT NOT NULL,
    commentaire TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    declare_par_personne_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (fusion_id, id),
    CONSTRAINT ent_personne_fusion_contact_supprime_fusion_fk FOREIGN KEY (etablissement_id, fusion_id)
        REFERENCES ent_personne_fusion (etablissement_id, id) DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX IF NOT EXISTS ent_personne_fusion_contact_supprime_etab_idx
    ON ent_personne_fusion_contact_supprime (etablissement_id, fusion_id);

-- Reprise (fusions pas encore normalisées). Identifiants des lignes `objet` déterministes (md5).
UPDATE ent_personne_fusion SET
    compte_transfere = CASE WHEN instantane ->> 'compte_transfere' ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
                            THEN (instantane ->> 'compte_transfere')::uuid END,
    agent_deplace = CASE WHEN instantane ->> 'agent_deplace' ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
                         THEN (instantane ->> 'agent_deplace')::uuid END,
    agents_fusionnes = coalesce((instantane ->> 'agents_fusionnes') = 'true', false)
 WHERE NOT detail_normalise AND jsonb_typeof(instantane) = 'object';

INSERT INTO ent_personne_fusion_objet (id, etablissement_id, fusion_id, nature, objet_id)
SELECT md5(f.id::text || '/lien/' || t)::uuid, f.etablissement_id, f.id, 'lien', t::uuid
  FROM ent_personne_fusion f, LATERAL jsonb_array_elements_text(
           CASE WHEN jsonb_typeof(f.instantane -> 'liens_deplaces') = 'array' THEN f.instantane -> 'liens_deplaces' ELSE '[]'::jsonb END) t
 WHERE NOT f.detail_normalise AND t ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
ON CONFLICT DO NOTHING;
INSERT INTO ent_personne_fusion_objet (id, etablissement_id, fusion_id, nature, objet_id)
SELECT md5(f.id::text || '/contact_urgence/' || t)::uuid, f.etablissement_id, f.id, 'contact_urgence', t::uuid
  FROM ent_personne_fusion f, LATERAL jsonb_array_elements_text(
           CASE WHEN jsonb_typeof(f.instantane -> 'contacts_deplaces') = 'array' THEN f.instantane -> 'contacts_deplaces' ELSE '[]'::jsonb END) t
 WHERE NOT f.detail_normalise AND t ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
ON CONFLICT DO NOTHING;

INSERT INTO ent_personne_fusion_lien_supprime
    (fusion_id, etablissement_id, id, eleve_id, user_id, foyer_id, lien, peut_voir_notes, peut_voir_vs,
     peut_justifier_absence, created_at, peut_autoriser_sorties, est_payeur, part_paiement, personne_id,
     autorite_parentale, garde, date_debut, date_fin, suspendu_le, suspendu_par, suspension_motif,
     suspension_reference, updated_at)
SELECT f.id, f.etablissement_id, r.id, r.eleve_id, r.user_id, r.foyer_id, r.lien, r.peut_voir_notes, r.peut_voir_vs,
       r.peut_justifier_absence, r.created_at, r.peut_autoriser_sorties, r.est_payeur, r.part_paiement, r.personne_id,
       r.autorite_parentale, r.garde, r.date_debut, r.date_fin, r.suspendu_le, r.suspendu_par, r.suspension_motif,
       r.suspension_reference, r.updated_at
  FROM ent_personne_fusion f,
       LATERAL jsonb_array_elements(
           CASE WHEN jsonb_typeof(f.instantane -> 'liens_supprimes') = 'array' THEN f.instantane -> 'liens_supprimes' ELSE '[]'::jsonb END) e,
       LATERAL jsonb_populate_record(NULL::ent_eleve_lien, e) r
 WHERE NOT f.detail_normalise AND f.annule_le IS NULL AND r.id IS NOT NULL AND r.eleve_id IS NOT NULL
   AND r.personne_id IS NOT NULL AND r.lien IS NOT NULL
   AND r.peut_voir_notes IS NOT NULL AND r.peut_voir_vs IS NOT NULL AND r.peut_justifier_absence IS NOT NULL
   AND r.created_at IS NOT NULL AND r.peut_autoriser_sorties IS NOT NULL AND r.est_payeur IS NOT NULL
   AND r.autorite_parentale IS NOT NULL AND r.updated_at IS NOT NULL
ON CONFLICT DO NOTHING;

INSERT INTO ent_personne_fusion_contact_supprime
    (fusion_id, etablissement_id, id, eleve_id, personne_id, relation, ordre, peut_recuperer, statut,
     commentaire, created_at, updated_at, declare_par_personne_id)
SELECT f.id, f.etablissement_id, r.id, r.eleve_id, r.personne_id, r.relation, r.ordre, r.peut_recuperer, r.statut,
       r.commentaire, r.created_at, r.updated_at, r.declare_par_personne_id
  FROM ent_personne_fusion f,
       LATERAL jsonb_array_elements(
           CASE WHEN jsonb_typeof(f.instantane -> 'contacts_supprimes') = 'array' THEN f.instantane -> 'contacts_supprimes' ELSE '[]'::jsonb END) e,
       LATERAL jsonb_populate_record(NULL::ent_eleve_contact_urgence, e) r
 WHERE NOT f.detail_normalise AND f.annule_le IS NULL AND r.id IS NOT NULL AND r.eleve_id IS NOT NULL
   AND r.personne_id IS NOT NULL AND r.relation IS NOT NULL AND r.ordre IS NOT NULL
   AND r.peut_recuperer IS NOT NULL AND r.statut IS NOT NULL AND r.created_at IS NOT NULL AND r.updated_at IS NOT NULL
ON CONFLICT DO NOTHING;

UPDATE ent_personne_fusion SET detail_normalise = true WHERE NOT detail_normalise AND instantane IS NOT NULL;

CREATE OR REPLACE FUNCTION ent_personne_fusion_instantane_lecture_seule() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.instantane IS NOT NULL AND (TG_OP = 'INSERT' OR NEW.instantane IS DISTINCT FROM OLD.instantane) THEN
        RAISE EXCEPTION 'ent_personne_fusion.instantane est en lecture seule (D-036) : utiliser les tables de fusion'
            USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_personne_fusion_instantane_lecture_seule ON ent_personne_fusion;
CREATE TRIGGER ent_personne_fusion_instantane_lecture_seule
    BEFORE INSERT OR UPDATE OF instantane ON ent_personne_fusion
    FOR EACH ROW EXECUTE FUNCTION ent_personne_fusion_instantane_lecture_seule();

SELECT ent_rls_poser(ARRAY['ent_personne_fusion_objet', 'ent_personne_fusion_lien_supprime',
                           'ent_personne_fusion_contact_supprime']);
