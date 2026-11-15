-- 2310 — Socle 15 (absences des personnels), lot A7 : éléments variables de paie et suivi DSN (OGEC)
-- (`docs/socles/15_ABSENCES_PERSONNELS.md` §14.4 à §14.6 ; R-16, R-AP29, R-AP30 ; ABP-207, 209 à 211).
-- Additive et rejouable (E8) ; aucune colonne JSON (D-036) ; RLS posée (E13). Numérotation : 2309 (lot A8) reste
-- réservée ; `MIGRATION_TAG` est la plus haute du répertoire.
--
-- 1. `ent_absp_element_paie` : UN élément par (absence ou heure, mois, nature) — unicité EN BASE (R-AP29, AD-11).
--    Une période déjà envoyée reçoit un élément de RÉGULARISATION daté du mois ouvert, jamais une réécriture.
--    Nature CODÉE, jamais un libellé (AD-03) ; pas de pièce, pas de texte libre.
-- 2. `ent_absp_dsn_suivi` : l'arrêt à signaler en DSN sous cinq jours (R-16) ; la comptable coche « signalé le ».
--    ScolIA ne produit pas de DSN (socle 49).

CREATE TABLE IF NOT EXISTS ent_absp_element_paie (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    agent_id UUID NOT NULL,
    absence_id UUID,
    heure_id UUID,
    -- Premier jour du mois de PAIE auquel l'élément s'impute.
    mois DATE NOT NULL CHECK (EXTRACT(DAY FROM mois) = 1),
    nature_code TEXT NOT NULL CHECK (nature_code ~ '^[a-z][a-z0-9_]{1,39}$'),
    -- Signée pour une régularisation.
    quantite NUMERIC(7,2) NOT NULL CHECK (quantite <> 0),
    unite TEXT NOT NULL CHECK (unite IN ('heure', 'jour', 'demi_jour')),
    etat TEXT NOT NULL DEFAULT 'prepare' CHECK (etat IN ('prepare', 'envoye', 'regularise')),
    -- Régularisation : l'élément déjà envoyé qu'elle corrige.
    regularise_de_id UUID,
    envoye_le TIMESTAMPTZ,
    envoye_par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_element_paie_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_element_paie_agent_fk FOREIGN KEY (etablissement_id, agent_id)
        REFERENCES ent_agent (etablissement_id, id),
    CONSTRAINT ent_absp_element_paie_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_absp_absence (etablissement_id, id),
    CONSTRAINT ent_absp_element_paie_heure_fk FOREIGN KEY (etablissement_id, heure_id)
        REFERENCES ent_absp_heure (etablissement_id, id),
    CONSTRAINT ent_absp_element_paie_regul_fk FOREIGN KEY (etablissement_id, regularise_de_id)
        REFERENCES ent_absp_element_paie (etablissement_id, id),
    CONSTRAINT ent_absp_element_paie_source_chk CHECK ((absence_id IS NULL) <> (heure_id IS NULL)),
    CONSTRAINT ent_absp_element_paie_envoi_chk CHECK ((etat = 'prepare') = (envoye_le IS NULL))
);
-- R-AP29 : un élément par (absence ou heure, mois, nature).
CREATE UNIQUE INDEX IF NOT EXISTS ent_absp_element_paie_unique
    ON ent_absp_element_paie (etablissement_id, COALESCE(absence_id, heure_id), mois, nature_code);
CREATE INDEX IF NOT EXISTS ent_absp_element_paie_mois_idx ON ent_absp_element_paie (etablissement_id, mois, etat);

-- Un élément envoyé ne se réécrit pas : seuls l'envoi (prepare → envoye) est permis, jamais la quantité.
CREATE OR REPLACE FUNCTION ent_trg_absp_element_paie_garde() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF OLD.etat <> 'prepare' THEN
            RAISE EXCEPTION 'ent_absp_element_paie : un élément envoyé ne se supprime pas';
        END IF;
        RETURN OLD;
    END IF;
    IF NEW.id <> OLD.id OR NEW.etablissement_id <> OLD.etablissement_id OR NEW.agent_id <> OLD.agent_id
       OR NEW.absence_id IS DISTINCT FROM OLD.absence_id OR NEW.heure_id IS DISTINCT FROM OLD.heure_id
       OR NEW.mois <> OLD.mois OR NEW.nature_code <> OLD.nature_code THEN
        RAISE EXCEPTION 'ent_absp_element_paie : identité de l''élément immuable';
    END IF;
    IF OLD.etat <> 'prepare' AND (NEW.quantite <> OLD.quantite OR NEW.unite <> OLD.unite OR NEW.etat = 'prepare') THEN
        RAISE EXCEPTION 'ent_absp_element_paie : un élément envoyé ne se réécrit pas (régularisation sur le mois ouvert)';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absp_element_paie_garde ON ent_absp_element_paie;
CREATE TRIGGER ent_absp_element_paie_garde BEFORE UPDATE OR DELETE ON ent_absp_element_paie
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_element_paie_garde();

CREATE TABLE IF NOT EXISTS ent_absp_dsn_suivi (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    absence_id UUID NOT NULL,
    -- J+5 à compter de la connaissance de l'arrêt (R-16), jour de l'établissement.
    echeance_le DATE NOT NULL,
    signale_le DATE,
    signale_par_user_id UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_dsn_suivi_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_dsn_suivi_absence_uq UNIQUE (etablissement_id, absence_id),
    CONSTRAINT ent_absp_dsn_suivi_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_absp_absence (etablissement_id, id),
    CONSTRAINT ent_absp_dsn_suivi_signale_chk CHECK ((signale_le IS NULL) = (signale_par_user_id IS NULL))
);
CREATE INDEX IF NOT EXISTS ent_absp_dsn_suivi_echeance_idx
    ON ent_absp_dsn_suivi (etablissement_id, echeance_le) WHERE signale_le IS NULL;

SELECT ent_rls_poser(ARRAY['ent_absp_element_paie', 'ent_absp_dsn_suivi']);
