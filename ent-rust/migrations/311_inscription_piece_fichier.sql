-- Lot I3 (socle 3, §8.6) — le fichier d'une pièce d'inscription est une pièce du socle 7.
--
-- Avant l'admission, l'élève n'existe pas (le socle 3 n'en crée aucun avant la validation) : la pièce d'un
-- dossier d'admission ne peut donc pas avoir l'élève pour sujet. Le socle 7 (`07_DOCUMENTS.md` §6, « Sujet »)
-- prévoit « une démarche (dossier d'admission avant l'existence de l'élève) » : `ent_doc_piece` reçoit un
-- quatrième sujet, le DOSSIER d'inscription. Tout le reste du socle 7 s'applique sans changement : quarantaine,
-- antivirus (fermé par défaut), contrôle du type réel, conversion des photos en PDF, PDF désarmé, empreinte,
-- machine à états gardée par la base, décisions en ajout seul, RLS.
--
-- À la validation, la pièce est RATTACHÉE (§8.6) : une nouvelle pièce du socle 7, au sujet élève ou
-- personne, qui référence les mêmes objets (même empreinte, aucune copie d'octets) et reprend la décision
-- (`acceptee`). Les pièces du dossier qui ne se rattachent pas (pièce d'identité vue seulement, accord
-- d'un dossier non abouti) restent au dossier puis se purgent.
--
-- Additif : une colonne, la contrainte de sujet élargie, la garde d'immutabilité étendue au nouveau sujet.

ALTER TABLE ent_doc_piece ADD COLUMN IF NOT EXISTS dossier_id UUID;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_doc_piece_sujet_chk') THEN
        ALTER TABLE ent_doc_piece DROP CONSTRAINT ent_doc_piece_sujet_chk;
    END IF;
    ALTER TABLE ent_doc_piece ADD CONSTRAINT ent_doc_piece_sujet_chk
        CHECK (num_nonnulls(eleve_id, personne_id, agent_id, dossier_id) = 1);
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_doc_piece_dossier_cfk') THEN
        ALTER TABLE ent_doc_piece ADD CONSTRAINT ent_doc_piece_dossier_cfk FOREIGN KEY (etablissement_id, dossier_id)
            REFERENCES ent_inscription_dossier (etablissement_id, id) ON DELETE RESTRICT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_piece_doc_cfk') THEN
        ALTER TABLE ent_inscription_piece ADD CONSTRAINT ent_inscription_piece_doc_cfk FOREIGN KEY (etablissement_id, piece_doc_id)
            REFERENCES ent_doc_piece (etablissement_id, id) ON DELETE RESTRICT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_piece_rattachee_cfk') THEN
        ALTER TABLE ent_inscription_piece ADD CONSTRAINT ent_inscription_piece_rattachee_cfk FOREIGN KEY (etablissement_id, rattachee_doc_id)
            REFERENCES ent_doc_piece (etablissement_id, id) ON DELETE RESTRICT;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_inscription_piece_reprise_cfk') THEN
        ALTER TABLE ent_inscription_piece ADD CONSTRAINT ent_inscription_piece_reprise_cfk FOREIGN KEY (etablissement_id, reprise_de_piece_id)
            REFERENCES ent_inscription_piece (etablissement_id, id) ON DELETE SET NULL (reprise_de_piece_id);
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS ent_doc_piece_dossier_idx
    ON ent_doc_piece (etablissement_id, dossier_id, type_code, deposee_le DESC) WHERE dossier_id IS NOT NULL;

-- Garde de 3704, étendue : le dossier fait partie du sujet immuable.
CREATE OR REPLACE FUNCTION ent_doc_piece_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_doc_piece : suppression interdite (retirer la pièce)' USING ERRCODE = '23001';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.type_code IS DISTINCT FROM OLD.type_code
       OR NEW.eleve_id IS DISTINCT FROM OLD.eleve_id
       OR NEW.personne_id IS DISTINCT FROM OLD.personne_id
       OR NEW.agent_id IS DISTINCT FROM OLD.agent_id
       OR NEW.dossier_id IS DISTINCT FROM OLD.dossier_id
       OR NEW.motif_autre IS DISTINCT FROM OLD.motif_autre
       OR NEW.demande_id IS DISTINCT FROM OLD.demande_id
       OR NEW.deposant_user_id IS DISTINCT FROM OLD.deposant_user_id
       OR NEW.deposant_role IS DISTINCT FROM OLD.deposant_role
       OR NEW.deposee_le IS DISTINCT FROM OLD.deposee_le
       OR NEW.cle_objet_original IS DISTINCT FROM OLD.cle_objet_original
       OR NEW.sha256_original IS DISTINCT FROM OLD.sha256_original
       OR NEW.taille_originale IS DISTINCT FROM OLD.taille_originale
       OR NEW.mime_original IS DISTINCT FROM OLD.mime_original
       OR NEW.nb_fichiers_origine IS DISTINCT FROM OLD.nb_fichiers_origine THEN
        RAISE EXCEPTION 'ent_doc_piece : identité, sujet, déposant et original sont immuables' USING ERRCODE = '23001';
    END IF;
    IF OLD.cle_objet_servi IS NOT NULL
       AND (NEW.cle_objet_servi IS DISTINCT FROM OLD.cle_objet_servi OR NEW.sha256_servi IS DISTINCT FROM OLD.sha256_servi
            OR NEW.nb_pages IS DISTINCT FROM OLD.nb_pages) THEN
        RAISE EXCEPTION 'ent_doc_piece : le fichier servi ne change plus' USING ERRCODE = '23001';
    END IF;
    IF NEW.etat IS DISTINCT FROM OLD.etat THEN
        IF NOT (
               (OLD.etat = 'en_quarantaine' AND NEW.etat IN ('recue', 'infectee', 'rejetee', 'retiree'))
            OR (OLD.etat = 'recue' AND NEW.etat IN ('acceptee', 'refusee', 'sans_objet', 'retiree'))) THEN
            RAISE EXCEPTION 'ent_doc_piece : transition % → % interdite', OLD.etat, NEW.etat USING ERRCODE = '23001';
        END IF;
        IF OLD.etat = 'en_quarantaine' AND NEW.etat = 'recue' AND NEW.cle_objet_servi IS NULL THEN
            RAISE EXCEPTION 'ent_doc_piece : une pièce ne sort de quarantaine qu''avec son fichier servi' USING ERRCODE = '23001';
        END IF;
    ELSIF OLD.etat IN ('acceptee', 'refusee', 'sans_objet', 'infectee', 'rejetee', 'retiree')
          AND (NEW.valable_jusqu_au IS DISTINCT FROM OLD.valable_jusqu_au
               OR NEW.controle_par IS DISTINCT FROM OLD.controle_par OR NEW.controle_le IS DISTINCT FROM OLD.controle_le
               OR NEW.refus_motif IS DISTINCT FROM OLD.refus_motif OR NEW.commentaire IS DISTINCT FROM OLD.commentaire) THEN
        RAISE EXCEPTION 'ent_doc_piece : décision finale, plus modifiable' USING ERRCODE = '23001';
    END IF;
    RETURN NEW;
END
$$;
