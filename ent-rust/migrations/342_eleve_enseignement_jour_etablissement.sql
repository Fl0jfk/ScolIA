-- Jour d'établissement dans la dernière fonction SQL du socle élève qui lisait `current_date` : celle de 340
-- (suite de 295 et 296). Elle ne peut pas être redéfinie par 296 : sur une base neuve, 340 s'applique après 296
-- et réécrirait l'ancienne définition. Même définition que 340, avec le jour de l'ÉTABLISSEMENT
-- (migration 265 : ent_aujourdhui_etab). Idempotent.
-- 340 : enseignement repris depuis une option d'année
CREATE OR REPLACE FUNCTION ent_eleve_enseignement_depuis_option_annee(o ent_eleve_option_annee)
 RETURNS uuid
 LANGUAGE plpgsql
AS $$
DECLARE v_an ent_annee_scolaire; v_offre UUID; v_ens UUID; v_etat TEXT;
BEGIN
    SELECT enseignement_id INTO v_ens FROM ent_eleve_enseignement_reprise WHERE source = 'option_annee' AND option_id = o.id;
    IF v_ens IS NULL THEN
        IF btrim(coalesce(o.code, '')) = '' THEN
            PERFORM ent_eleve_enseignement_a_traiter_ajouter(o.etablissement_id, o.eleve_id, 'option_annee', o.id, 'code_vide');
            RETURN NULL;
        END IF;
        SELECT * INTO v_an FROM ent_annee_scolaire a WHERE a.etablissement_id = o.etablissement_id AND a.libelle = o.annee_scolaire;
        IF v_an.id IS NULL THEN
            PERFORM ent_eleve_enseignement_a_traiter_ajouter(o.etablissement_id, o.eleve_id, 'option_annee', o.id, 'annee_inconnue');
            RETURN NULL;
        END IF;
        v_offre := ent_offre_assurer_depuis_option(o.etablissement_id, v_an.id, o.code, o.libelle, o.nature);
        SELECT id INTO v_ens FROM ent_eleve_enseignement
         WHERE eleve_id = o.eleve_id AND offre_id = v_offre AND etat IN ('choisi', 'accepte');
        IF v_ens IS NULL THEN
            v_etat := CASE o.statut WHEN 'retenue' THEN 'accepte' WHEN 'retiree' THEN 'retire' ELSE 'choisi' END;
            v_ens := gen_random_uuid();
            INSERT INTO ent_eleve_enseignement (id, etablissement_id, annee_id, eleve_id, offre_id, etat, choisi_le, debut_le, source)
            VALUES (v_ens, o.etablissement_id, v_an.id, o.eleve_id, v_offre, v_etat,
                    o.created_at::date, CASE WHEN v_etat = 'accepte' THEN v_an.date_debut END, 'reprise_option');
        END IF;
        INSERT INTO ent_eleve_enseignement_reprise (source, option_id, etablissement_id, enseignement_id)
        VALUES ('option_annee', o.id, o.etablissement_id, v_ens) ON CONFLICT DO NOTHING;
        RETURN v_ens;
    END IF;
    SELECT * INTO v_an FROM ent_annee_scolaire a WHERE a.id = (SELECT annee_id FROM ent_eleve_enseignement WHERE id = v_ens);
    IF o.statut = 'retenue' THEN
        UPDATE ent_eleve_enseignement SET etat = 'accepte', debut_le = v_an.date_debut WHERE id = v_ens AND etat = 'choisi';
    ELSIF o.statut = 'retiree' THEN
        UPDATE ent_eleve_enseignement SET etat = 'retire' WHERE id = v_ens AND etat = 'choisi';
        UPDATE ent_eleve_enseignement
           SET etat = 'quitte', quitte_le = greatest(debut_le, least(ent_aujourdhui_etab(o.etablissement_id, now()), v_an.date_fin)),
               fin_nature = 'changement_en_cours_annee', motif = 'Reprise : option retirée sur l''historique'
         WHERE id = v_ens AND etat = 'accepte';
    END IF;
    RETURN v_ens;
END $$;
