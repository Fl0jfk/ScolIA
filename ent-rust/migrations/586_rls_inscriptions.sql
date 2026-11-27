-- 586 — RLS d'établissement, lot « inscriptions » (26 tables). Même politique que le pilote
-- (migration 100, docs/RLS_ISOLATION.md) : FORCE ROW LEVEL SECURITY + politique
-- ent_rls_etablissement (variable app.etablissement_id posée par TenantTx ; sans
-- variable : permissif tant que app.rls_strict n'est pas actif). Idempotent (rejoué à chaque
-- démarrage). Pose via ent_rls_poser (définie en 580).

-- E6 : ent_inscription_responsable n'avait aucun horodatage propre. Ajout une seule fois
-- (idempotent), valeur reprise du dossier parent pour les lignes existantes.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                    WHERE table_schema = current_schema()
                      AND table_name = 'ent_inscription_responsable' AND column_name = 'cree_le') THEN
        ALTER TABLE ent_inscription_responsable ADD COLUMN cree_le TIMESTAMPTZ;
        UPDATE ent_inscription_responsable r SET cree_le = d.created_at
          FROM ent_inscription_dossier d WHERE d.id = r.dossier_id;
        UPDATE ent_inscription_responsable SET cree_le = now() WHERE cree_le IS NULL;
        ALTER TABLE ent_inscription_responsable ALTER COLUMN cree_le SET DEFAULT now();
        ALTER TABLE ent_inscription_responsable ALTER COLUMN cree_le SET NOT NULL;
    END IF;
END
$$;

SELECT ent_rls_poser(ARRAY[
        'ent_inscription_campagne',
        'ent_inscription_campagne_niveau',
        'ent_inscription_campagne_piece',
        'ent_inscription_campagne_version',
        'ent_inscription_decision',
        'ent_inscription_declaration_responsable',
        'ent_inscription_demarche',
        'ent_inscription_dossier',
        'ent_inscription_fermeture',
        'ent_inscription_lien_attente',
        'ent_inscription_offre',
        'ent_inscription_offre_historique',
        'ent_inscription_offre_unite',
        'ent_inscription_parametre_lien',
        'ent_inscription_piece',
        'ent_inscription_prolongation',
        'ent_inscription_reference_compteur',
        'ent_inscription_reprise_rapport',
        'ent_inscription_responsable',
        'ent_inscription_suivi',
        'ent_inscription_suivi_dossier',
        'ent_inscription_voeu',
        'ent_lien_attente',
        'ent_reinscription_campagne',
        'ent_reinscription_exigence_doc',
        'ent_voeu_inscription'
]);
