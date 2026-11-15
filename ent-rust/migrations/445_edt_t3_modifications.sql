-- 445 — Socle 10 (emploi du temps), lot T3 : modifier le réel (docs/socles/10_EMPLOI_DU_TEMPS.md §9, §21.3).
--
-- * `ent_edt_lot` : un lot de modifications (grève, fermeture, sortie, salle fermée…) avec son aperçu chiffré,
--   sa clé d'idempotence (deux clics = un lot) et sa levée. Reprend `ent_edt_annulation_masse` (171) ;
--   les cibles sont des lignes (`ent_edt_lot_cible`), pas un tableau (D-036).
-- * `ent_edt_modification` : le JOURNAL du réel. Une ligne par séance et par changement, avec l'état avant
--   et après en colonnes typées (pas de JSON), le motif PUBLIC (liste courte + texte relu), l'origine, la
--   clé d'idempotence, le forçage motivé d'un conflit, la levée. Le motif INTERNE n'est PAS dans cette
--   table : il est dans `ent_edt_modification_interne`, que seule la direction lit (cloisonnement par table,
--   aucune lecture « SELECT * » ne peut le faire fuir).
-- * L'état d'une séance reste porté par `ent_edt_seance` (443) ; les anciens écrivains (exceptions,
--   déplacements, lots de 042/170/171) continuent d'écrire et leurs lignes sont reprises dans le journal
--   par la migration 446 (et, ensuite, par le pont de leurs écrivains).
-- * `ent_edt_seance_derive` (443) dérive désormais aussi `deplacee`, `neutralisee`, `non_assuree` et `modifiee`
--   des modifications vivantes du journal (la projection historique `statut` reste `annulee` ou `prevue`).
-- Additive, rejouable (E8). Aucune migration existante modifiée. Aucune colonne JSON ni tableau, aucune date du jour en UTC.
-- RLS d'établissement en 453 (après la définition de `ent_rls_poser`, en 580).

CREATE TABLE IF NOT EXISTS ent_edt_lot (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    cle TEXT CHECK (cle IS NULL OR char_length(cle) BETWEEN 1 AND 80),
    nature TEXT NOT NULL CHECK (nature IN
        ('annulation', 'greve', 'salle_fermee', 'neutralisation', 'changement_lieu', 'reprise')),
    du DATE NOT NULL,
    au DATE NOT NULL,
    portee TEXT NOT NULL CHECK (portee IN
        ('etablissement', 'site', 'niveau', 'classes', 'groupes', 'intervenant', 'lieu', 'seances')),
    heure_debut TIME,
    heure_fin TIME,
    motif_public_code TEXT NOT NULL CHECK (motif_public_code IN
        ('absence_professeur', 'sortie', 'examen', 'reunion', 'travaux', 'intemperies', 'mouvement_social', 'autre')),
    motif_public TEXT CHECK (motif_public IS NULL OR char_length(motif_public) <= 200),
    origine TEXT NOT NULL DEFAULT 'saisie' CHECK (origine IN
        ('saisie', 'fermeture', 'sortie', 'examen', 'calendrier', 'absence', 'reprise')),
    origine_ref UUID,
    -- Aperçu chiffré au moment de l'application (ce que le serveur a montré avant confirmation).
    nb_seances INTEGER NOT NULL DEFAULT 0 CHECK (nb_seances >= 0),
    nb_classes INTEGER NOT NULL DEFAULT 0 CHECK (nb_classes >= 0),
    nb_eleves INTEGER NOT NULL DEFAULT 0 CHECK (nb_eleves >= 0),
    nb_intervenants INTEGER NOT NULL DEFAULT 0 CHECK (nb_intervenants >= 0),
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    leve_le TIMESTAMPTZ,
    leve_par UUID,
    -- Reprise de `ent_edt_annulation_masse` (171) : lien vers l'ancien lot, une seule fois.
    ancien_masse_id UUID,
    CONSTRAINT ent_edt_lot_dates_chk CHECK (au >= du),
    CONSTRAINT ent_edt_lot_heures_chk CHECK (
        (heure_debut IS NULL AND heure_fin IS NULL) OR (heure_debut IS NOT NULL AND heure_fin > heure_debut)),
    CONSTRAINT ent_edt_lot_leve_chk CHECK ((leve_le IS NULL) = (leve_par IS NULL) OR nature = 'reprise')
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_lot_id_etab_uq ON ent_edt_lot (id, etablissement_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_lot_cle_uq ON ent_edt_lot (etablissement_id, cle) WHERE cle IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_lot_ancien_uq ON ent_edt_lot (etablissement_id, ancien_masse_id)
    WHERE ancien_masse_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_edt_lot_du_idx ON ent_edt_lot (etablissement_id, du, au);

CREATE TABLE IF NOT EXISTS ent_edt_lot_cible (
    lot_id UUID NOT NULL,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    cible_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (lot_id, cible_id),
    CONSTRAINT ent_edt_lot_cible_lot_fk FOREIGN KEY (lot_id, etablissement_id)
        REFERENCES ent_edt_lot (id, etablissement_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS ent_edt_modification (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    -- La séance modifiée. NULL seulement quand elle n'existe plus (reprise d'une exception ancienne dont la
    -- séance n'a jamais été matérialisée) : `creneau_id` et `jour` gardent alors la trace.
    seance_id UUID,
    -- Séance d'arrivée (déplacement, ajout, fusion).
    seance_arrivee_id UUID,
    lot_id UUID,
    creneau_id UUID,
    jour DATE NOT NULL,
    type TEXT NOT NULL CHECK (type IN
        ('annulation', 'deplacement', 'changement_lieu', 'changement_horaire', 'intervenant', 'ajout',
         'fusion', 'neutralisation', 'distance', 'correction')),
    avant_etat TEXT,
    apres_etat TEXT,
    avant_jour DATE,
    apres_jour DATE,
    avant_debut TIME,
    avant_fin TIME,
    apres_debut TIME,
    apres_fin TIME,
    avant_lieu_id UUID,
    apres_lieu_id UUID,
    apres_salle_texte TEXT,
    apres_remplacant_user_id UUID,
    -- Motif public : liste courte + texte libre court relu. Jamais le motif interne.
    motif_public_code TEXT NOT NULL CHECK (motif_public_code IN
        ('absence_professeur', 'sortie', 'examen', 'reunion', 'travaux', 'intemperies', 'mouvement_social', 'autre')),
    motif_public TEXT CHECK (motif_public IS NULL OR char_length(motif_public) <= 200),
    origine TEXT NOT NULL CHECK (origine IN
        ('saisie', 'absence', 'fermeture', 'sortie', 'examen', 'calendrier', 'demande', 'import', 'lot', 'reprise')),
    origine_ref UUID,
    -- Forçage d'un conflit dur par la direction : motif obligatoire et codes des conflits forcés.
    force_motif TEXT CHECK (force_motif IS NULL OR char_length(btrim(force_motif)) BETWEEN 1 AND 300),
    force_conflits TEXT,
    -- Correction d'une séance passée (§9.6).
    passe BOOLEAN NOT NULL DEFAULT false,
    cle TEXT CHECK (cle IS NULL OR char_length(cle) BETWEEN 1 AND 80),
    levee_le TIMESTAMPTZ,
    levee_par UUID,
    cree_par UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Reprise et pont des écrivains historiques (exception 042, déplacement 170, archives 2300).
    ancien_exception_id UUID,
    ancien_deplacement_id UUID,
    ancien_archive_id UUID,
    CONSTRAINT ent_edt_modification_levee_chk CHECK ((levee_le IS NULL) = (levee_par IS NULL)),
    CONSTRAINT ent_edt_modification_force_chk CHECK ((force_motif IS NULL) = (force_conflits IS NULL)),
    CONSTRAINT ent_edt_modification_heures_chk CHECK (
        (apres_debut IS NULL AND apres_fin IS NULL) OR (apres_debut IS NOT NULL AND apres_fin > apres_debut)),
    CONSTRAINT ent_edt_modification_passe_chk CHECK (NOT passe OR type = 'correction')
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_modification_id_etab_uq ON ent_edt_modification (id, etablissement_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_modification_cle_uq ON ent_edt_modification (etablissement_id, cle)
    WHERE cle IS NOT NULL;
-- Une exception historique qu'une autre décision remplace garde son identifiant (042) : plusieurs lignes du
-- journal peuvent donc porter le même `ancien_exception_id` ; seule l'archive est unique.
CREATE INDEX IF NOT EXISTS ent_edt_modification_ancien_exc_idx
    ON ent_edt_modification (etablissement_id, ancien_exception_id) WHERE ancien_exception_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_edt_modification_ancien_depl_idx
    ON ent_edt_modification (etablissement_id, ancien_deplacement_id) WHERE ancien_deplacement_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_modification_ancien_arch_uq
    ON ent_edt_modification (etablissement_id, ancien_archive_id) WHERE ancien_archive_id IS NOT NULL;
-- Une séance n'a qu'une modification vivante qui change son état (annuler, déplacer, neutraliser).
CREATE UNIQUE INDEX IF NOT EXISTS ent_edt_modification_etat_vivant_uq
    ON ent_edt_modification (etablissement_id, seance_id)
    WHERE levee_le IS NULL AND seance_id IS NOT NULL AND type IN ('annulation', 'deplacement', 'neutralisation');
CREATE INDEX IF NOT EXISTS ent_edt_modification_seance_idx
    ON ent_edt_modification (etablissement_id, seance_id) WHERE seance_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_edt_modification_lot_idx
    ON ent_edt_modification (etablissement_id, lot_id) WHERE lot_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ent_edt_modification_jour_idx ON ent_edt_modification (etablissement_id, jour);
CREATE INDEX IF NOT EXISTS ent_edt_modification_creneau_idx
    ON ent_edt_modification (etablissement_id, creneau_id, jour) WHERE creneau_id IS NOT NULL;

-- Motif INTERNE : table à part, lue seulement par la direction et le secrétariat (§9.1, R-E8).
CREATE TABLE IF NOT EXISTS ent_edt_modification_interne (
    modification_id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    motif_interne TEXT NOT NULL CHECK (char_length(btrim(motif_interne)) BETWEEN 1 AND 500),
    cree_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_edt_modification_interne_fk FOREIGN KEY (modification_id, etablissement_id)
        REFERENCES ent_edt_modification (id, etablissement_id) ON DELETE RESTRICT
);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_modification_seance_fk') THEN
        -- La séance est retirée, jamais supprimée quand quelque chose s'y attache ; si elle disparaît quand
        -- même, le journal garde la trace (créneau, jour) et perd seulement le lien.
        ALTER TABLE ent_edt_modification ADD CONSTRAINT ent_edt_modification_seance_fk
            FOREIGN KEY (seance_id, etablissement_id) REFERENCES ent_edt_seance (id, etablissement_id)
            ON DELETE SET NULL (seance_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_modification_arrivee_fk') THEN
        ALTER TABLE ent_edt_modification ADD CONSTRAINT ent_edt_modification_arrivee_fk
            FOREIGN KEY (seance_arrivee_id, etablissement_id) REFERENCES ent_edt_seance (id, etablissement_id)
            ON DELETE SET NULL (seance_arrivee_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_modification_lot_fk') THEN
        ALTER TABLE ent_edt_modification ADD CONSTRAINT ent_edt_modification_lot_fk
            FOREIGN KEY (lot_id, etablissement_id) REFERENCES ent_edt_lot (id, etablissement_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_edt_modification_lieu_fk') THEN
        ALTER TABLE ent_edt_modification ADD CONSTRAINT ent_edt_modification_lieu_fk
            FOREIGN KEY (apres_lieu_id, etablissement_id) REFERENCES ent_salle (id, etablissement_id);
    END IF;
END
$$;

-- Le journal est en ajout seul : on ne supprime rien, on ne réécrit rien ; seules la levée (et le détachement
-- d'une séance disparue) se posent après coup.
CREATE OR REPLACE FUNCTION ent_edt_modification_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_edt_modification est en ajout seul (DELETE interdit)' USING ERRCODE = '23514';
    END IF;
    IF (NEW.id, NEW.etablissement_id, NEW.lot_id, NEW.jour, NEW.type, NEW.avant_etat, NEW.apres_etat,
        NEW.avant_jour, NEW.apres_jour, NEW.avant_debut, NEW.avant_fin, NEW.apres_debut, NEW.apres_fin,
        NEW.avant_lieu_id, NEW.apres_lieu_id, NEW.apres_remplacant_user_id, NEW.motif_public_code,
        NEW.motif_public, NEW.origine, NEW.origine_ref, NEW.force_motif, NEW.force_conflits, NEW.passe,
        NEW.cle, NEW.cree_par, NEW.cree_le)
       IS DISTINCT FROM
       (OLD.id, OLD.etablissement_id, OLD.lot_id, OLD.jour, OLD.type, OLD.avant_etat, OLD.apres_etat,
        OLD.avant_jour, OLD.apres_jour, OLD.avant_debut, OLD.avant_fin, OLD.apres_debut, OLD.apres_fin,
        OLD.avant_lieu_id, OLD.apres_lieu_id, OLD.apres_remplacant_user_id, OLD.motif_public_code,
        OLD.motif_public, OLD.origine, OLD.origine_ref, OLD.force_motif, OLD.force_conflits, OLD.passe,
        OLD.cle, OLD.cree_par, OLD.cree_le) THEN
        RAISE EXCEPTION 'ent_edt_modification est en ajout seul (seule la levée se pose après coup)'
            USING ERRCODE = '23514';
    END IF;
    IF OLD.levee_le IS NOT NULL AND (NEW.levee_le IS DISTINCT FROM OLD.levee_le
                                     OR NEW.levee_par IS DISTINCT FROM OLD.levee_par) THEN
        RAISE EXCEPTION 'une modification levée ne se lève pas deux fois' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS ent_edt_modification_ajout_seul_tg ON ent_edt_modification;
CREATE TRIGGER ent_edt_modification_ajout_seul_tg BEFORE UPDATE OR DELETE ON ent_edt_modification
    FOR EACH ROW EXECUTE FUNCTION ent_edt_modification_ajout_seul();

-- Dérivation de l'état (remplace la fonction de 443, sans toucher au fichier). L'état est DÉRIVÉ du statut
-- historique et des modifications vivantes du journal (§8.2 : « états dérivés, stockés pour la lecture
-- rapide, recalculables ») : une séance de statut `annulee` est `deplacee` ou `neutralisee` si une
-- modification vivante de ce type la porte, `non_assuree` si l'écrivain le pose explicitement ; une séance
-- `prevue` ou `remplacee` est `modifiee` si une modification vivante de lieu, d'horaire, d'intervenant ou de
-- fusion la porte. L'insertion de la modification précède donc l'écriture de la séance ; un écrivain qui a
-- posé la ligne après coup relance le calcul par `UPDATE ent_edt_seance SET statut = statut`.
CREATE OR REPLACE FUNCTION ent_edt_seance_derive() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    fuseau TEXT;
    explicite TEXT := NEW.etat;
    portee TEXT;
BEGIN
    NEW.etat := CASE
        WHEN NEW.statut = 'retiree' THEN 'retiree'
        WHEN NEW.statut = 'annulee' THEN 'annulee'
        WHEN NEW.statut = 'remplacee' THEN 'modifiee'
        WHEN NEW.source IN ('deplacement', 'manuel') THEN 'ajoutee'
        ELSE 'prevue'
    END;
    IF NEW.statut = 'annulee' THEN
        SELECT m.type INTO portee FROM ent_edt_modification m
         WHERE m.etablissement_id = NEW.etablissement_id AND m.seance_id = NEW.id AND m.levee_le IS NULL
           AND m.type IN ('neutralisation', 'deplacement')
         ORDER BY m.cree_le DESC LIMIT 1;
        IF portee = 'neutralisation' THEN NEW.etat := 'neutralisee';
        ELSIF portee = 'deplacement' THEN NEW.etat := 'deplacee';
        ELSIF explicite = 'non_assuree' THEN NEW.etat := 'non_assuree';
        END IF;
    ELSIF NEW.statut IN ('prevue', 'remplacee') AND NEW.etat IN ('prevue', 'modifiee') THEN
        IF EXISTS (SELECT 1 FROM ent_edt_modification m
                    WHERE m.etablissement_id = NEW.etablissement_id AND m.seance_id = NEW.id
                      AND m.levee_le IS NULL
                      AND m.type IN ('changement_lieu', 'changement_horaire', 'intervenant', 'fusion', 'distance')) THEN
            NEW.etat := 'modifiee';
        END IF;
    END IF;
    NEW.origine := CASE NEW.source
        WHEN 'deplacement' THEN 'deplacement'
        WHEN 'manuel' THEN 'ajout'
        ELSE COALESCE(CASE WHEN NEW.origine IN ('fusion', 'import') THEN NEW.origine END, 'plan')
    END;
    IF TG_OP = 'INSERT' OR NEW.jour IS DISTINCT FROM OLD.jour
       OR NEW.heure_debut IS DISTINCT FROM OLD.heure_debut OR NEW.heure_fin IS DISTINCT FROM OLD.heure_fin
       OR NEW.debut_utc IS NULL THEN
        SELECT p.fuseau INTO fuseau FROM ent_etablissement_parametre p
         WHERE p.etablissement_id = NEW.etablissement_id;
        fuseau := COALESCE(fuseau, 'Europe/Paris');
        NEW.debut_utc := (NEW.jour + NEW.heure_debut) AT TIME ZONE fuseau;
        NEW.fin_utc := (NEW.jour + NEW.heure_fin) AT TIME ZONE fuseau;
    END IF;
    RETURN NEW;
END
$$;
