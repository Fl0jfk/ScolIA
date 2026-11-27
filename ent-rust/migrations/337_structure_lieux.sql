-- Socle 5, lot S2 (`docs/socles/05_STRUCTURE.md` §9.1 et §9.10, STR-05 à STR-09, STR-109, STR-110).
--
-- Sites enrichis, bâtiments, salles typées liées à leur site, lieux extérieurs, fenêtres de
-- disponibilité et indisponibilités (travaux, réquisition, lot d'événement).
--
-- Règles tenues PAR LA BASE (pas seulement par le service) :
--   * une salle appartient à un site DE SON ÉTABLISSEMENT (clé étrangère composite
--     `(site_id, etablissement_id)`) et ne peut plus en être détachée ;
--   * toute nouvelle salle a un site (déclencheur) : aucun site n'est jamais créé implicitement,
--     une salle dont le site texte est inconnu est REFUSÉE ;
--   * un bâtiment de salle appartient au même site que la salle (clé `(batiment_id, site_id)`) ;
--   * un lieu extérieur ou un bâtiment ne se croise jamais entre établissements.
-- Les salles historiques (colonne texte `site`, sans `site_id`) sont rattachées quand le texte
-- désigne exactement un site existant du MÊME établissement ; les autres restent sans site et sont
-- signalées par le service (« sans_site »). Rien n'est créé ni deviné.
--
-- Migration additive et rejouable.

-- ───────────────────────── Sites ─────────────────────────
ALTER TABLE ent_site ADD COLUMN IF NOT EXISTS degres TEXT[] NOT NULL DEFAULT '{}';
ALTER TABLE ent_site ADD COLUMN IF NOT EXISTS couleur TEXT;
-- Exception de fuseau d'un site (annexe à l'étranger) ; NULL = fuseau de l'établissement.
ALTER TABLE ent_site ADD COLUMN IF NOT EXISTS fuseau_horaire TEXT;
ALTER TABLE ent_site ADD COLUMN IF NOT EXISTS ferme_le DATE;

ALTER TABLE ent_site DROP CONSTRAINT IF EXISTS ent_site_degres_chk;
ALTER TABLE ent_site ADD CONSTRAINT ent_site_degres_chk
    CHECK (degres <@ ARRAY['premier', 'second', 'post_bac']::TEXT[]);
ALTER TABLE ent_site DROP CONSTRAINT IF EXISTS ent_site_couleur_chk;
ALTER TABLE ent_site ADD CONSTRAINT ent_site_couleur_chk
    CHECK (couleur IS NULL OR couleur ~ '^#[0-9A-Fa-f]{6}$');
ALTER TABLE ent_site DROP CONSTRAINT IF EXISTS ent_site_fuseau_chk;
ALTER TABLE ent_site ADD CONSTRAINT ent_site_fuseau_chk
    CHECK (fuseau_horaire IS NULL
           OR (length(fuseau_horaire) BETWEEN 1 AND 64
               AND timezone(fuseau_horaire, TIMESTAMPTZ '2000-01-01 00:00:00+00') IS NOT NULL));
-- Lignes historiques non contrôlées : la contrainte vaut pour toute écriture à venir.
ALTER TABLE ent_site DROP CONSTRAINT IF EXISTS ent_site_uai_chk;
ALTER TABLE ent_site ADD CONSTRAINT ent_site_uai_chk
    CHECK (uai IS NULL OR uai ~ '^[0-9]{7}[A-Z]$') NOT VALID;
-- Cible des clés étrangères composites (cloisonnement par établissement).
CREATE UNIQUE INDEX IF NOT EXISTS ent_site_id_etab_uq ON ent_site (id, etablissement_id);

-- ───────────────────────── Bâtiments ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_batiment (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    site_id UUID NOT NULL,
    code TEXT NOT NULL CHECK (length(btrim(code)) BETWEEN 1 AND 40),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    ferme_le DATE,
    created_by UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_batiment_site_fk FOREIGN KEY (site_id, etablissement_id)
        REFERENCES ent_site (id, etablissement_id),
    CONSTRAINT ent_batiment_code_uq UNIQUE (etablissement_id, site_id, code),
    CONSTRAINT ent_batiment_id_site_uq UNIQUE (id, site_id)
);

-- ───────────────────────── Lieux extérieurs (STR-109) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_lieu (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    code TEXT NOT NULL CHECK (length(btrim(code)) BETWEEN 1 AND 40),
    libelle TEXT NOT NULL CHECK (length(btrim(libelle)) BETWEEN 1 AND 120),
    adresse TEXT CHECK (adresse IS NULL OR length(adresse) <= 400),
    -- Propriétaire / gestionnaire (mairie, département, club…).
    proprietaire TEXT CHECK (proprietaire IS NULL OR length(proprietaire) <= 200),
    -- Convention : document du socle 7 (pas de clé étrangère tant que ce socle n'existe pas).
    convention_document_id UUID,
    trajet_minutes SMALLINT CHECK (trajet_minutes IS NULL OR trajet_minutes BETWEEN 0 AND 240),
    mode_trajet TEXT CHECK (mode_trajet IS NULL OR mode_trajet IN ('pied', 'car', 'transport_commun', 'autre')),
    ferme_le DATE,
    created_by UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_lieu_code_uq UNIQUE (etablissement_id, code),
    CONSTRAINT ent_lieu_id_etab_uq UNIQUE (id, etablissement_id)
);

-- ───────────────────────── Salles ─────────────────────────
ALTER TABLE ent_salle ADD COLUMN IF NOT EXISTS site_id UUID;
ALTER TABLE ent_salle ADD COLUMN IF NOT EXISTS batiment_id UUID;
ALTER TABLE ent_salle ADD COLUMN IF NOT EXISTS lieu_id UUID;
ALTER TABLE ent_salle ADD COLUMN IF NOT EXISTS etage TEXT;
ALTER TABLE ent_salle ADD COLUMN IF NOT EXISTS type TEXT NOT NULL DEFAULT 'banalisee';
-- Libellés libres de l'établissement (vidéoprojecteur, paillasses…).
ALTER TABLE ent_salle ADD COLUMN IF NOT EXISTS equipements TEXT[] NOT NULL DEFAULT '{}';
ALTER TABLE ent_salle ADD COLUMN IF NOT EXISTS accessible_pmr BOOLEAN;
-- Lu par le socle 44 (réservation) : une salle n'est réservable que si on l'a décidé.
ALTER TABLE ent_salle ADD COLUMN IF NOT EXISTS reservable BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE ent_salle ADD COLUMN IF NOT EXISTS ferme_le DATE;

ALTER TABLE ent_salle DROP CONSTRAINT IF EXISTS ent_salle_type_chk;
ALTER TABLE ent_salle ADD CONSTRAINT ent_salle_type_chk CHECK (type IN
    ('banalisee', 'laboratoire', 'informatique', 'gymnase', 'atelier', 'cdi', 'musique', 'arts',
     'amphitheatre', 'exterieur', 'autre'));
ALTER TABLE ent_salle DROP CONSTRAINT IF EXISTS ent_salle_equipements_chk;
ALTER TABLE ent_salle ADD CONSTRAINT ent_salle_equipements_chk CHECK (cardinality(equipements) <= 40);
-- Un lieu extérieur n'accueille que des salles de type « exterieur ».
ALTER TABLE ent_salle DROP CONSTRAINT IF EXISTS ent_salle_lieu_type_chk;
ALTER TABLE ent_salle ADD CONSTRAINT ent_salle_lieu_type_chk CHECK (lieu_id IS NULL OR type = 'exterieur');

CREATE UNIQUE INDEX IF NOT EXISTS ent_salle_id_etab_uq ON ent_salle (id, etablissement_id);

ALTER TABLE ent_salle DROP CONSTRAINT IF EXISTS ent_salle_site_fk;
ALTER TABLE ent_salle ADD CONSTRAINT ent_salle_site_fk FOREIGN KEY (site_id, etablissement_id)
    REFERENCES ent_site (id, etablissement_id);
-- Le bâtiment doit appartenir au site de la salle (MATCH SIMPLE : ignoré quand batiment_id est NULL).
ALTER TABLE ent_salle DROP CONSTRAINT IF EXISTS ent_salle_batiment_fk;
ALTER TABLE ent_salle ADD CONSTRAINT ent_salle_batiment_fk FOREIGN KEY (batiment_id, site_id)
    REFERENCES ent_batiment (id, site_id);
ALTER TABLE ent_salle DROP CONSTRAINT IF EXISTS ent_salle_lieu_fk;
ALTER TABLE ent_salle ADD CONSTRAINT ent_salle_lieu_fk FOREIGN KEY (lieu_id, etablissement_id)
    REFERENCES ent_lieu (id, etablissement_id);

CREATE INDEX IF NOT EXISTS ent_salle_site_idx ON ent_salle (etablissement_id, site_id);

-- Rattachement des salles historiques : correspondance EXACTE (code ou libellé, sans la casse)
-- avec un site du même établissement. Aucune création, aucun choix par défaut.
UPDATE ent_salle s
   SET site_id = (SELECT x.id FROM ent_site x
                   WHERE x.etablissement_id = s.etablissement_id
                     AND (lower(x.code) = lower(btrim(s.site)) OR lower(x.libelle) = lower(btrim(s.site)))
                   ORDER BY (lower(x.code) = lower(btrim(s.site))) DESC, x.code
                   LIMIT 1)
 WHERE s.site_id IS NULL AND s.site IS NOT NULL AND btrim(s.site) <> '';

-- Garde : site résolu depuis le texte historique si besoin, jamais créé ; obligatoire à la
-- création ; une salle déjà rattachée ne peut plus en être détachée ; le texte `site` reste le
-- miroir du code du site (lecteurs historiques).
CREATE OR REPLACE FUNCTION ent_salle_site_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE v UUID;
BEGIN
    IF NEW.site_id IS NULL THEN
        IF TG_OP = 'UPDATE' AND OLD.site_id IS NOT NULL THEN
            RAISE EXCEPTION 'une salle ne peut pas être détachée de son site' USING ERRCODE = '23514';
        END IF;
        IF NEW.site IS NOT NULL AND btrim(NEW.site) <> '' THEN
            SELECT x.id INTO v FROM ent_site x
             WHERE x.etablissement_id = NEW.etablissement_id
               AND (lower(x.code) = lower(btrim(NEW.site)) OR lower(x.libelle) = lower(btrim(NEW.site)))
             ORDER BY (lower(x.code) = lower(btrim(NEW.site))) DESC, x.code
             LIMIT 1;
            NEW.site_id := v;
        END IF;
    END IF;
    IF NEW.site_id IS NULL AND TG_OP = 'INSERT' THEN
        RAISE EXCEPTION 'une salle doit être liée à un site existant de son établissement' USING ERRCODE = '23514';
    END IF;
    IF NEW.site_id IS NOT NULL THEN
        SELECT x.code INTO NEW.site FROM ent_site x
         WHERE x.id = NEW.site_id AND x.etablissement_id = NEW.etablissement_id;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_salle_site_garde ON ent_salle;
CREATE TRIGGER ent_salle_site_garde BEFORE INSERT OR UPDATE ON ent_salle
    FOR EACH ROW EXECUTE FUNCTION ent_salle_site_garde();

-- ───────────────────────── Disponibilités et indisponibilités ─────────────────────────
-- Fenêtres hebdomadaires attribuées à une salle ou à un lieu (R-S14). Sans aucune fenêtre, la
-- cible est disponible sur les heures ouvrées ; dès qu'une fenêtre existe, elle ne l'est que
-- dans ses fenêtres (moins ses indisponibilités).
CREATE TABLE IF NOT EXISTS ent_disponibilite (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    salle_id UUID,
    lieu_id UUID,
    jour_semaine SMALLINT NOT NULL CHECK (jour_semaine BETWEEN 1 AND 7),
    heure_debut TIME NOT NULL,
    heure_fin TIME NOT NULL,
    du DATE,
    au DATE,
    libelle TEXT CHECK (libelle IS NULL OR length(libelle) <= 120),
    annulee_le TIMESTAMPTZ,
    created_by UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_disponibilite_cible_chk CHECK ((salle_id IS NULL) <> (lieu_id IS NULL)),
    CONSTRAINT ent_disponibilite_heures_chk CHECK (heure_fin > heure_debut),
    CONSTRAINT ent_disponibilite_dates_chk CHECK (du IS NULL OR au IS NULL OR au >= du),
    CONSTRAINT ent_disponibilite_salle_fk FOREIGN KEY (salle_id, etablissement_id)
        REFERENCES ent_salle (id, etablissement_id),
    CONSTRAINT ent_disponibilite_lieu_fk FOREIGN KEY (lieu_id, etablissement_id)
        REFERENCES ent_lieu (id, etablissement_id)
);
CREATE INDEX IF NOT EXISTS ent_disponibilite_salle_idx ON ent_disponibilite (etablissement_id, salle_id)
    WHERE annulee_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_disponibilite_lieu_idx ON ent_disponibilite (etablissement_id, lieu_id)
    WHERE annulee_le IS NULL;

-- Indisponibilités datées (travaux de la mairie, réquisition pour un examen). `lot_id` regroupe
-- les salles rendues indisponibles en un geste (STR-110) ; `evenement_ref` pointera le calendrier
-- (socle 5, lot S3) sans clé étrangère tant qu'il n'existe pas. Annulées, jamais supprimées.
CREATE TABLE IF NOT EXISTS ent_salle_indisponibilite (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    salle_id UUID,
    lieu_id UUID,
    du TIMESTAMPTZ NOT NULL,
    au TIMESTAMPTZ NOT NULL,
    motif_code TEXT NOT NULL CHECK (motif_code IN
        ('travaux', 'evenement', 'examen', 'requisition', 'securite', 'autre')),
    motif TEXT CHECK (motif IS NULL OR length(motif) <= 300),
    lot_id UUID,
    evenement_ref UUID,
    annulee_le TIMESTAMPTZ,
    annulee_par UUID,
    par UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_indispo_cible_chk CHECK ((salle_id IS NULL) <> (lieu_id IS NULL)),
    CONSTRAINT ent_indispo_periode_chk CHECK (au > du),
    CONSTRAINT ent_indispo_salle_fk FOREIGN KEY (salle_id, etablissement_id)
        REFERENCES ent_salle (id, etablissement_id),
    CONSTRAINT ent_indispo_lieu_fk FOREIGN KEY (lieu_id, etablissement_id)
        REFERENCES ent_lieu (id, etablissement_id)
);
CREATE INDEX IF NOT EXISTS ent_indispo_salle_idx ON ent_salle_indisponibilite (etablissement_id, salle_id, du)
    WHERE annulee_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_indispo_lieu_idx ON ent_salle_indisponibilite (etablissement_id, lieu_id, du)
    WHERE annulee_le IS NULL;
CREATE INDEX IF NOT EXISTS ent_indispo_lot_idx ON ent_salle_indisponibilite (etablissement_id, lot_id)
    WHERE lot_id IS NOT NULL;

-- Une indisponibilité ne se modifie que pour être annulée.
CREATE OR REPLACE FUNCTION ent_indispo_immuable() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'une indisponibilité s''annule, elle ne se supprime pas' USING ERRCODE = '42501';
    END IF;
    IF (NEW.id, NEW.etablissement_id, NEW.salle_id, NEW.lieu_id, NEW.du, NEW.au, NEW.motif_code,
        NEW.motif, NEW.lot_id, NEW.evenement_ref, NEW.par)
       IS DISTINCT FROM
       (OLD.id, OLD.etablissement_id, OLD.salle_id, OLD.lieu_id, OLD.du, OLD.au, OLD.motif_code,
        OLD.motif, OLD.lot_id, OLD.evenement_ref, OLD.par) THEN
        RAISE EXCEPTION 'une indisponibilité ne se modifie pas (annuler puis recréer)' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_indispo_immuable ON ent_salle_indisponibilite;
CREATE TRIGGER ent_indispo_immuable BEFORE UPDATE OR DELETE ON ent_salle_indisponibilite
    FOR EACH ROW EXECUTE FUNCTION ent_indispo_immuable();

-- ───────────────────────── RLS ─────────────────────────
DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_batiment', 'ent_lieu', 'ent_disponibilite', 'ent_salle_indisponibilite'] LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('DROP POLICY IF EXISTS ent_rls_etablissement ON %I', t);
        EXECUTE format(
            'CREATE POLICY ent_rls_etablissement ON %I FOR ALL
                USING (etablissement_id = (SELECT ent_rls_etablissement())
                    OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
                WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
                    OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))', t);
    END LOOP;
END
$$;
