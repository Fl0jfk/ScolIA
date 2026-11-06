-- Socle 2, lot S2-7 : bulletin périodique versionné et immuable (EVA-10, EVA-14),
-- archivé au dossier de l'élève ; trace des exports de livrets (EVA-12).

CREATE TABLE IF NOT EXISTS ent_bulletin (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    eleve_id UUID NOT NULL REFERENCES ent_eleve (id),
    periode_id UUID NOT NULL REFERENCES ent_note_periode (id),
    version INTEGER NOT NULL,
    statut TEXT NOT NULL DEFAULT 'genere',
    -- Instantané complet (notes, moyennes, appréciations, décisions) : le PDF en dérive.
    contenu JSONB NOT NULL,
    empreinte TEXT NOT NULL,
    storage_key TEXT NOT NULL,
    octets INTEGER NOT NULL,
    motif TEXT,
    genere_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    genere_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    publie_par_user_id UUID REFERENCES ent_user (id),
    publie_le TIMESTAMPTZ,
    document_id UUID REFERENCES ent_eleve_document (id),
    CONSTRAINT ent_bulletin_version_uq UNIQUE (etablissement_id, eleve_id, periode_id, version),
    CONSTRAINT ent_bulletin_version_chk CHECK (version >= 1),
    CONSTRAINT ent_bulletin_statut_chk CHECK (statut IN ('genere', 'publie', 'remplace'))
);
-- Une seule version publiée à la fois par élève et période.
CREATE UNIQUE INDEX IF NOT EXISTS ent_bulletin_publie_uq
    ON ent_bulletin (etablissement_id, eleve_id, periode_id) WHERE statut = 'publie';
CREATE INDEX IF NOT EXISTS ent_bulletin_periode_idx
    ON ent_bulletin (etablissement_id, periode_id);

-- Immuabilité : le contenu d'une version ne change jamais ; seules les transitions
-- genere → publie, genere → remplace, publie → remplace sont admises ; aucune suppression.
CREATE OR REPLACE FUNCTION ent_trg_bulletin_immuable() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'bulletin archive : suppression interdite' USING ERRCODE = '23514';
    END IF;
    IF NEW.contenu IS DISTINCT FROM OLD.contenu
       OR NEW.empreinte IS DISTINCT FROM OLD.empreinte
       OR NEW.storage_key IS DISTINCT FROM OLD.storage_key
       OR NEW.version IS DISTINCT FROM OLD.version
       OR NEW.eleve_id IS DISTINCT FROM OLD.eleve_id
       OR NEW.periode_id IS DISTINCT FROM OLD.periode_id
       OR NEW.etablissement_id IS DISTINCT FROM OLD.etablissement_id
       OR NEW.genere_le IS DISTINCT FROM OLD.genere_le THEN
        RAISE EXCEPTION 'bulletin immuable' USING ERRCODE = '23514';
    END IF;
    IF NEW.statut IS DISTINCT FROM OLD.statut AND NOT (
           (OLD.statut = 'genere' AND NEW.statut IN ('publie', 'remplace'))
        OR (OLD.statut = 'publie' AND NEW.statut = 'remplace')) THEN
        RAISE EXCEPTION 'transition de bulletin interdite' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS ent_bulletin_immuable ON ent_bulletin;
CREATE TRIGGER ent_bulletin_immuable BEFORE UPDATE OR DELETE ON ent_bulletin
    FOR EACH ROW EXECUTE FUNCTION ent_trg_bulletin_immuable();

-- Trace des exports de livrets (LSU, LSL) : qui, quand, quoi, empreinte du fichier.
CREATE TABLE IF NOT EXISTS ent_eva_export (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    format TEXT NOT NULL,
    periode_id UUID NOT NULL REFERENCES ent_note_periode (id),
    classe_id UUID REFERENCES ent_classe (id),
    nb_eleves INTEGER NOT NULL,
    empreinte TEXT NOT NULL,
    conforme_verifie BOOLEAN NOT NULL DEFAULT false,
    exporte_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    exporte_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_eva_export_format_chk CHECK (format IN ('lsu', 'lsl'))
);
CREATE INDEX IF NOT EXISTS ent_eva_export_etab_idx ON ent_eva_export (etablissement_id, exporte_le DESC);
