-- Socle 2, lot S2-2 : `classe_id` à côté du texte `classe` des 9 tables historiques (V1).
-- Double écriture assurée par la base : un trigger BEFORE INSERT/UPDATE garde texte et
-- identifiant cohérents quel que soit le code appelant (aucun module à réécrire d'un coup).
--   * texte seul écrit  -> classe_id résolu (classe et année créées au besoin, comme l'amorçage) ;
--   * classe_id écrit   -> le texte est aligné sur le code de la classe (l'identifiant fait foi) ;
--   * classe d'un autre établissement -> refusée.
-- Les colonnes texte restent en place (plan de coupure : docs/SOCLE2_DOMAINES.md, S2-2).
-- Idempotent : relançable à chaque démarrage.

CREATE OR REPLACE FUNCTION ent_niveau_de_code(p_code text) RETURNS text AS $$
    SELECT CASE lower(substr(btrim(p_code), 1, 1))
        WHEN '3' THEN '3e' WHEN '4' THEN '4e' WHEN '5' THEN '5e' WHEN '6' THEN '6e'
        WHEN '2' THEN '2nde' WHEN '1' THEN '1re' WHEN 't' THEN 'Tle'
        ELSE NULL END
$$ LANGUAGE sql IMMUTABLE;

CREATE OR REPLACE FUNCTION ent_jour_de_annee_label(p_label text) RETURNS date AS $$
    SELECT CASE WHEN p_label ~ '^[0-9]{4}-[0-9]{4}$'
                THEN make_date(substr(p_label, 1, 4)::int, 9, 1)
                ELSE current_date END
$$ LANGUAGE sql IMMUTABLE;

-- Année scolaire contenant `p_jour`, créée (1er septembre -> 31 août) si absente.
CREATE OR REPLACE FUNCTION ent_assurer_annee(p_etab uuid, p_jour date) RETURNS uuid AS $$
DECLARE
    v_id uuid;
    v_y int;
    v_label text;
    v_debut date;
    v_fin date;
    v_courante boolean;
BEGIN
    SELECT a.id INTO v_id FROM ent_annee_scolaire a
     WHERE a.etablissement_id = p_etab AND a.date_debut <= p_jour AND a.date_fin >= p_jour
     ORDER BY a.courante DESC, a.date_debut DESC LIMIT 1;
    IF v_id IS NOT NULL THEN
        RETURN v_id;
    END IF;
    v_y := CASE WHEN extract(month FROM p_jour) >= 9
                THEN extract(year FROM p_jour)::int ELSE extract(year FROM p_jour)::int - 1 END;
    v_label := v_y || '-' || (v_y + 1);
    v_debut := make_date(v_y, 9, 1);
    v_fin := make_date(v_y + 1, 8, 31);
    v_courante := current_date BETWEEN v_debut AND v_fin
        AND NOT EXISTS (SELECT 1 FROM ent_annee_scolaire a
                         WHERE a.etablissement_id = p_etab AND a.courante);
    INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin, courante)
    VALUES (gen_random_uuid(), p_etab, v_label, v_debut, v_fin, v_courante)
    ON CONFLICT DO NOTHING;
    SELECT a.id INTO v_id FROM ent_annee_scolaire a
     WHERE a.etablissement_id = p_etab AND a.libelle = v_label;
    RETURN v_id;
END
$$ LANGUAGE plpgsql;

-- Classe `p_code` de l'année contenant `p_jour` ; créée au besoin.
CREATE OR REPLACE FUNCTION ent_resoudre_classe(p_etab uuid, p_code text, p_jour date) RETURNS uuid AS $$
DECLARE
    v_code text := btrim(p_code);
    v_annee uuid;
    v_id uuid;
BEGIN
    IF v_code IS NULL OR v_code = '' THEN
        RETURN NULL;
    END IF;
    v_annee := ent_assurer_annee(p_etab, p_jour);
    SELECT c.id INTO v_id FROM ent_classe c
     WHERE c.etablissement_id = p_etab AND c.annee_id = v_annee AND c.code = v_code;
    IF v_id IS NULL THEN
        INSERT INTO ent_classe (id, etablissement_id, annee_id, code, libelle, niveau)
        VALUES (gen_random_uuid(), p_etab, v_annee, v_code, v_code, ent_niveau_de_code(v_code))
        ON CONFLICT DO NOTHING;
        SELECT c.id INTO v_id FROM ent_classe c
         WHERE c.etablissement_id = p_etab AND c.annee_id = v_annee AND c.code = v_code;
    END IF;
    RETURN v_id;
END
$$ LANGUAGE plpgsql;

-- Règle de cohérence texte <-> identifiant (voir l'en-tête).
CREATE OR REPLACE FUNCTION ent_synchro_classe(
    p_etab uuid, p_texte text, p_id uuid, p_old_texte text, p_old_id uuid,
    p_jour date, p_insert boolean
) RETURNS TABLE (o_classe_id uuid, o_classe text) AS $$
DECLARE
    v_code text;
BEGIN
    IF p_id IS NOT NULL AND (p_insert OR p_old_id IS DISTINCT FROM p_id) THEN
        SELECT c.code INTO v_code FROM ent_classe c
         WHERE c.id = p_id AND c.etablissement_id = p_etab;
        IF v_code IS NULL THEN
            RAISE EXCEPTION 'classe_id inconnu ou d''un autre etablissement' USING ERRCODE = '23503';
        END IF;
        IF p_texte IS NOT NULL AND btrim(p_texte) = v_code THEN
            RETURN QUERY SELECT p_id, p_texte;
        ELSE
            RETURN QUERY SELECT p_id, v_code;
        END IF;
        RETURN;
    END IF;
    IF p_texte IS NULL OR btrim(p_texte) = '' THEN
        RETURN QUERY SELECT NULL::uuid, p_texte;
        RETURN;
    END IF;
    IF p_id IS NOT NULL AND NOT p_insert AND p_old_texte IS NOT DISTINCT FROM p_texte THEN
        RETURN QUERY SELECT p_id, p_texte;
        RETURN;
    END IF;
    RETURN QUERY SELECT ent_resoudre_classe(p_etab, p_texte, p_jour), p_texte;
END
$$ LANGUAGE plpgsql;

ALTER TABLE ent_eleve ADD COLUMN IF NOT EXISTS classe_id UUID REFERENCES ent_classe (id);
CREATE INDEX IF NOT EXISTS ent_eleve_classe_id_idx ON ent_eleve (etablissement_id, classe_id) WHERE classe_id IS NOT NULL;
CREATE OR REPLACE FUNCTION ent_trg_classe_id_ent_eleve() RETURNS trigger AS $$
BEGIN
    SELECT r.o_classe_id, r.o_classe INTO NEW.classe_id, NEW.classe
      FROM ent_synchro_classe(
          NEW.etablissement_id, NEW.classe, NEW.classe_id,
          CASE WHEN TG_OP = 'UPDATE' THEN OLD.classe END,
          CASE WHEN TG_OP = 'UPDATE' THEN OLD.classe_id END,
          current_date, TG_OP = 'INSERT') r;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_classe_id_sync ON ent_eleve;
CREATE TRIGGER ent_classe_id_sync BEFORE INSERT OR UPDATE OF classe, classe_id ON ent_eleve
    FOR EACH ROW EXECUTE FUNCTION ent_trg_classe_id_ent_eleve();

ALTER TABLE ent_eleve_periode ADD COLUMN IF NOT EXISTS classe_id UUID REFERENCES ent_classe (id);
CREATE INDEX IF NOT EXISTS ent_eleve_periode_classe_id_idx ON ent_eleve_periode (etablissement_id, classe_id) WHERE classe_id IS NOT NULL;
CREATE OR REPLACE FUNCTION ent_trg_classe_id_ent_eleve_periode() RETURNS trigger AS $$
BEGIN
    IF NOT (NEW.kind = 'classe') THEN
        RETURN NEW;
    END IF;
    SELECT r.o_classe_id, r.o_classe INTO NEW.classe_id, NEW.valeur
      FROM ent_synchro_classe(
          NEW.etablissement_id, NEW.valeur, NEW.classe_id,
          CASE WHEN TG_OP = 'UPDATE' THEN OLD.valeur END,
          CASE WHEN TG_OP = 'UPDATE' THEN OLD.classe_id END,
          NEW.date_debut, TG_OP = 'INSERT') r;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_classe_id_sync ON ent_eleve_periode;
CREATE TRIGGER ent_classe_id_sync BEFORE INSERT OR UPDATE OF valeur, classe_id ON ent_eleve_periode
    FOR EACH ROW EXECUTE FUNCTION ent_trg_classe_id_ent_eleve_periode();

ALTER TABLE ent_eleve_scolarite_annee ADD COLUMN IF NOT EXISTS classe_id UUID REFERENCES ent_classe (id);
CREATE INDEX IF NOT EXISTS ent_eleve_scolarite_annee_classe_id_idx ON ent_eleve_scolarite_annee (etablissement_id, classe_id) WHERE classe_id IS NOT NULL;
CREATE OR REPLACE FUNCTION ent_trg_classe_id_ent_eleve_scolarite_annee() RETURNS trigger AS $$
BEGIN
    SELECT r.o_classe_id, r.o_classe INTO NEW.classe_id, NEW.classe
      FROM ent_synchro_classe(
          NEW.etablissement_id, NEW.classe, NEW.classe_id,
          CASE WHEN TG_OP = 'UPDATE' THEN OLD.classe END,
          CASE WHEN TG_OP = 'UPDATE' THEN OLD.classe_id END,
          ent_jour_de_annee_label(NEW.annee_scolaire), TG_OP = 'INSERT') r;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_classe_id_sync ON ent_eleve_scolarite_annee;
CREATE TRIGGER ent_classe_id_sync BEFORE INSERT OR UPDATE OF classe, classe_id ON ent_eleve_scolarite_annee
    FOR EACH ROW EXECUTE FUNCTION ent_trg_classe_id_ent_eleve_scolarite_annee();

ALTER TABLE ent_appel ADD COLUMN IF NOT EXISTS classe_id UUID REFERENCES ent_classe (id);
CREATE INDEX IF NOT EXISTS ent_appel_classe_id_idx ON ent_appel (etablissement_id, classe_id) WHERE classe_id IS NOT NULL;
CREATE OR REPLACE FUNCTION ent_trg_classe_id_ent_appel() RETURNS trigger AS $$
BEGIN
    SELECT r.o_classe_id, r.o_classe INTO NEW.classe_id, NEW.classe
      FROM ent_synchro_classe(
          NEW.etablissement_id, NEW.classe, NEW.classe_id,
          CASE WHEN TG_OP = 'UPDATE' THEN OLD.classe END,
          CASE WHEN TG_OP = 'UPDATE' THEN OLD.classe_id END,
          NEW.jour, TG_OP = 'INSERT') r;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_classe_id_sync ON ent_appel;
CREATE TRIGGER ent_classe_id_sync BEFORE INSERT OR UPDATE OF classe, classe_id ON ent_appel
    FOR EACH ROW EXECUTE FUNCTION ent_trg_classe_id_ent_appel();

ALTER TABLE ent_edt_creneau ADD COLUMN IF NOT EXISTS classe_id UUID REFERENCES ent_classe (id);
CREATE INDEX IF NOT EXISTS ent_edt_creneau_classe_id_idx ON ent_edt_creneau (etablissement_id, classe_id) WHERE classe_id IS NOT NULL;
CREATE OR REPLACE FUNCTION ent_trg_classe_id_ent_edt_creneau() RETURNS trigger AS $$
BEGIN
    SELECT r.o_classe_id, r.o_classe INTO NEW.classe_id, NEW.classe
      FROM ent_synchro_classe(
          NEW.etablissement_id, NEW.classe, NEW.classe_id,
          CASE WHEN TG_OP = 'UPDATE' THEN OLD.classe END,
          CASE WHEN TG_OP = 'UPDATE' THEN OLD.classe_id END,
          current_date, TG_OP = 'INSERT') r;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_classe_id_sync ON ent_edt_creneau;
CREATE TRIGGER ent_classe_id_sync BEFORE INSERT OR UPDATE OF classe, classe_id ON ent_edt_creneau
    FOR EACH ROW EXECUTE FUNCTION ent_trg_classe_id_ent_edt_creneau();

ALTER TABLE ent_devoir ADD COLUMN IF NOT EXISTS classe_id UUID REFERENCES ent_classe (id);
CREATE INDEX IF NOT EXISTS ent_devoir_classe_id_idx ON ent_devoir (etablissement_id, classe_id) WHERE classe_id IS NOT NULL;
CREATE OR REPLACE FUNCTION ent_trg_classe_id_ent_devoir() RETURNS trigger AS $$
BEGIN
    SELECT r.o_classe_id, r.o_classe INTO NEW.classe_id, NEW.classe
      FROM ent_synchro_classe(
          NEW.etablissement_id, NEW.classe, NEW.classe_id,
          CASE WHEN TG_OP = 'UPDATE' THEN OLD.classe END,
          CASE WHEN TG_OP = 'UPDATE' THEN OLD.classe_id END,
          NEW.date_devoir, TG_OP = 'INSERT') r;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_classe_id_sync ON ent_devoir;
CREATE TRIGGER ent_classe_id_sync BEFORE INSERT OR UPDATE OF classe, classe_id ON ent_devoir
    FOR EACH ROW EXECUTE FUNCTION ent_trg_classe_id_ent_devoir();

ALTER TABLE ent_cahier_texte ADD COLUMN IF NOT EXISTS classe_id UUID REFERENCES ent_classe (id);
CREATE INDEX IF NOT EXISTS ent_cahier_texte_classe_id_idx ON ent_cahier_texte (etablissement_id, classe_id) WHERE classe_id IS NOT NULL;
CREATE OR REPLACE FUNCTION ent_trg_classe_id_ent_cahier_texte() RETURNS trigger AS $$
BEGIN
    SELECT r.o_classe_id, r.o_classe INTO NEW.classe_id, NEW.classe
      FROM ent_synchro_classe(
          NEW.etablissement_id, NEW.classe, NEW.classe_id,
          CASE WHEN TG_OP = 'UPDATE' THEN OLD.classe END,
          CASE WHEN TG_OP = 'UPDATE' THEN OLD.classe_id END,
          NEW.pour_le, TG_OP = 'INSERT') r;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_classe_id_sync ON ent_cahier_texte;
CREATE TRIGGER ent_classe_id_sync BEFORE INSERT OR UPDATE OF classe, classe_id ON ent_cahier_texte
    FOR EACH ROW EXECUTE FUNCTION ent_trg_classe_id_ent_cahier_texte();

ALTER TABLE ent_conseil_classe ADD COLUMN IF NOT EXISTS classe_id UUID REFERENCES ent_classe (id);
CREATE INDEX IF NOT EXISTS ent_conseil_classe_classe_id_idx ON ent_conseil_classe (etablissement_id, classe_id) WHERE classe_id IS NOT NULL;
CREATE OR REPLACE FUNCTION ent_trg_classe_id_ent_conseil_classe() RETURNS trigger AS $$
BEGIN
    SELECT r.o_classe_id, r.o_classe INTO NEW.classe_id, NEW.classe
      FROM ent_synchro_classe(
          NEW.etablissement_id, NEW.classe, NEW.classe_id,
          CASE WHEN TG_OP = 'UPDATE' THEN OLD.classe END,
          CASE WHEN TG_OP = 'UPDATE' THEN OLD.classe_id END,
          NEW.jour, TG_OP = 'INSERT') r;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_classe_id_sync ON ent_conseil_classe;
CREATE TRIGGER ent_classe_id_sync BEFORE INSERT OR UPDATE OF classe, classe_id ON ent_conseil_classe
    FOR EACH ROW EXECUTE FUNCTION ent_trg_classe_id_ent_conseil_classe();

ALTER TABLE ent_annonce ADD COLUMN IF NOT EXISTS classe_id UUID REFERENCES ent_classe (id);
CREATE INDEX IF NOT EXISTS ent_annonce_classe_id_idx ON ent_annonce (etablissement_id, classe_id) WHERE classe_id IS NOT NULL;
CREATE OR REPLACE FUNCTION ent_trg_classe_id_ent_annonce() RETURNS trigger AS $$
BEGIN
    SELECT r.o_classe_id, r.o_classe INTO NEW.classe_id, NEW.cible_classe
      FROM ent_synchro_classe(
          NEW.etablissement_id, NEW.cible_classe, NEW.classe_id,
          CASE WHEN TG_OP = 'UPDATE' THEN OLD.cible_classe END,
          CASE WHEN TG_OP = 'UPDATE' THEN OLD.classe_id END,
          NEW.created_at::date, TG_OP = 'INSERT') r;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_classe_id_sync ON ent_annonce;
CREATE TRIGGER ent_classe_id_sync BEFORE INSERT OR UPDATE OF cible_classe, classe_id ON ent_annonce
    FOR EACH ROW EXECUTE FUNCTION ent_trg_classe_id_ent_annonce();
