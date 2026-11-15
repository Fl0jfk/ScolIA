-- Socle 7, lot D2 — migration 3702 : compteurs de numérotation. Réf. : 07_DOCUMENTS.md §8.4, §9.5, §8.14 (R-D13).
--
-- Un compteur par (établissement, entité émettrice, série, année civile). Numéro SANS TROU : l'incrément a lieu dans la
-- transaction d'émission ; si l'émission échoue, l'incrément est annulé avec elle (pas de numéro perdu). Le verrou de
-- ligne pris par l'incrément n'est tenu que le temps de cette transaction courte (un rendu, deux écritures) : deux
-- émissions de la MÊME série et de la MÊME année s'enchaînent, celles des autres séries, entités ou années ne
-- s'attendent jamais.
CREATE TABLE IF NOT EXISTS ent_doc_compteur (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    entite_id UUID NOT NULL,
    serie_code TEXT NOT NULL CHECK (serie_code IN (
        'SCO', 'ADM', 'VS', 'DIS', 'PED', 'STG', 'FIN', 'PER', 'ETB', 'CRA', 'CRD')),
    annee INT NOT NULL CHECK (annee BETWEEN 2000 AND 2999),
    dernier INT NOT NULL DEFAULT 0 CHECK (dernier >= 0),
    maj_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, entite_id, serie_code, annee),
    CONSTRAINT ent_doc_compteur_entite_cfk FOREIGN KEY (etablissement_id, entite_id)
        REFERENCES ent_doc_entite (etablissement_id, id) ON DELETE RESTRICT
);

-- Numéro suivant, atomique : un seul INSERT … ON CONFLICT DO UPDATE (deux émissions simultanées n'obtiennent jamais
-- le même numéro). À appeler UNIQUEMENT dans la transaction d'émission (jamais « pour voir »).
CREATE OR REPLACE FUNCTION ent_doc_numero_suivant(p_etab UUID, p_entite UUID, p_serie TEXT, p_annee INT)
RETURNS INT
LANGUAGE sql AS $$
    INSERT INTO ent_doc_compteur (etablissement_id, entite_id, serie_code, annee, dernier)
    VALUES (p_etab, p_entite, p_serie, p_annee, 1)
    ON CONFLICT (etablissement_id, entite_id, serie_code, annee)
    DO UPDATE SET dernier = ent_doc_compteur.dernier + 1, maj_le = now()
    RETURNING dernier
$$;

-- Un compteur ne recule jamais et ne se supprime pas.
CREATE OR REPLACE FUNCTION ent_doc_compteur_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_doc_compteur : suppression interdite' USING ERRCODE = '23001';
    END IF;
    IF NEW.dernier < OLD.dernier OR NEW.dernier > OLD.dernier + 1
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.entite_id IS DISTINCT FROM OLD.entite_id
       OR NEW.serie_code IS DISTINCT FROM OLD.serie_code
       OR NEW.annee IS DISTINCT FROM OLD.annee THEN
        RAISE EXCEPTION 'ent_doc_compteur : le compteur avance d''un pas, jamais en arrière' USING ERRCODE = '23001';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_doc_compteur_garde_trg ON ent_doc_compteur;
CREATE TRIGGER ent_doc_compteur_garde_trg
    BEFORE UPDATE OR DELETE ON ent_doc_compteur
    FOR EACH ROW EXECUTE FUNCTION ent_doc_compteur_garde();

SELECT ent_rls_poser(ARRAY['ent_doc_compteur']);
