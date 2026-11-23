-- Socle 21 — contre-revue de V5 (M-A, F-3). Additive et rejouable ; 477 (versée) n'est PAS modifiée.
--
-- M-A (D-069) : une moyenne externe publiée puis CORRIGÉE devient « non publiée » (la famille et l'élève ne voient plus
-- rien jusqu'à une nouvelle publication) ; l'événement « correction » du journal dit qu'elle était visible. Une
-- publication se RETIRE par un geste distinct (direction, motif de 3 à 500 caractères, événement « retrait »).
-- F-3 : seule une ligne de statut « moyenne » se publie, vérifié en base (avant : seulement par le service).

ALTER TABLE eva_moyenne_externe_historique ADD COLUMN IF NOT EXISTS etait_publiee BOOLEAN NOT NULL DEFAULT false;

ALTER TABLE eva_moyenne_externe_historique DROP CONSTRAINT IF EXISTS eva_moyenne_externe_historique_evenement_check;
ALTER TABLE eva_moyenne_externe_historique ADD CONSTRAINT eva_moyenne_externe_historique_evenement_check
    CHECK (evenement IN ('saisie', 'correction', 'publication', 'retrait'));
ALTER TABLE eva_moyenne_externe_historique DROP CONSTRAINT IF EXISTS eva_moyenne_externe_histo_motif_chk;
ALTER TABLE eva_moyenne_externe_historique ADD CONSTRAINT eva_moyenne_externe_histo_motif_chk
    CHECK (evenement NOT IN ('correction', 'retrait') OR motif IS NOT NULL);

-- L'auteur d'un retrait de publication est porté par une colonne dédiée : `saisi_par` reste l'auteur de la VALEUR.
-- `retrait_par` est une ENTRÉE (remise à NULL à chaque écriture, comme `motif_correction`) ; `dernier_retrait_par` la garde.
ALTER TABLE eva_moyenne_externe ADD COLUMN IF NOT EXISTS retrait_par UUID;
ALTER TABLE eva_moyenne_externe ADD COLUMN IF NOT EXISTS dernier_retrait_par UUID;

-- La contrainte est posée NOT VALID : sous 477, une correction GARDAIT la publication, donc une ligne publiée puis
-- corrigée en « en_attente » ou « dispense » peut déjà exister quelque part ; la migration ne doit pas échouer. Elle
-- s'applique à toute écriture nouvelle ; les lignes hors règle sont dépubliées plus bas, et constatées au démarrage
-- (`migrate.rs` : journal `tracing::error!`).
ALTER TABLE eva_moyenne_externe DROP CONSTRAINT IF EXISTS eva_moyenne_externe_publication_statut_chk;
ALTER TABLE eva_moyenne_externe ADD CONSTRAINT eva_moyenne_externe_publication_statut_chk
    CHECK (publiee_le IS NULL OR statut = 'moyenne') NOT VALID;

CREATE OR REPLACE FUNCTION ent_trg_eva_moyenne_externe() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    v_eleve uuid;
    v_motif text;
    v_contenu boolean;
    v_publication boolean;
    v_retrait_par uuid;
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'eva_moyenne_externe : suppression physique interdite' USING ERRCODE = '23514';
    END IF;
    SELECT e.eleve_id INTO v_eleve FROM eva_enseignement_externe e
     WHERE e.etablissement_id = NEW.etablissement_id AND e.id = NEW.enseignement_externe_id;
    IF TG_OP = 'INSERT' THEN
        IF NEW.publiee_le IS NOT NULL OR NEW.publiee_par IS NOT NULL THEN
            RAISE EXCEPTION 'eva_moyenne_externe : une moyenne externe naît non publiée' USING ERRCODE = '23514';
        END IF;
        IF NOT eva_document_de_l_eleve(NEW.etablissement_id, v_eleve, NEW.document_id) THEN
            RAISE EXCEPTION 'eva_moyenne_externe : la pièce n''appartient pas à cet élève dans cet établissement'
                USING ERRCODE = '23514';
        END IF;
        NEW.motif_correction := NULL;
        NEW.dernier_motif_correction := NULL;
        NEW.retrait_par := NULL;
        NEW.dernier_retrait_par := NULL;
        RETURN NEW;
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.enseignement_externe_id IS DISTINCT FROM OLD.enseignement_externe_id
       OR NEW.periode_id IS DISTINCT FROM OLD.periode_id OR NEW.bareme IS DISTINCT FROM OLD.bareme
       OR NEW.cree_le IS DISTINCT FROM OLD.cree_le THEN
        RAISE EXCEPTION 'eva_moyenne_externe : enseignement, période et barème ne changent pas' USING ERRCODE = '23514';
    END IF;
    v_motif := NEW.motif_correction;
    v_retrait_par := NEW.retrait_par;
    NEW.motif_correction := NULL;
    NEW.retrait_par := NULL;
    v_contenu := NEW.statut IS DISTINCT FROM OLD.statut OR NEW.valeur IS DISTINCT FROM OLD.valeur
              OR NEW.appreciation_transcrite IS DISTINCT FROM OLD.appreciation_transcrite
              OR NEW.document_id IS DISTINCT FROM OLD.document_id;
    v_publication := NEW.publiee_le IS DISTINCT FROM OLD.publiee_le OR NEW.publiee_par IS DISTINCT FROM OLD.publiee_par;
    -- Rien ne change : aucune écriture, aucun journal.
    IF NOT v_contenu AND NOT v_publication THEN
        RETURN NULL;
    END IF;
    IF v_contenu AND v_publication THEN
        RAISE EXCEPTION 'eva_moyenne_externe : on ne corrige pas et ne publie (ou retire) pas dans le même geste'
            USING ERRCODE = '23514';
    END IF;
    IF v_publication THEN
        IF OLD.publiee_le IS NULL THEN
            -- Publication (second geste) : seule une moyenne (F-3), jamais une ligne « en attente » ou « dispense ».
            IF NEW.statut <> 'moyenne' THEN
                RAISE EXCEPTION 'eva_moyenne_externe : seule une moyenne se publie' USING ERRCODE = '23514';
            END IF;
        ELSE
            -- Retrait de publication : la ligne redevient non publiée, avec un motif tracé ; on ne « republie » pas
            -- d'un cran (changer la date ou l'auteur d'une publication) : on retire, puis on publie.
            IF NEW.publiee_le IS NOT NULL OR NEW.publiee_par IS NOT NULL THEN
                RAISE EXCEPTION 'eva_moyenne_externe : une publication ne se modifie pas, elle se retire (avec un motif)'
                    USING ERRCODE = '23514';
            END IF;
            IF v_motif IS NULL OR char_length(btrim(v_motif)) NOT BETWEEN 3 AND 500 THEN
                RAISE EXCEPTION 'eva_moyenne_externe : un retrait de publication porte un motif de 3 à 500 caractères'
                    USING ERRCODE = '23514';
            END IF;
            IF v_retrait_par IS NULL THEN
                RAISE EXCEPTION 'eva_moyenne_externe : un retrait de publication porte son auteur' USING ERRCODE = '23514';
            END IF;
            NEW.dernier_motif_correction := btrim(v_motif);
            NEW.dernier_retrait_par := v_retrait_par;
        END IF;
        RETURN NEW;
    END IF;
    -- Correction : motif de 3 à 500 caractères, pièce de l'élève ; si la ligne était publiée, elle REDEVIENT non
    -- publiée (la famille ne voit plus rien jusqu'à la nouvelle publication) ; le journal dit qu'elle était visible.
    IF v_motif IS NULL OR char_length(btrim(v_motif)) NOT BETWEEN 3 AND 500 THEN
        RAISE EXCEPTION 'eva_moyenne_externe : une correction porte un motif de 3 à 500 caractères' USING ERRCODE = '23514';
    END IF;
    IF NEW.document_id IS DISTINCT FROM OLD.document_id
       AND NOT eva_document_de_l_eleve(NEW.etablissement_id, v_eleve, NEW.document_id) THEN
        RAISE EXCEPTION 'eva_moyenne_externe : la pièce n''appartient pas à cet élève dans cet établissement'
            USING ERRCODE = '23514';
    END IF;
    NEW.dernier_motif_correction := btrim(v_motif);
    NEW.publiee_le := NULL;
    NEW.publiee_par := NULL;
    RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION ent_trg_eva_moyenne_externe_historique() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    v_evt text;
BEGIN
    IF TG_OP = 'INSERT' THEN
        v_evt := 'saisie';
    ELSIF NEW.statut IS DISTINCT FROM OLD.statut OR NEW.valeur IS DISTINCT FROM OLD.valeur
       OR NEW.appreciation_transcrite IS DISTINCT FROM OLD.appreciation_transcrite
       OR NEW.document_id IS DISTINCT FROM OLD.document_id THEN
        v_evt := 'correction';
    ELSIF OLD.publiee_le IS NULL THEN
        v_evt := 'publication';
    ELSE
        v_evt := 'retrait';
    END IF;
    INSERT INTO eva_moyenne_externe_historique (id, etablissement_id, moyenne_id, enseignement_externe_id, evenement,
                                                statut_avant, valeur_avant, statut_apres, valeur_apres,
                                                document_avant, document_apres, motif, par, etait_publiee)
    VALUES (gen_random_uuid(), NEW.etablissement_id, NEW.id, NEW.enseignement_externe_id, v_evt,
            CASE WHEN TG_OP = 'UPDATE' THEN OLD.statut END, CASE WHEN TG_OP = 'UPDATE' THEN OLD.valeur END,
            NEW.statut, NEW.valeur,
            CASE WHEN TG_OP = 'UPDATE' THEN OLD.document_id END, NEW.document_id,
            CASE WHEN v_evt IN ('correction', 'retrait') THEN NEW.dernier_motif_correction END,
            CASE v_evt WHEN 'publication' THEN NEW.publiee_par WHEN 'retrait' THEN NEW.dernier_retrait_par ELSE NEW.saisi_par END,
            TG_OP = 'UPDATE' AND OLD.publiee_le IS NOT NULL);
    RETURN NULL;
END
$$;

-- Lignes hors règle héritées de 477 (publiées, puis corrigées alors que la publication était gardée : en « en_attente » /
-- « dispense », ou en « moyenne » avec une autre valeur, que le journal date APRÈS la publication) : DÉPUBLIÉES, pour qu'une valeur corrigée et non republiée ne reste jamais visible de la famille. Le
-- geste passe par le déclencheur ci-dessus comme un retrait ordinaire : le journal reçoit l'événement « retrait »
-- (motif « correction antérieure à 478 », auteur = dernier auteur de la valeur). Aucune valeur n'est modifiée.
-- Rejouable : une fois dépubliées, plus aucune ligne n'est concernée.
UPDATE eva_moyenne_externe
   SET publiee_le = NULL, publiee_par = NULL,
       motif_correction = 'correction antérieure à 478', retrait_par = saisi_par
 WHERE publiee_le IS NOT NULL
   AND (statut <> 'moyenne'
        OR EXISTS (SELECT 1 FROM public.eva_moyenne_externe_historique h
                    WHERE h.etablissement_id = eva_moyenne_externe.etablissement_id
                      AND h.moyenne_id = eva_moyenne_externe.id
                      AND h.evenement = 'correction'
                      AND h.le > eva_moyenne_externe.publiee_le));

-- Plus aucune ligne hors règle : la contrainte est validée. Si une ligne échappait au geste ci-dessus, la migration
-- s'arrête ici plutôt que de laisser une valeur non republiée visible.
ALTER TABLE eva_moyenne_externe VALIDATE CONSTRAINT eva_moyenne_externe_publication_statut_chk;
