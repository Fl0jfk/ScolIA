-- Socle 18, lot FO3 — migration 2367 : dossier de preuve d'une décision par la brique de signature du socle 7 (lot D6)
-- (`docs/socles/18_FORMULAIRES.md` §10, §21.1 , `docs/socles/DOCUMENTS_D6.md` , règles R-FC15, R-FC44 , action FRM-119).
--
-- Additif, rejouable, sans JSON et sans date UTC. Cette migration référence les tables de signature du socle 7 (3705) : elle est
-- donc appliquée **après** elles (l'ordre d'application est celui du tableau `MIGRATIONS`) et son numéro reste sous 3808.
--
-- * `ent_acc_document_lien` : le document émis (socle 7) qui porte le texte de l'accord et que le signataire signe. Il est rattaché
--   à une **version** d'objet et à un **élève** : une signature sur un autre document, une autre version ou un autre élève ne
--   prouve rien pour cette décision.
-- * `ent_acc_preuve` : la signature (D6) qui prouve une décision, une seule fois. Niveau et méthode sont ceux de la signature,
--   jamais relevés. Les limites de D6 restent entières : le jeton d'horodatage est « reçu, non vérifié » et la tablette sans
--   confirmation (G0) n'est jamais une preuve ici.

CREATE TABLE IF NOT EXISTS ent_acc_document_lien (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    document_id UUID NOT NULL,
    objet_id UUID NOT NULL,
    version_id UUID NOT NULL,
    sujet_eleve_id UUID NOT NULL,
    lie_par UUID NOT NULL,
    lie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_acc_document_lien_uq UNIQUE (etablissement_id, document_id, objet_id, version_id, sujet_eleve_id),
    CONSTRAINT ent_acc_document_lien_document_cfk FOREIGN KEY (etablissement_id, document_id)
        REFERENCES ent_doc_document (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_acc_document_lien_version_cfk FOREIGN KEY (etablissement_id, objet_id, version_id)
        REFERENCES ent_acc_version (etablissement_id, objet_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_acc_document_lien_eleve_cfk FOREIGN KEY (etablissement_id, sujet_eleve_id)
        REFERENCES ent_eleve (etablissement_id, id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_acc_document_lien_document_idx ON ent_acc_document_lien (etablissement_id, document_id);

CREATE TABLE IF NOT EXISTS ent_acc_preuve (
    decision_id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    signature_id UUID NOT NULL,
    circuit_id UUID NOT NULL,
    niveau TEXT NOT NULL CHECK (niveau IN ('S1', 'S2', 'G', 'P')),
    methode TEXT NOT NULL CHECK (methode IN ('passkey', 'code_sms', 'guichet', 'papier')),
    lie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_acc_preuve_signature_uq UNIQUE (signature_id),
    CONSTRAINT ent_acc_preuve_decision_cfk FOREIGN KEY (etablissement_id, decision_id)
        REFERENCES ent_acc_decision (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_acc_preuve_signature_cfk FOREIGN KEY (etablissement_id, signature_id)
        REFERENCES ent_doc_signature (etablissement_id, id) ON DELETE RESTRICT,
    CONSTRAINT ent_acc_preuve_circuit_cfk FOREIGN KEY (etablissement_id, circuit_id)
        REFERENCES ent_doc_circuit (etablissement_id, id) ON DELETE RESTRICT
);

CREATE OR REPLACE FUNCTION ent_acc_preuve_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF current_setting('ent.purge_etablissement', true) = 'on' THEN
        RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    END IF;
    RAISE EXCEPTION '% : table en ajout seul', TG_TABLE_NAME USING ERRCODE = '42501';
END $$;
DROP TRIGGER IF EXISTS ent_acc_preuve_garde ON ent_acc_preuve;
CREATE TRIGGER ent_acc_preuve_garde BEFORE UPDATE OR DELETE ON ent_acc_preuve
    FOR EACH ROW EXECUTE FUNCTION ent_acc_preuve_garde();
DROP TRIGGER IF EXISTS ent_acc_document_lien_garde ON ent_acc_document_lien;
CREATE TRIGGER ent_acc_document_lien_garde BEFORE UPDATE OR DELETE ON ent_acc_document_lien
    FOR EACH ROW EXECUTE FUNCTION ent_acc_preuve_garde();

SELECT ent_rls_poser(ARRAY['ent_acc_document_lien', 'ent_acc_preuve']);
