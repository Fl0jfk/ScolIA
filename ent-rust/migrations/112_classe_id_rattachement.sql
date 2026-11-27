-- Socle 2, lot S2-2 : rattachement des données existantes à `classe_id` et rapatriement
-- des périodes de notes. Migration de données IDEMPOTENTE : ne touche que les lignes dont
-- `classe_id` est nul ; aucun texte n'est modifié ; relançable sans effet à vide.
-- Les classes et années absentes sont créées (même règle que l'amorçage `/referentiel/amorcer`).

-- Rattache (une table, un établissement ou tous) ; renvoie le nombre de lignes rattachées.
CREATE OR REPLACE FUNCTION ent_rattacher_classes(p_etab uuid DEFAULT NULL)
RETURNS TABLE (o_table text, o_rattachees bigint) AS $$
DECLARE
    n bigint;
BEGIN
    UPDATE ent_eleve t SET classe_id = ent_resoudre_classe(t.etablissement_id, t.classe, current_date)
     WHERE t.classe_id IS NULL AND btrim(coalesce(t.classe, '')) <> ''
       AND (p_etab IS NULL OR t.etablissement_id = p_etab);
    GET DIAGNOSTICS n = ROW_COUNT; o_table := 'ent_eleve'; o_rattachees := n; RETURN NEXT;

    UPDATE ent_eleve_periode t SET classe_id = ent_resoudre_classe(t.etablissement_id, t.valeur, t.date_debut)
     WHERE t.kind = 'classe' AND t.classe_id IS NULL AND btrim(t.valeur) <> ''
       AND (p_etab IS NULL OR t.etablissement_id = p_etab);
    GET DIAGNOSTICS n = ROW_COUNT; o_table := 'ent_eleve_periode'; o_rattachees := n; RETURN NEXT;

    UPDATE ent_eleve_scolarite_annee t
       SET classe_id = ent_resoudre_classe(t.etablissement_id, t.classe, ent_jour_de_annee_label(t.annee_scolaire))
     WHERE t.classe_id IS NULL AND btrim(coalesce(t.classe, '')) <> ''
       AND (p_etab IS NULL OR t.etablissement_id = p_etab);
    GET DIAGNOSTICS n = ROW_COUNT; o_table := 'ent_eleve_scolarite_annee'; o_rattachees := n; RETURN NEXT;

    UPDATE ent_appel t SET classe_id = ent_resoudre_classe(t.etablissement_id, t.classe, t.jour)
     WHERE t.classe_id IS NULL AND btrim(t.classe) <> ''
       AND (p_etab IS NULL OR t.etablissement_id = p_etab);
    GET DIAGNOSTICS n = ROW_COUNT; o_table := 'ent_appel'; o_rattachees := n; RETURN NEXT;

    UPDATE ent_edt_creneau t SET classe_id = ent_resoudre_classe(t.etablissement_id, t.classe, current_date)
     WHERE t.classe_id IS NULL AND btrim(t.classe) <> ''
       AND (p_etab IS NULL OR t.etablissement_id = p_etab);
    GET DIAGNOSTICS n = ROW_COUNT; o_table := 'ent_edt_creneau'; o_rattachees := n; RETURN NEXT;

    UPDATE ent_devoir t SET classe_id = ent_resoudre_classe(t.etablissement_id, t.classe, t.date_devoir)
     WHERE t.classe_id IS NULL AND btrim(t.classe) <> ''
       AND (p_etab IS NULL OR t.etablissement_id = p_etab);
    GET DIAGNOSTICS n = ROW_COUNT; o_table := 'ent_devoir'; o_rattachees := n; RETURN NEXT;

    UPDATE ent_cahier_texte t SET classe_id = ent_resoudre_classe(t.etablissement_id, t.classe, t.pour_le)
     WHERE t.classe_id IS NULL AND btrim(t.classe) <> ''
       AND (p_etab IS NULL OR t.etablissement_id = p_etab);
    GET DIAGNOSTICS n = ROW_COUNT; o_table := 'ent_cahier_texte'; o_rattachees := n; RETURN NEXT;

    UPDATE ent_conseil_classe t SET classe_id = ent_resoudre_classe(t.etablissement_id, t.classe, t.jour)
     WHERE t.classe_id IS NULL AND btrim(t.classe) <> ''
       AND (p_etab IS NULL OR t.etablissement_id = p_etab);
    GET DIAGNOSTICS n = ROW_COUNT; o_table := 'ent_conseil_classe'; o_rattachees := n; RETURN NEXT;

    UPDATE ent_annonce t SET classe_id = ent_resoudre_classe(t.etablissement_id, t.cible_classe, t.created_at::date)
     WHERE t.classe_id IS NULL AND btrim(coalesce(t.cible_classe, '')) <> ''
       AND (p_etab IS NULL OR t.etablissement_id = p_etab);
    GET DIAGNOSTICS n = ROW_COUNT; o_table := 'ent_annonce'; o_rattachees := n; RETURN NEXT;
END
$$ LANGUAGE plpgsql;

SELECT count(*) FROM ent_rattacher_classes();

-- Période du référentiel correspondant à une période de notes (créée si absente).
CREATE OR REPLACE FUNCTION ent_periode_pour_note(
    p_etab uuid, p_code text, p_libelle text, p_debut date, p_fin date
) RETURNS uuid AS $$
DECLARE
    v_annee uuid := ent_assurer_annee(p_etab, p_debut);
    v_id uuid;
BEGIN
    SELECT p.id INTO v_id FROM ent_periode p
     WHERE p.etablissement_id = p_etab AND p.annee_id = v_annee
       AND p.niveau_id IS NULL AND p.code = p_code;
    IF v_id IS NULL THEN
        INSERT INTO ent_periode (id, etablissement_id, annee_id, niveau_id, code, libelle, kind, ordre, date_debut, date_fin)
        VALUES (
            gen_random_uuid(), p_etab, v_annee, NULL, p_code, p_libelle,
            CASE WHEN p_code ~* '^T[0-9]' THEN 'trimestre'
                 WHEN p_code ~* '^S[0-9]' THEN 'semestre' ELSE 'periode' END,
            (SELECT coalesce(max(q.ordre), 0) + 1 FROM ent_periode q
              WHERE q.etablissement_id = p_etab AND q.annee_id = v_annee AND q.niveau_id IS NULL),
            p_debut, p_fin)
        ON CONFLICT DO NOTHING;
        SELECT p.id INTO v_id FROM ent_periode p
         WHERE p.etablissement_id = p_etab AND p.annee_id = v_annee
           AND p.niveau_id IS NULL AND p.code = p_code;
    END IF;
    RETURN v_id;
END
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION ent_trg_note_periode_rapatrie() RETURNS trigger AS $$
BEGIN
    IF NEW.periode_id IS NULL THEN
        NEW.periode_id := ent_periode_pour_note(
            NEW.etablissement_id, NEW.code, NEW.libelle, NEW.date_debut, NEW.date_fin);
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_note_periode_rapatrie ON ent_note_periode;
CREATE TRIGGER ent_note_periode_rapatrie BEFORE INSERT ON ent_note_periode
    FOR EACH ROW EXECUTE FUNCTION ent_trg_note_periode_rapatrie();

UPDATE ent_note_periode
   SET periode_id = ent_periode_pour_note(etablissement_id, code, libelle, date_debut, date_fin)
 WHERE periode_id IS NULL;
