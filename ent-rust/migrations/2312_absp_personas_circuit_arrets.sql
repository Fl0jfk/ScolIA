-- 2312 — Socle 15 (absences des personnels), lot A1, passe personas : arrêts, décision d'un employeur
-- externe, paramètres de confirmation (`15_ABSENCES_PERSONNELS.md` §32.1, §32.6, §32.7).
-- Additive et rejouable (E8) ; aucune colonne JSON (D-036) ; RLS posée.
--
-- Le circuit EXTERNE (R-AP47) : pour un agent de l'État ou un maître, l'établissement PREND EN COMPTE et
-- transmet, l'employeur (rectorat, DSDEN) décide. Les états `prise_en_compte` / `decision_employeur`
-- sont déjà permis par `ent_absp_absence.etat` (2302) ; cette migration ajoute les tables qui les portent.

-- 1. Arrêts : une ligne par arrêt (initial, prolongation, rechute) ; date de réception, échéance 48 h.
CREATE TABLE IF NOT EXISTS ent_absp_arret (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    absence_id UUID NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('initial', 'prolongation', 'rechute')),
    du DATE NOT NULL,
    au DATE NOT NULL,
    recu_le TIMESTAMPTZ,
    recu_par_user_id UUID,
    -- R-AP16 : échéance de transmission affichée à l'instructeur ; ScolIA ne sanctionne pas.
    echeance_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_par_user_id UUID,
    CONSTRAINT ent_absp_arret_dates_chk CHECK (au >= du),
    CONSTRAINT ent_absp_arret_recu_chk CHECK ((recu_le IS NULL) = (recu_par_user_id IS NULL)),
    CONSTRAINT ent_absp_arret_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_absp_arret_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_absp_absence (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_absp_arret_absence_idx ON ent_absp_arret (etablissement_id, absence_id, du);

-- 2. Décision de l'employeur externe (ajout seul, la dernière fait foi).
CREATE TABLE IF NOT EXISTS ent_absp_decision_externe (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    absence_id UUID NOT NULL,
    employeur TEXT NOT NULL CHECK (employeur IN ('rectorat', 'dsden', 'employeur_mutualisateur', 'autre')),
    decision TEXT NOT NULL CHECK (decision IN ('accordee', 'refusee', 'requalifiee')),
    nature_apres_id UUID REFERENCES ent_absp_nature (id),
    le DATE NOT NULL,
    saisie_par_user_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_absp_decision_externe_requalif_chk CHECK (decision <> 'requalifiee' OR nature_apres_id IS NOT NULL),
    CONSTRAINT ent_absp_decision_externe_absence_fk FOREIGN KEY (etablissement_id, absence_id)
        REFERENCES ent_absp_absence (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_absp_decision_externe_idx
    ON ent_absp_decision_externe (etablissement_id, absence_id, le);

CREATE OR REPLACE FUNCTION ent_trg_absp_decision_externe_ajout_seul() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION '% est en ajout seul (% interdit)', TG_TABLE_NAME, TG_OP;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_absp_decision_externe_ajout_seul ON ent_absp_decision_externe;
CREATE TRIGGER ent_absp_decision_externe_ajout_seul BEFORE UPDATE OR DELETE ON ent_absp_decision_externe
    FOR EACH ROW EXECUTE FUNCTION ent_trg_absp_decision_externe_ajout_seul();

-- 3. Paramètres (R-AP45, R-AP46, R-AP55).
-- EDT de référence : conditionne TOUT effet vers les familles et les élèves et toute proposition de
-- sortie anticipée (ABP-272). Éteint par défaut : pendant le doublon 2027-2028, l'EDT officiel peut
-- rester celui d'un autre outil (Q-AP-F3).
ALTER TABLE ent_absp_param ADD COLUMN IF NOT EXISTS edt_reference BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_absp_param ADD COLUMN IF NOT EXISTS edt_reference_pose_le TIMESTAMPTZ;
ALTER TABLE ent_absp_param ADD COLUMN IF NOT EXISTS edt_reference_pose_par_user_id UUID;
-- Jour du mois de la clôture des variables de paie (R-AP55).
ALTER TABLE ent_absp_param ADD COLUMN IF NOT EXISTS jour_cloture_variables SMALLINT NOT NULL DEFAULT 25
    CHECK (jour_cloture_variables BETWEEN 1 AND 28);
-- Conservation des zones de qualification issues de la reprise, au plus 60 jours (S-1521).
ALTER TABLE ent_absp_param ADD COLUMN IF NOT EXISTS reprise_conservation_jours SMALLINT NOT NULL DEFAULT 60
    CHECK (reprise_conservation_jours BETWEEN 1 AND 60);

SELECT ent_rls_poser(ARRAY['ent_absp_arret', 'ent_absp_decision_externe']);
