-- Socle 4, lot A0 (`docs/socles/04_ADMINISTRATIF.md` §16, étape 260 et §2.2 pts 1, 4, 12, 16, 17).
--
-- 1. Fuseau : les dates métier des liens sont calculées en Europe/Paris (et non en UTC).
--    Un lien qui finit le 31 août reste actif jusqu'à minuit heure de Paris.
-- 2. Clés composées (établissement, id) sur personnes, foyers, liens ; clés étrangères
--    composites `NOT VALID` puis validées SEULEMENT s'il n'y a aucun orphelin (sinon elles
--    restent non validées et les orphelins sont listés par la vue de contrôle : rien n'est supprimé).
-- 3. Lien EN ATTENTE (invitation) : un lien vers une personne qui a un compte n'ouvre aucun
--    accès tant que cette personne ne l'a pas confirmé. Table du socle 4, utilisée par les
--    inscriptions (source `inscription`) et par la saisie directe (source `saisie`).
-- 4. Préférences de canal de la personne (sous-ensemble de `ent_personne_preference`, §8.4,
--    étendu plus tard par la migration 269) : lues par les envois aux familles.
-- Migration additive et rejouable.

-- 1. Jour métier --------------------------------------------------------------
CREATE OR REPLACE FUNCTION ent_aujourdhui(instant TIMESTAMPTZ DEFAULT now())
RETURNS DATE
LANGUAGE sql STABLE
AS $$ SELECT (instant AT TIME ZONE 'Europe/Paris')::date $$;

DROP VIEW IF EXISTS ent_eleve_lien_actif;
CREATE VIEW ent_eleve_lien_actif AS
SELECT l.*
FROM ent_eleve_lien l
JOIN ent_personne p
  ON p.id = l.personne_id AND p.etablissement_id = l.etablissement_id
WHERE l.suspendu_le IS NULL
  AND (l.date_debut IS NULL OR l.date_debut <= ent_aujourdhui())
  AND (l.date_fin IS NULL OR l.date_fin >= ent_aujourdhui())
  AND p.fusionnee_dans IS NULL;

-- 2. Clés composées -----------------------------------------------------------
CREATE UNIQUE INDEX IF NOT EXISTS ent_personne_etab_id_uq ON ent_personne (etablissement_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_foyer_etab_id_uq ON ent_foyer (etablissement_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_eleve_lien_etab_id_uq ON ent_eleve_lien (etablissement_id, id);

-- Orphelins : lignes dont la personne appartient à un autre établissement (ou n'existe pas).
CREATE OR REPLACE VIEW ent_dossier_orphelins_v AS
SELECT 'ent_eleve_lien'::text AS table_source, l.id, l.etablissement_id, l.personne_id
  FROM ent_eleve_lien l
 WHERE l.personne_id IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM ent_personne p
                    WHERE p.id = l.personne_id AND p.etablissement_id = l.etablissement_id)
UNION ALL
SELECT 'ent_eleve_contact_urgence', c.id, c.etablissement_id, c.personne_id
  FROM ent_eleve_contact_urgence c
 WHERE NOT EXISTS (SELECT 1 FROM ent_personne p
                    WHERE p.id = c.personne_id AND p.etablissement_id = c.etablissement_id)
UNION ALL
SELECT 'ent_personne_fusion', f.id, f.etablissement_id, f.source_id
  FROM ent_personne_fusion f
 WHERE NOT EXISTS (SELECT 1 FROM ent_personne p
                    WHERE p.id = f.source_id AND p.etablissement_id = f.etablissement_id)
    OR NOT EXISTS (SELECT 1 FROM ent_personne p
                    WHERE p.id = f.cible_id AND p.etablissement_id = f.etablissement_id);

DO $$
DECLARE
    orphelins BIGINT;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_eleve_lien_personne_etab_fk') THEN
        ALTER TABLE ent_eleve_lien ADD CONSTRAINT ent_eleve_lien_personne_etab_fk
            FOREIGN KEY (etablissement_id, personne_id) REFERENCES ent_personne (etablissement_id, id) NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_eleve_contact_urgence_personne_etab_fk') THEN
        ALTER TABLE ent_eleve_contact_urgence ADD CONSTRAINT ent_eleve_contact_urgence_personne_etab_fk
            FOREIGN KEY (etablissement_id, personne_id) REFERENCES ent_personne (etablissement_id, id) NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_personne_fusion_source_etab_fk') THEN
        ALTER TABLE ent_personne_fusion ADD CONSTRAINT ent_personne_fusion_source_etab_fk
            FOREIGN KEY (etablissement_id, source_id) REFERENCES ent_personne (etablissement_id, id) NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_personne_fusion_cible_etab_fk') THEN
        ALTER TABLE ent_personne_fusion ADD CONSTRAINT ent_personne_fusion_cible_etab_fk
            FOREIGN KEY (etablissement_id, cible_id) REFERENCES ent_personne (etablissement_id, id) NOT VALID;
    END IF;
    SELECT count(*) INTO orphelins FROM ent_dossier_orphelins_v;
    IF orphelins = 0 THEN
        ALTER TABLE ent_eleve_lien VALIDATE CONSTRAINT ent_eleve_lien_personne_etab_fk;
        ALTER TABLE ent_eleve_contact_urgence VALIDATE CONSTRAINT ent_eleve_contact_urgence_personne_etab_fk;
        ALTER TABLE ent_personne_fusion VALIDATE CONSTRAINT ent_personne_fusion_source_etab_fk;
        ALTER TABLE ent_personne_fusion VALIDATE CONSTRAINT ent_personne_fusion_cible_etab_fk;
    ELSE
        RAISE NOTICE 'ent_dossier_orphelins_v : % ligne(s) orpheline(s), clés composées laissées NOT VALID', orphelins;
    END IF;
END $$;

-- 3. Liens en attente de confirmation par la personne ---------------------------
CREATE TABLE IF NOT EXISTS ent_lien_attente (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id) ON DELETE CASCADE,
    personne_id UUID NOT NULL REFERENCES ent_personne (id) ON DELETE CASCADE,
    foyer_id UUID REFERENCES ent_foyer (id) ON DELETE SET NULL,
    lien TEXT NOT NULL CHECK (length(btrim(lien)) BETWEEN 1 AND 40),
    autorite_parentale BOOLEAN NOT NULL,
    garde TEXT CHECK (garde IS NULL OR garde IN ('exclusive', 'alternee', 'aucune')),
    date_debut DATE,
    date_fin DATE,
    peut_voir_notes BOOLEAN NOT NULL,
    peut_voir_vs BOOLEAN NOT NULL,
    peut_justifier_absence BOOLEAN NOT NULL,
    peut_autoriser_sorties BOOLEAN NOT NULL,
    est_payeur BOOLEAN NOT NULL DEFAULT false,
    part_paiement SMALLINT CHECK (part_paiement IS NULL OR part_paiement BETWEEN 0 AND 100),
    -- Qui a demandé le lien : `inscription` (source_ref = responsable déclaré) ou `saisie`.
    source TEXT NOT NULL CHECK (source IN ('inscription', 'saisie')),
    source_ref UUID,
    demande_par UUID,
    statut TEXT NOT NULL DEFAULT 'en_attente'
        CHECK (statut IN ('en_attente', 'confirme', 'refuse', 'annule')),
    decide_le TIMESTAMPTZ,
    decide_par UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_lien_attente_dates_chk CHECK (date_debut IS NULL OR date_fin IS NULL OR date_fin >= date_debut)
);
-- Une seule demande en attente par (élève, personne).
CREATE UNIQUE INDEX IF NOT EXISTS ent_lien_attente_ouverte_uq
    ON ent_lien_attente (etablissement_id, eleve_id, personne_id) WHERE statut = 'en_attente';
CREATE INDEX IF NOT EXISTS ent_lien_attente_personne_idx
    ON ent_lien_attente (etablissement_id, personne_id, statut);
CREATE INDEX IF NOT EXISTS ent_lien_attente_source_idx
    ON ent_lien_attente (etablissement_id, source, source_ref);

-- Reprise des liens en attente de la branche I0 (table `ent_inscription_lien_attente`).
DO $$
BEGIN
    IF to_regclass('ent_inscription_lien_attente') IS NOT NULL THEN
        INSERT INTO ent_lien_attente (id, etablissement_id, eleve_id, personne_id, foyer_id, lien,
                                      autorite_parentale, peut_voir_notes, peut_voir_vs,
                                      peut_justifier_absence, peut_autoriser_sorties, est_payeur,
                                      part_paiement, source, source_ref, statut, decide_le, created_at)
        SELECT a.id, a.etablissement_id, a.eleve_id, a.personne_id, a.foyer_id, a.lien,
               a.autorite_parentale, a.peut_voir_notes, a.peut_voir_vs, a.peut_justifier_absence,
               a.peut_autoriser_sorties, a.est_payeur, a.part_paiement, 'inscription',
               a.responsable_id, a.statut, a.decide_le, a.created_at
          FROM ent_inscription_lien_attente a
         WHERE NOT EXISTS (SELECT 1 FROM ent_lien_attente x WHERE x.id = a.id)
           AND (a.statut <> 'en_attente' OR NOT EXISTS (
                 SELECT 1 FROM ent_lien_attente y
                  WHERE y.etablissement_id = a.etablissement_id AND y.eleve_id = a.eleve_id
                    AND y.personne_id = a.personne_id AND y.statut = 'en_attente'));
    END IF;
END $$;

-- 4. Préférences de canal de la personne ----------------------------------------
CREATE TABLE IF NOT EXISTS ent_personne_preference (
    personne_id UUID PRIMARY KEY REFERENCES ent_personne (id) ON DELETE CASCADE,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    canal_email BOOLEAN NOT NULL DEFAULT true,
    canal_sms BOOLEAN NOT NULL DEFAULT true,
    courrier_papier BOOLEAN NOT NULL DEFAULT false,
    updated_by UUID,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ent_personne_preference_etab_idx ON ent_personne_preference (etablissement_id);
