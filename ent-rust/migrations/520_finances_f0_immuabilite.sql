-- 520 — Finances, lot F0 (correctifs urgents de l'existant) : socles 24 et 25.
--
-- Réf. : docs/socles/24_FINANCES.md (FD-05, FD-08, FD-09, FD-10, FD-11, FD-13,
-- FD-16, R-F2, R-F3, R-P6, R-P9, R-P16), 25_PAIEMENTS.md. Règle commune : une
-- pièce comptable n'est jamais effacée ni réécrite ; seuls des champs de suivi
-- (statut de solde, date de solde, code de lettrage) changent après l'émission.
--
-- Cette migration ne modifie aucune donnée monétaire (aucun montant, aucun
-- numéro, aucune date de pièce) : elle supprime les cascades de suppression,
-- pose des déclencheurs de protection, annule les prélèvements prévus sur des
-- mandats déjà révoqués et force à zéro les frais de relance.

-- ── FD-08 : plus aucun ON DELETE CASCADE depuis une pièce comptable ──────────
DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN
        SELECT c.conname, c.conrelid::regclass AS rel, pg_get_constraintdef(c.oid) AS def
          FROM pg_constraint c
         WHERE c.contype = 'f'
           AND c.confdeltype = 'c'
           AND c.confrelid IN (
                'ent_fac_facture'::regclass, 'ent_fac_ligne'::regclass,
                'ent_fac_echeance'::regclass, 'ent_fac_encaissement'::regclass,
                'ent_fac_quittance'::regclass, 'ent_fac_sepa_mandat'::regclass,
                'ent_fac_sepa_prelevement'::regclass)
    LOOP
        EXECUTE format('ALTER TABLE %s DROP CONSTRAINT %I', r.rel, r.conname);
        EXECUTE format('ALTER TABLE %s ADD CONSTRAINT %I %s', r.rel, r.conname,
                       replace(r.def, 'ON DELETE CASCADE', 'ON DELETE RESTRICT'));
    END LOOP;
END
$$;

-- ── FD-09 / R-F2 : facture émise ou soldée immuable ──────────────────────────
-- Seuls `statut` (emise <-> soldee) et `soldee_at` peuvent changer ; jamais de
-- retour à `brouillon`, jamais de suppression.
CREATE OR REPLACE FUNCTION ent_fac_facture_protege() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF OLD.statut <> 'brouillon' THEN
            RAISE EXCEPTION 'ent_fac_facture %: une pièce émise ne se supprime pas', OLD.id
                USING ERRCODE = 'restrict_violation';
        END IF;
        RETURN OLD;
    END IF;
    IF OLD.statut = 'brouillon' THEN
        RETURN NEW;
    END IF;
    IF NEW.statut = 'brouillon' THEN
        RAISE EXCEPTION 'ent_fac_facture %: retour au brouillon interdit', OLD.id
            USING ERRCODE = 'restrict_violation';
    END IF;
    IF (to_jsonb(NEW) - 'statut' - 'soldee_at') IS DISTINCT FROM (to_jsonb(OLD) - 'statut' - 'soldee_at') THEN
        RAISE EXCEPTION 'ent_fac_facture %: une pièce émise est immuable (montants, numéro, débiteur, dates)', OLD.id
            USING ERRCODE = 'restrict_violation';
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS ent_fac_facture_protege_trg ON ent_fac_facture;
CREATE TRIGGER ent_fac_facture_protege_trg
    BEFORE UPDATE OR DELETE ON ent_fac_facture
    FOR EACH ROW EXECUTE FUNCTION ent_fac_facture_protege();

-- Lignes : plus aucune écriture dès que la facture n'est plus un brouillon.
CREATE OR REPLACE FUNCTION ent_fac_ligne_protege() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_facture UUID;
    v_statut TEXT;
BEGIN
    v_facture := CASE WHEN TG_OP = 'DELETE' THEN OLD.facture_id ELSE NEW.facture_id END;
    SELECT statut INTO v_statut FROM ent_fac_facture WHERE id = v_facture;
    IF v_statut IS NOT NULL AND v_statut <> 'brouillon' THEN
        RAISE EXCEPTION 'ent_fac_ligne: la facture % est émise, ses lignes sont figées', v_facture
            USING ERRCODE = 'restrict_violation';
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END
$$;

DROP TRIGGER IF EXISTS ent_fac_ligne_protege_trg ON ent_fac_ligne;
CREATE TRIGGER ent_fac_ligne_protege_trg
    BEFORE INSERT OR UPDATE OR DELETE ON ent_fac_ligne
    FOR EACH ROW EXECUTE FUNCTION ent_fac_ligne_protege();

-- Échéances, encaissements, quittances : jamais modifiés ni supprimés
-- (une erreur se corrige par une pièce de sens contraire, pas par une réécriture).
CREATE OR REPLACE FUNCTION ent_fac_piece_figee() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '%: pièce comptable figée (% interdit)', TG_TABLE_NAME, TG_OP
        USING ERRCODE = 'restrict_violation';
END
$$;

DROP TRIGGER IF EXISTS ent_fac_echeance_figee_trg ON ent_fac_echeance;
CREATE TRIGGER ent_fac_echeance_figee_trg
    BEFORE UPDATE OR DELETE ON ent_fac_echeance
    FOR EACH ROW EXECUTE FUNCTION ent_fac_piece_figee();

DROP TRIGGER IF EXISTS ent_fac_encaissement_figee_trg ON ent_fac_encaissement;
CREATE TRIGGER ent_fac_encaissement_figee_trg
    BEFORE UPDATE OR DELETE ON ent_fac_encaissement
    FOR EACH ROW EXECUTE FUNCTION ent_fac_piece_figee();

DROP TRIGGER IF EXISTS ent_fac_quittance_figee_trg ON ent_fac_quittance;
CREATE TRIGGER ent_fac_quittance_figee_trg
    BEFORE UPDATE OR DELETE ON ent_fac_quittance
    FOR EACH ROW EXECUTE FUNCTION ent_fac_piece_figee();

-- Aucune suppression non plus pour les autres pièces et preuves financières.
CREATE OR REPLACE FUNCTION ent_fin_pas_de_suppression() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '%: suppression interdite (trace comptable)', TG_TABLE_NAME
        USING ERRCODE = 'restrict_violation';
END
$$;

DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY[
        'ent_fac_sepa_mandat', 'ent_fac_sepa_prelevement', 'ent_fac_rappel',
        'ent_fin_prenotification', 'ent_fin_rejet', 'ent_fin_reglement',
        'ent_fin_lettrage', 'ent_fin_relance', 'ent_fin_export', 'ent_fin_export_piece'
    ]
    LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', t || '_sans_suppression_trg', t);
        EXECUTE format(
            'CREATE TRIGGER %I BEFORE DELETE ON %I FOR EACH ROW EXECUTE FUNCTION ent_fin_pas_de_suppression()',
            t || '_sans_suppression_trg', t);
    END LOOP;
END
$$;

-- Garde-fous de saisie sur les lignes (FD-16). NOT VALID : l'historique n'est pas réécrit.
ALTER TABLE ent_fac_ligne DROP CONSTRAINT IF EXISTS ent_fac_ligne_bornes_chk;
ALTER TABLE ent_fac_ligne
    ADD CONSTRAINT ent_fac_ligne_bornes_chk CHECK (
        quantite <= 100000 AND abs(montant_unitaire_centimes) <= 100000000
    ) NOT VALID;

-- ── FD-10 / R-P9 : aucun prélèvement sur un mandat qui n'est pas actif ───────
CREATE OR REPLACE FUNCTION ent_fac_sepa_prelevement_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_statut TEXT;
BEGIN
    IF TG_OP = 'INSERT' THEN
        SELECT statut INTO v_statut FROM ent_fac_sepa_mandat WHERE id = NEW.mandat_id;
        IF v_statut IS DISTINCT FROM 'actif' THEN
            RAISE EXCEPTION 'prélèvement refusé : le mandat % n''est pas actif', NEW.mandat_id
                USING ERRCODE = 'restrict_violation';
        END IF;
        RETURN NEW;
    END IF;
    -- R-P6 : on n'exporte (prevu -> exporte) que ce qui a été pré-notifié.
    IF NEW.statut = 'exporte' AND OLD.statut <> 'exporte' AND NEW.prenotification_id IS NULL THEN
        RAISE EXCEPTION 'prélèvement % : export refusé sans pré-notification', OLD.id
            USING ERRCODE = 'restrict_violation';
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS ent_fac_sepa_prelevement_garde_trg ON ent_fac_sepa_prelevement;
CREATE TRIGGER ent_fac_sepa_prelevement_garde_trg
    BEFORE INSERT OR UPDATE ON ent_fac_sepa_prelevement
    FOR EACH ROW EXECUTE FUNCTION ent_fac_sepa_prelevement_garde();

-- Prélèvements encore « prévus » (jamais exportés) sur un mandat déjà révoqué : annulés.
-- Le déclencheur d'immutabilité ne touche pas cette table ; le statut `annule` existe déjà.
UPDATE ent_fac_sepa_prelevement p
   SET statut = 'annule'
  FROM ent_fac_sepa_mandat m
 WHERE m.id = p.mandat_id
   AND m.statut <> 'actif'
   AND p.statut = 'prevu';

-- ── FD-13 / R-P16 : aucun frais de recouvrement facturé ──────────────────────
UPDATE ent_fin_palier_relance SET frais_centimes = 0 WHERE frais_centimes <> 0;
UPDATE ent_fin_parametres SET frais_rejet_centimes = 0 WHERE frais_rejet_centimes <> 0;
ALTER TABLE ent_fin_palier_relance DROP CONSTRAINT IF EXISTS ent_fin_palier_relance_sans_frais_chk;
ALTER TABLE ent_fin_palier_relance
    ADD CONSTRAINT ent_fin_palier_relance_sans_frais_chk CHECK (frais_centimes = 0);
ALTER TABLE ent_fin_parametres DROP CONSTRAINT IF EXISTS ent_fin_parametres_sans_frais_chk;
ALTER TABLE ent_fin_parametres
    ADD CONSTRAINT ent_fin_parametres_sans_frais_chk CHECK (frais_rejet_centimes = 0);

-- ── FD-11 / R-P21 : l'IBAN du créancier est chiffré comme les autres champs sensibles ──
-- Identifiant de ligne (nécessaire à l'authentification des données chiffrées, `champs_chiffres`).
ALTER TABLE ent_fin_parametres ADD COLUMN IF NOT EXISTS id UUID NOT NULL DEFAULT gen_random_uuid();
CREATE UNIQUE INDEX IF NOT EXISTS ent_fin_parametres_id_uq ON ent_fin_parametres (id);
