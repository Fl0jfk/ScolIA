-- Socle 41, lot I4, suites (passe 3) : contraintes de 2828 posées en ligne, donc jamais mises à jour au rejeu
-- (`CREATE TABLE IF NOT EXISTS` ne retouche pas une table existante). 2828 et 2829 sont versées et ne changent pas
-- (règle 25) : on remplace ici les deux CHECK par des contraintes NOMMÉES, couvrant toutes les valeurs du code
-- (modèle : 654:12-27). Rejouable : retrait de toute contrainte de contrôle portant sur la colonne (nom automatique de
-- l'ancienne, ou nom propre d'un rejeu), puis ajout de la contrainte nommée.
--
-- * `motif_code` du participant : la valeur `accord_annuel_tombe` (F-a, passe 2) manquait aux bases migrées par la version
--   de 2828 d'avant la passe 2 ; l'accord annuel « tombé » y violait le CHECK (23514).
-- * `acte` du journal des faits : ajout de `refus_apres_depart` (F-g, D-069 : un refus reçu après le départ est un fait
--   tracé, il n'annule pas le départ).
-- Aucune table, donc ni RLS, ni registre des données, ni SOURCES à recalculer.

DO $$
DECLARE
    c TEXT;
BEGIN
    FOR c IN SELECT conname FROM pg_constraint
              WHERE conrelid = 'ent_internat_sortie_groupe_participant'::regclass AND contype = 'c'
                AND pg_get_constraintdef(oid) LIKE '%motif_code%' LOOP
        EXECUTE format('ALTER TABLE ent_internat_sortie_groupe_participant DROP CONSTRAINT IF EXISTS %I', c);
    END LOOP;
    ALTER TABLE ent_internat_sortie_groupe_participant DROP CONSTRAINT IF EXISTS ent_internat_sg_part_motif_code_chk;
    ALTER TABLE ent_internat_sortie_groupe_participant ADD CONSTRAINT ent_internat_sg_part_motif_code_chk CHECK (
        motif_code IS NULL OR motif_code IN (
            'refus_famille', 'desaccord_connu', 'interdiction', 'plus_interne', 'retire_par_organisateur', 'majeur_refuse',
            'majorite', 'chevauchement', 'accord_annuel_tombe'));

    FOR c IN SELECT conname FROM pg_constraint
              WHERE conrelid = 'ent_internat_sortie_groupe_fait'::regclass AND contype = 'c'
                AND pg_get_constraintdef(oid) LIKE '%acte%' LOOP
        EXECUTE format('ALTER TABLE ent_internat_sortie_groupe_fait DROP CONSTRAINT IF EXISTS %I', c);
    END LOOP;
    ALTER TABLE ent_internat_sortie_groupe_fait DROP CONSTRAINT IF EXISTS ent_internat_sg_fait_acte_chk;
    ALTER TABLE ent_internat_sortie_groupe_fait ADD CONSTRAINT ent_internat_sg_fait_acte_chk CHECK (acte IN (
        'creation', 'duplication', 'modification', 'soumission', 'acceptation', 'refus_direction', 'annulation',
        'invitation', 'retrait', 'accord_famille', 'refus_famille', 'accord_annuel', 'accord_hors_ligne',
        'blocage_desaccord', 'blocage_interdiction', 'accord_tombe', 'depart', 'retour', 'cloture', 'encadrant_ajoute',
        'encadrant_retire', 'blocage_plus_interne', 'blocage_majorite', 'blocage_chevauchement', 'retour_annule',
        'refus_hors_ligne', 'encadrant_remplace', 'encadrant_present', 'nouvelle_decision', 'refus_apres_depart'));
END
$$;
