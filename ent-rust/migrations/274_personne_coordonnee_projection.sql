-- Socle 4, lot A2 (§16, étape « coordonnées ») : projection entre les colonnes héritées de
-- `ent_personne` (email, telephone, telephone_2, adresse) et les coordonnées datées.
--
--  * SENS 1 (coordonnées -> colonnes héritées) : après toute écriture dans
--    `ent_personne_coordonnee`, les colonnes héritées reçoivent la valeur « courante »
--    (principal d'abord). Les lecteurs historiques (envois, SMS, inscriptions) restent justes.
--  * SENS 2 (colonnes héritées -> coordonnées) : un écrivain historique (inscriptions, imports,
--    anciens chemins) qui modifie une colonne héritée ne PERD rien : l'ancienne valeur est
--    fermée dans l'historique, la nouvelle ouverte. Aucune coordonnée perdue.
--  * Le compte est une coordonnée de plus : quand un compte est rattaché à une personne, son
--    e-mail (et son téléphone) entrent dans la fiche avec l'usage `connexion`.
-- Un drapeau de transaction (`ent.coord_projection`) coupe la boucle entre les deux sens.
-- Reprise des données existantes : une seule fois (marqueur), avant la pose des déclencheurs.

CREATE OR REPLACE FUNCTION ent_coord_projetees(p_etab UUID, p_personne UUID, p_groupe TEXT)
RETURNS SETOF ent_personne_coordonnee
LANGUAGE sql STABLE AS $$
    SELECT c.* FROM ent_personne_coordonnee c
     WHERE c.etablissement_id = p_etab AND c.personne_id = p_personne AND c.au IS NULL
       AND ent_coord_groupe(c.type) = p_groupe
       AND c.delivrabilite NOT IN ('rebond_definitif', 'non_attribue', 'desinscrit')
       AND (p_groupe = 'adresse'
            OR c.usages && (CASE p_groupe
                  WHEN 'email' THEN ARRAY['notifications']
                  ELSE ARRAY['notifications', 'urgence', 'facturation'] END)::TEXT[])
     ORDER BY c.principal DESC,
              CASE c.type WHEN 'mobile' THEN 0 WHEN 'fixe' THEN 1 WHEN 'professionnel' THEN 2 ELSE 3 END,
              c.du ASC, c.id ASC
$$;

CREATE OR REPLACE FUNCTION ent_coord_histo(
    c ent_personne_coordonnee, p_nature TEXT, p_raison TEXT, p_via TEXT, p_par UUID
) RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE
    v_par UUID := COALESCE(p_par, NULLIF(current_setting('ent.coord_acteur', true), '')::UUID);
    v_via TEXT := COALESCE(NULLIF(current_setting('ent.coord_via', true), ''), p_via);
    v_raison TEXT := CASE WHEN p_raison = 'ecriture_historique'
                          THEN COALESCE(NULLIF(current_setting('ent.coord_raison', true), ''), p_raison)
                          ELSE p_raison END;
BEGIN
    INSERT INTO ent_personne_coordonnee_histo
        (id, etablissement_id, personne_id, coordonnee_id, type, nature, raison, via, par)
    VALUES (gen_random_uuid(), c.etablissement_id, c.personne_id, c.id, c.type, p_nature,
            v_raison, v_via, v_par);
END
$$;

-- Écrit les colonnes héritées d'après les coordonnées courantes.
CREATE OR REPLACE FUNCTION ent_coord_projeter(p_etab UUID, p_personne UUID) RETURNS VOID
LANGUAGE plpgsql AS $$
DECLARE
    v_email TEXT;
    v_tel TEXT;
    v_tel2 TEXT;
    v_adresse TEXT;
BEGIN
    SELECT valeur INTO v_email FROM ent_coord_projetees(p_etab, p_personne, 'email') LIMIT 1;
    SELECT valeur INTO v_tel FROM ent_coord_projetees(p_etab, p_personne, 'telephone') LIMIT 1;
    SELECT valeur INTO v_tel2 FROM ent_coord_projetees(p_etab, p_personne, 'telephone')
     OFFSET 1 LIMIT 1;
    SELECT valeur INTO v_adresse FROM ent_coord_projetees(p_etab, p_personne, 'adresse') LIMIT 1;
    IF EXISTS (SELECT 1 FROM ent_personne
                WHERE etablissement_id = p_etab AND id = p_personne
                  AND (email IS DISTINCT FROM v_email OR telephone IS DISTINCT FROM v_tel
                       OR telephone_2 IS DISTINCT FROM v_tel2
                       OR adresse IS DISTINCT FROM v_adresse)) THEN
        PERFORM set_config('ent.coord_projection', 'on', true);
        UPDATE ent_personne
           SET email = v_email, telephone = v_tel, telephone_2 = v_tel2, adresse = v_adresse,
               updated_at = now()
         WHERE etablissement_id = p_etab AND id = p_personne;
        PERFORM set_config('ent.coord_projection', 'off', true);
    END IF;
END
$$;

CREATE OR REPLACE FUNCTION ent_coord_trigger_projection() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF current_setting('ent.coord_projection', true) = 'on' THEN
        RETURN NULL;
    END IF;
    IF TG_OP = 'DELETE' THEN
        PERFORM ent_coord_projeter(OLD.etablissement_id, OLD.personne_id);
    ELSE
        PERFORM ent_coord_projeter(NEW.etablissement_id, NEW.personne_id);
    END IF;
    RETURN NULL;
END
$$;

-- Retrait d'une coordonnée par un écrivain historique : fermée, sauf si elle sert aussi à la
-- connexion (elle reste alors, réduite à cet usage).
CREATE OR REPLACE FUNCTION ent_coord_retirer_legacy(c ent_personne_coordonnee, p_remplacement BOOLEAN)
RETURNS VOID LANGUAGE plpgsql AS $$
BEGIN
    IF 'connexion' = ANY (c.usages) THEN
        UPDATE ent_personne_coordonnee
           SET principal = false, usages = ARRAY['connexion']::TEXT[]
         WHERE id = c.id;
        PERFORM ent_coord_histo(c, 'usages', 'ecriture_historique', 'systeme', NULL);
    ELSE
        UPDATE ent_personne_coordonnee SET principal = false, au = GREATEST(now(), du) WHERE id = c.id;
        PERFORM ent_coord_histo(c, CASE WHEN p_remplacement THEN 'remplacement' ELSE 'retrait' END,
                                'ecriture_historique', 'systeme', NULL);
    END IF;
END
$$;

-- Applique la valeur d'une colonne héritée (rang 1 = principal, rang 2 = second téléphone).
CREATE OR REPLACE FUNCTION ent_coord_poser(
    p_etab UUID, p_personne UUID, p_groupe TEXT, p_rang INT, p_brut TEXT
) RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE
    v TEXT;
    v_type TEXT;
    v_a_verifier BOOLEAN := false;
    cur ent_personne_coordonnee;
    ex ent_personne_coordonnee;
    nouvelle ent_personne_coordonnee;
BEGIN
    IF p_groupe = 'email' THEN
        v := NULLIF(lower(btrim(p_brut)), '');
        v_type := 'email';
    ELSIF p_groupe = 'adresse' THEN
        v := NULLIF(btrim(p_brut), '');
        v_type := 'adresse';
    ELSE
        v := ent_tel_e164(p_brut);
        IF v IS NULL THEN
            v := NULLIF(btrim(p_brut), '');
            v_a_verifier := v IS NOT NULL;
        END IF;
        v_type := CASE WHEN v ~ '^\+33[67]' THEN 'mobile' WHEN v ~ '^\+33' THEN 'fixe' ELSE 'mobile' END;
    END IF;
    SELECT * INTO cur FROM ent_coord_projetees(p_etab, p_personne, p_groupe)
     OFFSET (p_rang - 1) LIMIT 1;
    IF v IS NULL THEN
        IF cur.id IS NOT NULL THEN
            PERFORM ent_coord_retirer_legacy(cur, false);
        END IF;
        RETURN;
    END IF;
    IF cur.id IS NOT NULL AND cur.valeur = v THEN
        RETURN;
    END IF;
    SELECT * INTO ex FROM ent_personne_coordonnee
     WHERE etablissement_id = p_etab AND personne_id = p_personne AND au IS NULL
       AND ent_coord_groupe(type) = p_groupe AND valeur = v
     LIMIT 1;
    IF ex.id IS NOT NULL THEN
        IF p_rang = 1 AND NOT ex.principal THEN
            UPDATE ent_personne_coordonnee SET principal = false
             WHERE personne_id = p_personne AND au IS NULL AND principal
               AND ent_coord_groupe(type) = p_groupe;
            UPDATE ent_personne_coordonnee
               SET principal = true,
                   usages = (SELECT array_agg(DISTINCT u) FROM unnest(array_append(usages, 'notifications')) u)
             WHERE id = ex.id;
            PERFORM ent_coord_histo(ex, 'principal', 'ecriture_historique', 'systeme', NULL);
        END IF;
        RETURN;
    END IF;
    IF cur.id IS NOT NULL THEN
        PERFORM ent_coord_retirer_legacy(cur, true);
    END IF;
    IF p_rang = 1 THEN
        UPDATE ent_personne_coordonnee SET principal = false
         WHERE personne_id = p_personne AND au IS NULL AND principal
           AND ent_coord_groupe(type) = p_groupe;
    END IF;
    INSERT INTO ent_personne_coordonnee
        (id, etablissement_id, personne_id, type, valeur, usages, principal, a_verifier, source,
         remplace_id)
    VALUES (gen_random_uuid(), p_etab, p_personne, v_type, v, ARRAY['notifications']::TEXT[],
            p_rang = 1, v_a_verifier, 'ecriture_historique', cur.id)
    RETURNING * INTO nouvelle;
    PERFORM ent_coord_histo(nouvelle, 'ajout', 'ecriture_historique', 'systeme', NULL);
END
$$;

-- Le compte est une coordonnée : son e-mail et son téléphone entrent dans la fiche (usage
-- `connexion`) quand ils n'y sont pas. Ils servent aussi aux envois seulement si la fiche n'a
-- aucune autre coordonnée de ce genre (comportement d'avant : le compte était le repli).
CREATE OR REPLACE FUNCTION ent_coord_compte(p_etab UUID, p_personne UUID, p_user UUID)
RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE
    ue TEXT;
    ut TEXT;
    ut_norm TEXT;
    ex ent_personne_coordonnee;
    nouvelle ent_personne_coordonnee;
    autre BOOLEAN;
BEGIN
    SELECT NULLIF(lower(btrim(email)), ''), NULLIF(btrim(telephone), '')
      INTO ue, ut FROM ent_user WHERE id = p_user;
    IF ue IS NOT NULL THEN
        SELECT * INTO ex FROM ent_personne_coordonnee
         WHERE etablissement_id = p_etab AND personne_id = p_personne AND au IS NULL
           AND type = 'email' AND valeur = ue;
        IF ex.id IS NOT NULL THEN
            IF NOT ('connexion' = ANY (ex.usages)) THEN
                UPDATE ent_personne_coordonnee
                   SET usages = array_append(usages, 'connexion'),
                       verifiee_le = COALESCE(verifiee_le, now()),
                       verification_voie = COALESCE(verification_voie, 'compte')
                 WHERE id = ex.id;
                PERFORM ent_coord_histo(ex, 'usages', 'compte_rattache', 'systeme', NULL);
            END IF;
        ELSE
            autre := EXISTS (SELECT 1 FROM ent_coord_projetees(p_etab, p_personne, 'email'));
            INSERT INTO ent_personne_coordonnee
                (id, etablissement_id, personne_id, type, valeur, usages, principal, verifiee_le,
                 verification_voie, source)
            VALUES (gen_random_uuid(), p_etab, p_personne, 'email', ue,
                    CASE WHEN autre THEN ARRAY['connexion']::TEXT[]
                         ELSE ARRAY['connexion', 'notifications']::TEXT[] END,
                    NOT autre, now(), 'compte', 'compte')
            RETURNING * INTO nouvelle;
            PERFORM ent_coord_histo(nouvelle, 'ajout', 'compte_rattache', 'systeme', NULL);
        END IF;
    END IF;
    IF ut IS NOT NULL THEN
        ut_norm := COALESCE(ent_tel_e164(ut), ut);
        SELECT * INTO ex FROM ent_personne_coordonnee
         WHERE etablissement_id = p_etab AND personne_id = p_personne AND au IS NULL
           AND ent_coord_groupe(type) = 'telephone' AND valeur = ut_norm;
        IF ex.id IS NOT NULL THEN
            IF NOT ('connexion' = ANY (ex.usages)) THEN
                UPDATE ent_personne_coordonnee SET usages = array_append(usages, 'connexion') WHERE id = ex.id;
                PERFORM ent_coord_histo(ex, 'usages', 'compte_rattache', 'systeme', NULL);
            END IF;
        ELSE
            autre := EXISTS (SELECT 1 FROM ent_coord_projetees(p_etab, p_personne, 'telephone'));
            INSERT INTO ent_personne_coordonnee
                (id, etablissement_id, personne_id, type, valeur, usages, principal, a_verifier,
                 source)
            VALUES (gen_random_uuid(), p_etab, p_personne,
                    CASE WHEN ut_norm ~ '^\+33[67]' THEN 'mobile' WHEN ut_norm ~ '^\+33' THEN 'fixe'
                         ELSE 'mobile' END,
                    ut_norm,
                    CASE WHEN autre THEN ARRAY['connexion']::TEXT[]
                         ELSE ARRAY['connexion', 'notifications']::TEXT[] END,
                    NOT autre, ent_tel_e164(ut) IS NULL, 'compte')
            RETURNING * INTO nouvelle;
            PERFORM ent_coord_histo(nouvelle, 'ajout', 'compte_rattache', 'systeme', NULL);
        END IF;
    END IF;
END
$$;

-- Déclencheur du sens 2 : écrivain historique sur ent_personne.
CREATE OR REPLACE FUNCTION ent_coord_depuis_legacy() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF current_setting('ent.coord_projection', true) = 'on' THEN
        RETURN NULL;
    END IF;
    IF TG_OP = 'INSERT' THEN
        IF NEW.email IS NOT NULL THEN
            PERFORM ent_coord_poser(NEW.etablissement_id, NEW.id, 'email', 1, NEW.email);
        END IF;
        IF NEW.telephone IS NOT NULL THEN
            PERFORM ent_coord_poser(NEW.etablissement_id, NEW.id, 'telephone', 1, NEW.telephone);
        END IF;
        IF NEW.telephone_2 IS NOT NULL THEN
            PERFORM ent_coord_poser(NEW.etablissement_id, NEW.id, 'telephone', 2, NEW.telephone_2);
        END IF;
        IF NEW.adresse IS NOT NULL THEN
            PERFORM ent_coord_poser(NEW.etablissement_id, NEW.id, 'adresse', 1, NEW.adresse);
        END IF;
        IF NEW.user_id IS NOT NULL THEN
            PERFORM ent_coord_compte(NEW.etablissement_id, NEW.id, NEW.user_id);
        END IF;
    ELSE
        IF NEW.email IS DISTINCT FROM OLD.email THEN
            PERFORM ent_coord_poser(NEW.etablissement_id, NEW.id, 'email', 1, NEW.email);
        END IF;
        IF NEW.telephone IS DISTINCT FROM OLD.telephone THEN
            PERFORM ent_coord_poser(NEW.etablissement_id, NEW.id, 'telephone', 1, NEW.telephone);
        END IF;
        IF NEW.telephone_2 IS DISTINCT FROM OLD.telephone_2 THEN
            PERFORM ent_coord_poser(NEW.etablissement_id, NEW.id, 'telephone', 2, NEW.telephone_2);
        END IF;
        IF NEW.adresse IS DISTINCT FROM OLD.adresse THEN
            PERFORM ent_coord_poser(NEW.etablissement_id, NEW.id, 'adresse', 1, NEW.adresse);
        END IF;
        IF NEW.user_id IS NOT NULL AND NEW.user_id IS DISTINCT FROM OLD.user_id THEN
            PERFORM ent_coord_compte(NEW.etablissement_id, NEW.id, NEW.user_id);
        END IF;
    END IF;
    PERFORM ent_coord_projeter(NEW.etablissement_id, NEW.id);
    RETURN NULL;
END
$$;

-- Reprise unique des données existantes (avant la pose des déclencheurs).
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM ent_coord_reprise_marque WHERE cle = '273_reprise') THEN
        RETURN;
    END IF;
    PERFORM set_config('ent.coord_projection', 'on', true);
    INSERT INTO ent_personne_coordonnee
        (id, etablissement_id, personne_id, type, valeur, usages, principal, source)
    SELECT gen_random_uuid(), p.etablissement_id, p.id, 'email', lower(btrim(p.email)),
           ARRAY['notifications']::TEXT[], true, 'reprise'
      FROM ent_personne p
     WHERE p.email IS NOT NULL AND btrim(p.email) <> '' AND length(btrim(p.email)) <= 254
    ON CONFLICT DO NOTHING;
    INSERT INTO ent_personne_coordonnee
        (id, etablissement_id, personne_id, type, valeur, usages, principal, a_verifier, source)
    SELECT gen_random_uuid(), p.etablissement_id, p.id,
           CASE WHEN ent_tel_e164(p.telephone) ~ '^\+33[67]' THEN 'mobile'
                WHEN ent_tel_e164(p.telephone) ~ '^\+33' THEN 'fixe' ELSE 'mobile' END,
           COALESCE(ent_tel_e164(p.telephone), btrim(p.telephone)),
           ARRAY['notifications']::TEXT[], true, ent_tel_e164(p.telephone) IS NULL, 'reprise'
      FROM ent_personne p
     WHERE p.telephone IS NOT NULL AND btrim(p.telephone) <> ''
    ON CONFLICT DO NOTHING;
    INSERT INTO ent_personne_coordonnee
        (id, etablissement_id, personne_id, type, valeur, usages, principal, a_verifier, source)
    SELECT gen_random_uuid(), p.etablissement_id, p.id,
           CASE WHEN ent_tel_e164(p.telephone_2) ~ '^\+33[67]' THEN 'mobile'
                WHEN ent_tel_e164(p.telephone_2) ~ '^\+33' THEN 'fixe' ELSE 'mobile' END,
           COALESCE(ent_tel_e164(p.telephone_2), btrim(p.telephone_2)),
           ARRAY['notifications']::TEXT[], false, ent_tel_e164(p.telephone_2) IS NULL, 'reprise'
      FROM ent_personne p
     WHERE p.telephone_2 IS NOT NULL AND btrim(p.telephone_2) <> ''
    ON CONFLICT DO NOTHING;
    INSERT INTO ent_personne_coordonnee
        (id, etablissement_id, personne_id, type, valeur, usages, principal, source)
    SELECT gen_random_uuid(), p.etablissement_id, p.id, 'adresse', btrim(p.adresse),
           ARRAY['notifications']::TEXT[], true, 'reprise'
      FROM ent_personne p
     WHERE p.adresse IS NOT NULL AND btrim(p.adresse) <> ''
    ON CONFLICT DO NOTHING;
    PERFORM ent_coord_compte(p.etablissement_id, p.id, p.user_id)
       FROM ent_personne p WHERE p.user_id IS NOT NULL;
    INSERT INTO ent_personne_coordonnee_histo
        (id, etablissement_id, personne_id, coordonnee_id, type, nature, raison, via)
    SELECT gen_random_uuid(), c.etablissement_id, c.personne_id, c.id, c.type, 'reprise',
           CASE WHEN c.source = 'compte' THEN 'compte_rattache' ELSE 'reprise' END, 'systeme'
      FROM ent_personne_coordonnee c
     WHERE NOT EXISTS (SELECT 1 FROM ent_personne_coordonnee_histo h WHERE h.coordonnee_id = c.id);
    PERFORM set_config('ent.coord_projection', 'off', true);
    PERFORM ent_coord_projeter(p.etablissement_id, p.id)
       FROM ent_personne p
      WHERE EXISTS (SELECT 1 FROM ent_personne_coordonnee c WHERE c.personne_id = p.id);
    INSERT INTO ent_coord_reprise_marque (cle) VALUES ('273_reprise') ON CONFLICT DO NOTHING;
END
$$;

DROP TRIGGER IF EXISTS ent_personne_coordonnee_projection ON ent_personne_coordonnee;
CREATE TRIGGER ent_personne_coordonnee_projection
    AFTER INSERT OR UPDATE OR DELETE ON ent_personne_coordonnee
    FOR EACH ROW EXECUTE FUNCTION ent_coord_trigger_projection();

DROP TRIGGER IF EXISTS ent_personne_coordonnee_legacy ON ent_personne;
CREATE TRIGGER ent_personne_coordonnee_legacy
    AFTER INSERT OR UPDATE OF email, telephone, telephone_2, adresse, user_id ON ent_personne
    FOR EACH ROW EXECUTE FUNCTION ent_coord_depuis_legacy();
