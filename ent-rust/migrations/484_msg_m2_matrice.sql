-- Messagerie, lot M2 (`docs/socles/16_MESSAGERIE.md` §8, §9, R-M3, R-M5 à R-M8, R-M12) : matrice
-- « qui peut écrire à qui » et destinataires. Suite de 482 et 483 ; plage de repli 480–499.
--
-- Les **sept réglages bornés** de l'établissement (§8.3, S-132, D-023) deviennent des colonnes
-- typées de `ent_msg_reglage` (D-036 : jamais une liste de rôles en JSON, comme dans l'existant
-- Next.js `famille_messaging_settings.roles_can_initiate`). Chaque réglage a une valeur par défaut
-- (§8.2) ; la matrice, elle, est **calculée à partir des faits** (lien d'enseignement, lien
-- familial, mission) et n'est jamais saisie.
--
-- L'ancienne valeur d'un réglage est conservée à chaque modification (`ent_msg_reglage_version`,
-- ajout seul) : on sait ce qui était en vigueur à une date donnée.

ALTER TABLE ent_msg_reglage ADD COLUMN IF NOT EXISTS familles_vers_professeurs BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE ent_msg_reglage ADD COLUMN IF NOT EXISTS familles_vers_direction BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE ent_msg_reglage ADD COLUMN IF NOT EXISTS eleves_vers_professeurs BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE ent_msg_reglage ADD COLUMN IF NOT EXISTS eleves_vers_eleves TEXT NOT NULL DEFAULT 'non';
ALTER TABLE ent_msg_reglage ADD COLUMN IF NOT EXISTS aesh_vers_familles TEXT NOT NULL DEFAULT 'repondre';
ALTER TABLE ent_msg_reglage ADD COLUMN IF NOT EXISTS secretariat_vers_majeurs BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE ent_msg_reglage ADD COLUMN IF NOT EXISTS pieces_familles BOOLEAN NOT NULL DEFAULT true;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_reglage_eleves_chk') THEN
        ALTER TABLE ent_msg_reglage ADD CONSTRAINT ent_msg_reglage_eleves_chk
            CHECK (eleves_vers_eleves IN ('non', 'delegues', 'classe'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_reglage_aesh_chk') THEN
        ALTER TABLE ent_msg_reglage ADD CONSTRAINT ent_msg_reglage_aesh_chk
            CHECK (aesh_vers_familles IN ('non', 'repondre', 'ouvrir'));
    END IF;
END $$;

-- Historique : une ligne par valeur remplacée (ajout seul).
CREATE TABLE IF NOT EXISTS ent_msg_reglage_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    familles_vers_professeurs BOOLEAN NOT NULL,
    familles_vers_direction BOOLEAN NOT NULL,
    eleves_vers_professeurs BOOLEAN NOT NULL,
    eleves_vers_eleves TEXT NOT NULL CHECK (eleves_vers_eleves IN ('non', 'delegues', 'classe')),
    aesh_vers_familles TEXT NOT NULL CHECK (aesh_vers_familles IN ('non', 'repondre', 'ouvrir')),
    secretariat_vers_majeurs BOOLEAN NOT NULL,
    pieces_familles BOOLEAN NOT NULL,
    plage_calme BOOLEAN NOT NULL,
    modifie_par UUID,
    en_vigueur_depuis TIMESTAMPTZ NOT NULL,
    remplace_le TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_msg_reglage_version_idx
    ON ent_msg_reglage_version (etablissement_id, remplace_le);

CREATE OR REPLACE FUNCTION ent_msg_reglage_historique() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF (OLD.familles_vers_professeurs, OLD.familles_vers_direction, OLD.eleves_vers_professeurs,
        OLD.eleves_vers_eleves, OLD.aesh_vers_familles, OLD.secretariat_vers_majeurs, OLD.pieces_familles,
        OLD.plage_calme)
       IS DISTINCT FROM
       (NEW.familles_vers_professeurs, NEW.familles_vers_direction, NEW.eleves_vers_professeurs,
        NEW.eleves_vers_eleves, NEW.aesh_vers_familles, NEW.secretariat_vers_majeurs, NEW.pieces_familles,
        NEW.plage_calme) THEN
        INSERT INTO ent_msg_reglage_version
            (id, etablissement_id, familles_vers_professeurs, familles_vers_direction,
             eleves_vers_professeurs, eleves_vers_eleves, aesh_vers_familles, secretariat_vers_majeurs,
             pieces_familles, plage_calme, modifie_par, en_vigueur_depuis)
        VALUES (gen_random_uuid(), OLD.etablissement_id, OLD.familles_vers_professeurs,
                OLD.familles_vers_direction, OLD.eleves_vers_professeurs, OLD.eleves_vers_eleves,
                OLD.aesh_vers_familles, OLD.secretariat_vers_majeurs, OLD.pieces_familles,
                OLD.plage_calme, OLD.modifie_par, OLD.modifie_le);
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_msg_reglage_historique_trg ON ent_msg_reglage;
CREATE TRIGGER ent_msg_reglage_historique_trg BEFORE UPDATE ON ent_msg_reglage
    FOR EACH ROW EXECUTE FUNCTION ent_msg_reglage_historique();

DROP TRIGGER IF EXISTS ent_msg_reglage_version_ajout_seul ON ent_msg_reglage_version;
CREATE TRIGGER ent_msg_reglage_version_ajout_seul BEFORE UPDATE OR DELETE ON ent_msg_reglage_version
    FOR EACH ROW EXECUTE FUNCTION ent_msg_ajout_seul();

-- RLS permissive (même politique que 480 et 482).
ALTER TABLE ent_msg_reglage_version ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_msg_reglage_version FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_msg_reglage_version;
CREATE POLICY ent_rls_etablissement ON ent_msg_reglage_version FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));

-- Majorité (R-M12, raccord socle 4) ---------------------------------------------------------------
-- À la majorité d'un élève, les responsables sortent des fils que l'établissement leur avait
-- ouverts **à propos de lui** (nature « famille », ouverts par un personnel, `origine` personnel) ;
-- ils gardent ceux qu'ils ont ouverts eux-mêmes (leur propre échange avec l'établissement) et les
-- relances de facture dont ils sont payeurs. Aucun droit conservé du socle 4 n'existe encore
-- (Q-A7) : quand il existera, ce code lira ces droits au lieu de tout retirer. Motif codé
-- `majorite`, jamais un texte. Rend le nombre de participations closes.
CREATE OR REPLACE FUNCTION ent_msg_majorite_eleve(p_etab UUID, p_eleve UUID) RETURNS INTEGER
LANGUAGE plpgsql AS $$
DECLARE
    k INTEGER;
BEGIN
    UPDATE ent_msg_participant p
       SET au = now(), motif_sortie = 'majorite'
     WHERE p.etablissement_id = p_etab AND p.au IS NULL AND p.cote = 'famille'
       AND p.fil_id IN (SELECT f.id FROM ent_msg_fil f
                         WHERE f.etablissement_id = p_etab AND f.eleve_id = p_eleve
                           AND f.nature = 'famille' AND f.origine = 'personnel'
                           AND f.foyer_id IS NULL);
    GET DIAGNOSTICS k = ROW_COUNT;
    RETURN k;
END
$$;
