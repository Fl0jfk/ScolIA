-- Messagerie, lot M0a (`docs/socles/16_MESSAGERIE.md` §2.2 D1 à D10, §27) : correctifs urgents
-- des fuites, sans changer le modèle. Plage de repli 480–499 (Q-B2 non tranchée).
--
-- 1. Vues « communicant » et « sollicitable » : l'interdiction de communication (271) et
--    « ne pas solliciter » (260) relues à chaque lecture, écriture et envoi (D1, D6).
-- 2. `ent_msg_fil.audience` et `ent_msg_fil.vie_scolaire` (D3, D5) ; reprise des fils ouverts
--    par un élève (privés à l'élève, invisibles des parents).
-- 3. RLS permissive sur toutes les tables de communication (D10) ; la stricte viendra en M17.

-- 1. Liens qui peuvent encore recevoir ou lire une communication à propos de l'élève ---------
-- Une interdiction de communication en cours (levée ou échue : sans effet) vise une personne ;
-- la personne fusionnée dans le lien reste visée (`fusionnee_dans`).
-- Définie sur les tables (même critère que `ent_eleve_lien_actif`, migration 265) et non sur la
-- vue : les migrations 121 et 260 font `DROP VIEW ent_eleve_lien_actif`, qu'une vue dépendante
-- empêcherait à chaque rejeu des migrations.
CREATE OR REPLACE VIEW ent_eleve_lien_communicant AS
SELECT l.*
FROM ent_eleve_lien l
JOIN ent_personne p
  ON p.id = l.personne_id AND p.etablissement_id = l.etablissement_id
WHERE l.suspendu_le IS NULL
  AND (l.date_debut IS NULL OR l.date_debut <= ent_aujourdhui_etab(l.etablissement_id))
  AND (l.date_fin IS NULL OR l.date_fin >= ent_aujourdhui_etab(l.etablissement_id))
  AND p.fusionnee_dans IS NULL
  AND NOT EXISTS (
    SELECT 1
      FROM ent_eleve_interdiction i
     WHERE i.etablissement_id = l.etablissement_id
       AND i.eleve_id = l.eleve_id
       AND i.levee_le IS NULL
       AND 'communication' = ANY (i.objet)
       AND i.du <= ent_aujourdhui_etab(l.etablissement_id)
       AND (i.au IS NULL OR i.au >= ent_aujourdhui_etab(l.etablissement_id))
       AND i.personne_id IS NOT NULL
       AND (i.personne_id = l.personne_id
            OR i.personne_id IN (SELECT s.id FROM ent_personne s
                                  WHERE s.etablissement_id = l.etablissement_id
                                    AND s.fusionnee_dans = l.personne_id)));

-- Idem, hors personnes « ne pas solliciter » : pour tout envoi qui ouvre un contact
-- (push, annonce, campagne, rencontre). La réponse dans un fil ouvert par la personne
-- reste permise par `ent_eleve_lien_communicant`.
CREATE OR REPLACE VIEW ent_eleve_lien_sollicitable AS
SELECT c.*
FROM ent_eleve_lien_communicant c
WHERE NOT EXISTS (
    SELECT 1 FROM ent_personne_reserve nps
     WHERE nps.etablissement_id = c.etablissement_id
       AND nps.personne_id = c.personne_id
       AND nps.ne_pas_solliciter);

-- 2. Audience du fil (D3) et nature « vie scolaire » (D5) --------------------------------------
--   'responsables' : parents (selon leurs droits) et personnel ; jamais l'élève (défaut).
--   'famille'      : parents et élève.
--   'eleve'        : l'élève et le personnel ; jamais un parent.
ALTER TABLE ent_msg_fil ADD COLUMN IF NOT EXISTS audience TEXT NOT NULL DEFAULT 'responsables';
ALTER TABLE ent_msg_fil ADD COLUMN IF NOT EXISTS vie_scolaire BOOLEAN NOT NULL DEFAULT false;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_fil_audience_chk') THEN
        ALTER TABLE ent_msg_fil ADD CONSTRAINT ent_msg_fil_audience_chk
            CHECK (audience IN ('famille', 'responsables', 'eleve'));
    END IF;
END $$;

-- Fils ouverts par le compte d'un élève : privés à l'élève (et au personnel).
UPDATE ent_msg_fil f
   SET audience = 'eleve', prive_user_id = f.created_by_user_id
 WHERE f.prive_user_id IS NULL AND f.foyer_id IS NULL
   AND EXISTS (SELECT 1 FROM ent_membership m
                WHERE m.user_id = f.created_by_user_id
                  AND m.etablissement_id = f.etablissement_id AND m.role = 'eleve');

-- Fils de vie scolaire : alerte de seuil d'absences et carnet de correspondance.
UPDATE ent_msg_fil f SET vie_scolaire = true
 WHERE EXISTS (SELECT 1 FROM ent_carnet_entree c
                WHERE c.etablissement_id = f.etablissement_id AND c.fil_notif_id = f.id)
    OR EXISTS (SELECT 1 FROM ent_absence_signal s
                WHERE s.etablissement_id = f.etablissement_id AND s.fil_alerte_id = f.id);

-- 3. RLS permissive (politique des migrations 100 et 215) ---------------------------------------
DO $$
DECLARE
    t text;
    tables text[] := ARRAY[
        'ent_msg_fil', 'ent_msg', 'ent_msg_accuse', 'ent_msg_piece', 'ent_msg_modele',
        'ent_carnet_entree', 'ent_annonce',
        'ent_preference_notification', 'ent_notification_plafond', 'ent_notification_envoi',
        'ent_campagne', 'ent_campagne_destinataire',
        'ent_rencontre', 'ent_creneau_rdv',
        'ent_formulaire', 'ent_formulaire_reponse',
        'ent_notification'];
BEGIN
    FOREACH t IN ARRAY tables LOOP
        IF EXISTS (SELECT 1 FROM information_schema.columns c
                    WHERE c.table_schema = current_schema() AND c.table_name = t
                      AND c.column_name = 'etablissement_id') THEN
            EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
            EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
            EXECUTE format('DROP POLICY IF EXISTS ent_rls_etablissement ON %I', t);
            EXECUTE format(
                'CREATE POLICY ent_rls_etablissement ON %I FOR ALL '
                'USING (etablissement_id = (SELECT ent_rls_etablissement()) '
                '  OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict()))) '
                'WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement()) '
                '  OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))',
                t);
        END IF;
    END LOOP;
END
$$;
