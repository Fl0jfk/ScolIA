-- Jour d'établissement dans la conversion de `created_at` de la fonction de 340/342 (suite de 297). Elle est
-- créée par 340 et redéfinie par 342, qui s'appliquent après 297 : elle est donc redéfinie ici, après 342.
-- Même définition que 342, avec `o.created_at` converti au jour de l'ÉTABLISSEMENT de l'option
-- (ent_aujourdhui_etab, migration 265). Signature inchangée. Idempotent.
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
                    ent_aujourdhui_etab(o.etablissement_id, o.created_at), CASE WHEN v_etat = 'accepte' THEN v_an.date_debut END, 'reprise_option');
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
