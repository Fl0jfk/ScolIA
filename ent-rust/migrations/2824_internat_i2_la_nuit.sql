-- Socle 41 (internat), lot I2 « La nuit » — migration 2824 : attendu, appels, cas et procédures,
-- transmission du matin, rappels, journal des lectures sensibles.
-- Réf. : docs/socles/41_INTERNAT.md §9, §17, §20 (R-IT3, R-IT5, R-IT13, R-IT14, R-IT16 à R-IT18, R-IT26,
-- R-IT30, R-IT31, R-IT34, R-IT38, R-IT40), §22, §24 ; NI-17 à NI-25 ; DI-01, DI-11, DI-13, DI-19, DI-23.
-- Additive et rejouable (E8). Aucune colonne JSON (D-036), aucun BYTEA (E5), aucune donnée de santé (R-IT6).
-- Le PDF de l'appel est produit à la demande depuis ces tables : aucun binaire n'est stocké.

-- ───────────────────────── 1. Repères de temps de la nuit (R-IT5) ─────────────────────────
CREATE OR REPLACE FUNCTION ent_internat_fuseau(etab UUID) RETURNS TEXT
LANGUAGE sql STABLE AS $$
    SELECT COALESCE(
        (SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = etab),
        'Europe/Paris')
$$;

-- Instant de référence d'une nuit : 22 h 00, heure murale de l'établissement (cette heure existe
-- toujours, y compris la nuit des changements d'heure, qui se jouent à 2 h ou 3 h du matin).
CREATE OR REPLACE FUNCTION ent_internat_ancre_nuit(etab UUID, nuit DATE) RETURNS TIMESTAMPTZ
LANGUAGE sql STABLE AS $$
    SELECT (nuit + TIME '22:00') AT TIME ZONE ent_internat_fuseau(etab)
$$;

-- ───────────────────────── 2. Nuits exceptionnelles et retraits (adaptateur) ─────────────────────────
-- Nuit exceptionnelle : élève accueilli une nuit de plus (examen, neige, week-end d'un élève éloigné).
CREATE TABLE IF NOT EXISTS ent_internat_nuit_exceptionnelle (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    nuit DATE NOT NULL,
    motif_code TEXT NOT NULL CHECK (motif_code IN (
        'examen', 'accueil_weekend', 'intemperies', 'delai_accorde', 'decision_direction', 'autre')),
    par_user_id UUID NOT NULL REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    annulee_le TIMESTAMPTZ,
    annulee_par_user_id UUID REFERENCES ent_user (id),
    CONSTRAINT ent_internat_nuit_exceptionnelle_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_nuit_exceptionnelle_uq UNIQUE (etablissement_id, eleve_id, nuit),
    CONSTRAINT ent_internat_nuit_exceptionnelle_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_internat_nuit_exceptionnelle_annulation_chk CHECK (
        (annulee_le IS NULL) = (annulee_par_user_id IS NULL))
);

-- Retrait de l'attendu : adaptateur par lequel les autres socles (12 vie scolaire, 13 exclusion
-- d'internat, 34 hospitalisation, 39 stage, I3 absences annoncées) disent « cet élève ne dort pas
-- ici ». Le motif est un CODE, jamais un texte (R-IT6) ; l'hospitalisation s'écrit « sante » sans détail.
CREATE TABLE IF NOT EXISTS ent_internat_retrait (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    du_nuit DATE NOT NULL,
    au_nuit DATE NOT NULL,
    motif_code TEXT NOT NULL CHECK (motif_code IN (
        'absence_annoncee', 'sortie', 'mobilite', 'stage', 'sejour_scolaire',
        'exclusion_internat', 'fermeture', 'sante')),
    source TEXT NOT NULL CHECK (source IN (
        'cpe', 'direction', 'vie_scolaire', 'infirmerie', 'absence_internat', 'reprise')),
    par_user_id UUID NOT NULL REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    annule_le TIMESTAMPTZ,
    annule_par_user_id UUID REFERENCES ent_user (id),
    CONSTRAINT ent_internat_retrait_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_retrait_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_internat_retrait_periode_chk CHECK (au_nuit >= du_nuit AND au_nuit - du_nuit <= 400),
    CONSTRAINT ent_internat_retrait_annulation_chk CHECK ((annule_le IS NULL) = (annule_par_user_id IS NULL))
);
CREATE INDEX IF NOT EXISTS ent_internat_retrait_eleve_idx
    ON ent_internat_retrait (etablissement_id, eleve_id, du_nuit, au_nuit) WHERE annule_le IS NULL;

-- Ni suppression, ni modification autre que l'annulation (R-IT3).
CREATE OR REPLACE FUNCTION ent_internat_annulable_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    a_avant TIMESTAMPTZ;
    a_apres TIMESTAMPTZ;
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION '% : suppression interdite (R-IT3), annuler par un fait daté', TG_TABLE_NAME
            USING ERRCODE = '42501';
    END IF;
    IF TG_TABLE_NAME = 'ent_internat_retrait' THEN
        a_avant := OLD.annule_le;
        a_apres := NEW.annule_le;
        IF (OLD.id, OLD.etablissement_id, OLD.eleve_id, OLD.du_nuit, OLD.au_nuit, OLD.motif_code, OLD.source,
            OLD.par_user_id, OLD.cree_le)
           IS DISTINCT FROM (NEW.id, NEW.etablissement_id, NEW.eleve_id, NEW.du_nuit, NEW.au_nuit,
            NEW.motif_code, NEW.source, NEW.par_user_id, NEW.cree_le) THEN
            RAISE EXCEPTION '% : seul l''annulation est possible', TG_TABLE_NAME USING ERRCODE = '42501';
        END IF;
    ELSE
        a_avant := OLD.annulee_le;
        a_apres := NEW.annulee_le;
        IF (OLD.id, OLD.etablissement_id, OLD.eleve_id, OLD.nuit, OLD.motif_code, OLD.par_user_id, OLD.cree_le)
           IS DISTINCT FROM (NEW.id, NEW.etablissement_id, NEW.eleve_id, NEW.nuit, NEW.motif_code,
            NEW.par_user_id, NEW.cree_le) THEN
            RAISE EXCEPTION '% : seule l''annulation est possible', TG_TABLE_NAME USING ERRCODE = '42501';
        END IF;
    END IF;
    IF a_avant IS NOT NULL AND a_apres IS DISTINCT FROM a_avant THEN
        RAISE EXCEPTION '% : déjà annulé', TG_TABLE_NAME USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_internat_retrait_garde ON ent_internat_retrait;
CREATE TRIGGER ent_internat_retrait_garde BEFORE UPDATE OR DELETE ON ent_internat_retrait
    FOR EACH ROW EXECUTE FUNCTION ent_internat_annulable_garde();
DROP TRIGGER IF EXISTS ent_internat_nuit_exceptionnelle_garde ON ent_internat_nuit_exceptionnelle;
CREATE TRIGGER ent_internat_nuit_exceptionnelle_garde BEFORE UPDATE OR DELETE ON ent_internat_nuit_exceptionnelle
    FOR EACH ROW EXECUTE FUNCTION ent_internat_annulable_garde();

-- ───────────────────────── 3. L'attendu de la nuit (§9.1, corrige DI-11, DI-16, RI-06) ─────────────────────────
-- Fonction unique : séjours dont le rythme contient le jour de la nuit, plus les nuits
-- exceptionnelles ; moins les retraits (posés, situations connues du socle 12, mobilités du socle 2,
-- séjours scolaires du socle 42, fermeture de l'internat du socle 5). Les élèves hébergés qui ne
-- sont pas attendus ce soir sont listés « non_attendu » (pour la marque « non attendu présent »).
-- Un seul calcul, aucune table de jours codée en dur (R-IT40).
CREATE OR REPLACE FUNCTION ent_internat_attendu_calcule(etab UUID, nuit DATE)
RETURNS TABLE (eleve_id UUID, chambre_id UUID, lit_id UUID, statut TEXT, motif_code TEXT)
LANGUAGE sql STABLE AS $$
    WITH ancre AS (
        SELECT ent_internat_ancre_nuit(etab, nuit) AS t, ent_internat_fuseau(etab) AS tz
    ),
    du_rythme AS (
        SELECT s.eleve_id
          FROM ent_internat_sejour s
         WHERE s.etablissement_id = etab AND s.du <= nuit AND (s.au IS NULL OR s.au >= nuit)
           AND COALESCE((
                SELECT CASE extract(dow FROM nuit)::int
                         WHEN 0 THEN r.dimanche WHEN 1 THEN r.lundi WHEN 2 THEN r.mardi
                         WHEN 3 THEN r.mercredi WHEN 4 THEN r.jeudi WHEN 5 THEN r.vendredi
                         ELSE r.samedi END
                  FROM ent_internat_rythme r
                 WHERE r.etablissement_id = etab AND r.sejour_id = s.id AND r.du <= nuit
                 ORDER BY r.du DESC LIMIT 1), false)
    ),
    ajoutes AS (
        SELECT x.eleve_id FROM ent_internat_nuit_exceptionnelle x
         WHERE x.etablissement_id = etab AND x.nuit = ent_internat_attendu_calcule.nuit AND x.annulee_le IS NULL
    ),
    hebergees AS (
        SELECT DISTINCT ON (a.eleve_id) a.eleve_id, a.chambre_id, a.lit_id
          FROM ent_internat_affectation a
         WHERE a.etablissement_id = etab AND a.debut <= nuit AND (a.fin IS NULL OR a.fin >= nuit)
         ORDER BY a.eleve_id, a.debut DESC
    ),
    retraits AS (
        SELECT r.eleve_id, r.motif_code AS motif FROM ent_internat_retrait r
         WHERE r.etablissement_id = etab AND r.annule_le IS NULL AND r.du_nuit <= nuit AND r.au_nuit >= nuit
        UNION ALL
        SELECT sit.eleve_id,
               CASE sit.nature WHEN 'declaration' THEN 'absence_annoncee' WHEN 'sortie' THEN 'sortie'
                               WHEN 'mobilite' THEN 'mobilite' ELSE 'stage' END
          FROM ent_asd_situation sit, ancre
         WHERE sit.etablissement_id = etab AND sit.etat = 'active'
           AND sit.nature IN ('declaration', 'sortie', 'mobilite', 'pfmp')
           AND ((sit.debut_jour + COALESCE(sit.debut_heure, TIME '00:00')) AT TIME ZONE ancre.tz) <= ancre.t
           AND ancre.t < ((sit.fin_jour + COALESCE(sit.fin_heure, TIME '23:59:59')) AT TIME ZONE ancre.tz)
        UNION ALL
        SELECT m.eleve_id, 'mobilite' FROM ent_eleve_mobilite m
         WHERE m.etablissement_id = etab AND m.ferme_le IS NULL AND m.du <= nuit AND (m.au IS NULL OR m.au >= nuit)
        UNION ALL
        SELECT p.eleve_id, 'sejour_scolaire'
          FROM ent_sortie so
          JOIN ent_sortie_participant p ON p.etablissement_id = so.etablissement_id AND p.sortie_id = so.id
          CROSS JOIN ancre
         WHERE so.etablissement_id = etab AND so.etat IN ('prete', 'en_cours')
           AND p.etat IN ('inscrit', 'pret', 'parti')
           AND so.debut <= ancre.t AND ancre.t < so.fin
        UNION ALL
        SELECT d.eleve_id, 'fermeture'
          FROM (SELECT eleve_id FROM du_rythme UNION SELECT eleve_id FROM ajoutes) d
         WHERE EXISTS (
                SELECT 1 FROM ent_calendrier_evenement ev
                 WHERE ev.etablissement_id = etab AND ev.type = 'fermeture_exceptionnelle'
                   AND ev.annule_le IS NULL AND 'internat' = ANY (ev.services_touches)
                   AND NOT ev.internes_presents AND ev.du <= nuit AND ev.au >= nuit)
    ),
    retrait_retenu AS (
        SELECT DISTINCT ON (r.eleve_id) r.eleve_id, r.motif
          FROM retraits r
         ORDER BY r.eleve_id, CASE r.motif
               WHEN 'fermeture' THEN 1 WHEN 'exclusion_internat' THEN 2 WHEN 'sante' THEN 3
               WHEN 'sejour_scolaire' THEN 4 WHEN 'stage' THEN 5 WHEN 'mobilite' THEN 6
               WHEN 'sortie' THEN 7 ELSE 8 END
    ),
    concerne AS (
        SELECT eleve_id, 'attendu' AS origine FROM du_rythme
        UNION
        SELECT eleve_id, 'ajoute' FROM ajoutes
        UNION
        SELECT eleve_id, 'non_attendu' FROM hebergees
    ),
    unique_eleve AS (
        SELECT DISTINCT ON (c.eleve_id) c.eleve_id, c.origine
          FROM concerne c
         ORDER BY c.eleve_id, CASE c.origine WHEN 'attendu' THEN 1 WHEN 'ajoute' THEN 2 ELSE 3 END
    )
    SELECT u.eleve_id, h.chambre_id, h.lit_id,
           CASE WHEN u.origine = 'non_attendu' THEN
                    CASE WHEN rr.motif IS NOT NULL THEN 'retire' ELSE 'non_attendu' END
                WHEN rr.motif IS NOT NULL THEN 'retire'
                ELSE u.origine END,
           CASE WHEN u.origine = 'non_attendu' THEN NULL ELSE rr.motif END
      FROM unique_eleve u
      LEFT JOIN hebergees h ON h.eleve_id = u.eleve_id
      LEFT JOIN retrait_retenu rr ON rr.eleve_id = u.eleve_id
$$;

-- ───────────────────────── 4. Listes (groupes de chambres confiés) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_internat_liste (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 80),
    confie_a_user_id UUID REFERENCES ent_user (id),
    actif BOOLEAN NOT NULL DEFAULT true,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_liste_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_liste_libelle_uq UNIQUE (etablissement_id, libelle)
);
CREATE TABLE IF NOT EXISTS ent_internat_liste_chambre (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    liste_id UUID NOT NULL,
    chambre_id UUID NOT NULL,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (liste_id, chambre_id),
    CONSTRAINT ent_internat_liste_chambre_liste_cfk FOREIGN KEY (etablissement_id, liste_id)
        REFERENCES ent_internat_liste (etablissement_id, id),
    CONSTRAINT ent_internat_liste_chambre_chambre_cfk FOREIGN KEY (etablissement_id, chambre_id)
        REFERENCES ent_internat_chambre (etablissement_id, id)
);

-- ───────────────────────── 5. Appels (R-IT13, R-IT14, R-IT16, R-IT17, R-IT30, R-IT31) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_internat_appel (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nuit DATE NOT NULL,
    type TEXT NOT NULL CHECK (type IN (
        'arrivee', 'etude', 'soir', 'extinction', 'ronde', 'matin', 'rassemblement')),
    perimetre TEXT NOT NULL CHECK (perimetre IN ('internat', 'batiment', 'zone', 'liste')),
    batiment_id UUID,
    zone_id UUID,
    liste_id UUID,
    etat TEXT NOT NULL DEFAULT 'ouvert' CHECK (etat IN ('ouvert', 'clos', 'valide')),
    a_posteriori BOOLEAN NOT NULL DEFAULT false,
    ouvert_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    ouvert_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    clos_par_user_id UUID REFERENCES ent_user (id),
    clos_le TIMESTAMPTZ,
    valide_par_user_id UUID REFERENCES ent_user (id),
    valide_le TIMESTAMPTZ,
    nb_attendus INTEGER NOT NULL DEFAULT 0,
    nb_ecarts INTEGER,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_appel_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_appel_batiment_cfk FOREIGN KEY (etablissement_id, batiment_id)
        REFERENCES ent_batiment (etablissement_id, id),
    CONSTRAINT ent_internat_appel_zone_cfk FOREIGN KEY (etablissement_id, zone_id)
        REFERENCES ent_internat_zone (etablissement_id, id),
    CONSTRAINT ent_internat_appel_liste_cfk FOREIGN KEY (etablissement_id, liste_id)
        REFERENCES ent_internat_liste (etablissement_id, id),
    CONSTRAINT ent_internat_appel_perimetre_chk CHECK (
        (perimetre = 'internat' AND batiment_id IS NULL AND zone_id IS NULL AND liste_id IS NULL)
        OR (perimetre = 'batiment' AND batiment_id IS NOT NULL AND zone_id IS NULL AND liste_id IS NULL)
        OR (perimetre = 'zone' AND zone_id IS NOT NULL AND batiment_id IS NULL AND liste_id IS NULL)
        OR (perimetre = 'liste' AND liste_id IS NOT NULL AND batiment_id IS NULL AND zone_id IS NULL)),
    CONSTRAINT ent_internat_appel_etat_chk CHECK (
        (etat = 'ouvert' AND clos_le IS NULL AND valide_le IS NULL)
        OR (etat = 'clos' AND clos_le IS NOT NULL AND valide_le IS NULL)
        OR (etat = 'valide' AND clos_le IS NOT NULL AND valide_le IS NOT NULL
            AND valide_par_user_id IS NOT NULL AND clos_par_user_id IS NOT NULL))
);
-- Un seul appel par type, nuit et périmètre (R-IT31, CIT-40) : les surveillants se partagent des
-- listes, jamais un document.
CREATE UNIQUE INDEX IF NOT EXISTS ent_internat_appel_perimetre_uq ON ent_internat_appel (
    etablissement_id, nuit, type, perimetre,
    COALESCE(batiment_id, '00000000-0000-0000-0000-000000000000'::uuid),
    COALESCE(zone_id, '00000000-0000-0000-0000-000000000000'::uuid),
    COALESCE(liste_id, '00000000-0000-0000-0000-000000000000'::uuid));
CREATE INDEX IF NOT EXISTS ent_internat_appel_nuit_idx ON ent_internat_appel (etablissement_id, nuit, type);

CREATE OR REPLACE FUNCTION ent_internat_appel_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_internat_appel : suppression interdite (R-IT3)' USING ERRCODE = '42501';
    END IF;
    IF TG_OP = 'INSERT' THEN
        -- Défense en profondeur : une nuit future n'a pas d'appel (le service le dit avant, avec
        -- son horloge ; ici l'horloge de la base).
        IF NEW.nuit > ent_nuit_etab(NEW.etablissement_id, now()) THEN
            RAISE EXCEPTION 'appel d''une nuit future refusé' USING ERRCODE = '22007';
        END IF;
        RETURN NEW;
    END IF;
    IF OLD.etat = 'valide' THEN
        RAISE EXCEPTION 'appel validé : corriger par un fait motivé (R-IT17)' USING ERRCODE = '42501';
    END IF;
    IF (OLD.id, OLD.etablissement_id, OLD.nuit, OLD.type, OLD.perimetre, OLD.batiment_id, OLD.zone_id,
        OLD.liste_id, OLD.ouvert_par_user_id, OLD.ouvert_le, OLD.nb_attendus)
       IS DISTINCT FROM (NEW.id, NEW.etablissement_id, NEW.nuit, NEW.type, NEW.perimetre, NEW.batiment_id,
        NEW.zone_id, NEW.liste_id, NEW.ouvert_par_user_id, NEW.ouvert_le, NEW.nb_attendus) THEN
        RAISE EXCEPTION 'appel : nuit, type et périmètre ne se modifient pas' USING ERRCODE = '42501';
    END IF;
    IF OLD.etat = 'clos' AND NEW.etat = 'ouvert' THEN
        RAISE EXCEPTION 'appel clos : il ne se rouvre pas, on corrige par un fait motivé' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_internat_appel_garde ON ent_internat_appel;
CREATE TRIGGER ent_internat_appel_garde BEFORE INSERT OR UPDATE OR DELETE ON ent_internat_appel
    FOR EACH ROW EXECUTE FUNCTION ent_internat_appel_garde();

-- Liste figée à l'ouverture : ce qu'on croyait à l'heure de l'appel. Ajout seul.
CREATE TABLE IF NOT EXISTS ent_internat_appel_attendu (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    appel_id UUID NOT NULL,
    eleve_id UUID NOT NULL,
    chambre_id UUID,
    lit_id UUID,
    statut TEXT NOT NULL CHECK (statut IN ('attendu', 'ajoute', 'retire', 'non_attendu')),
    motif_code TEXT CHECK (motif_code IS NULL OR motif_code IN (
        'absence_annoncee', 'sortie', 'mobilite', 'stage', 'sejour_scolaire',
        'exclusion_internat', 'fermeture', 'sante')),
    calcule_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (appel_id, eleve_id),
    CONSTRAINT ent_internat_appel_attendu_appel_cfk FOREIGN KEY (etablissement_id, appel_id)
        REFERENCES ent_internat_appel (etablissement_id, id),
    CONSTRAINT ent_internat_appel_attendu_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_internat_appel_attendu_chambre_cfk FOREIGN KEY (etablissement_id, chambre_id)
        REFERENCES ent_internat_chambre (etablissement_id, id),
    CONSTRAINT ent_internat_appel_attendu_lit_cfk FOREIGN KEY (etablissement_id, lit_id)
        REFERENCES ent_internat_lit (etablissement_id, id),
    CONSTRAINT ent_internat_appel_attendu_motif_chk CHECK ((statut = 'retire') = (motif_code IS NOT NULL))
);
CREATE INDEX IF NOT EXISTS ent_internat_appel_attendu_eleve_idx
    ON ent_internat_appel_attendu (etablissement_id, eleve_id);

CREATE OR REPLACE FUNCTION ent_internat_i2_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION '% est en ajout seul (% interdit)', TG_TABLE_NAME, TG_OP USING ERRCODE = '42501';
END
$$;
DROP TRIGGER IF EXISTS ent_internat_appel_attendu_ajout_seul ON ent_internat_appel_attendu;
CREATE TRIGGER ent_internat_appel_attendu_ajout_seul BEFORE UPDATE OR DELETE ON ent_internat_appel_attendu
    FOR EACH ROW EXECUTE FUNCTION ent_internat_i2_ajout_seul();

-- Les faits de présence (2820) se rattachent à l'appel : clé composite posée ici, puis garde réécrite.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_presence_fait_appel_cfk') THEN
        ALTER TABLE ent_internat_presence_fait
            ADD CONSTRAINT ent_internat_presence_fait_appel_cfk
            FOREIGN KEY (etablissement_id, appel_id) REFERENCES ent_internat_appel (etablissement_id, id);
    END IF;
END
$$;
CREATE INDEX IF NOT EXISTS ent_internat_presence_fait_appel_idx
    ON ent_internat_presence_fait (etablissement_id, appel_id, eleve_id, recu_le DESC, id DESC)
    WHERE appel_id IS NOT NULL;

-- Garde (remplace celle de 2820, qui reste telle quelle dans son fichier) : un fait RATTACHÉ à un
-- appel (a) porte la nuit de l'appel, (b) concerne un élève de la liste figée de l'appel (R-IT14,
-- DI-19), (c) sur un appel clos ou validé exige un motif de correction (R-IT17). La date n'est
-- plus comparée à l'horloge de la base pour ces faits : l'appel porte la nuit, et son ouverture
-- (service, horloge du service) a déjà borné la date. Un fait sans appel suit l'ancienne règle.
CREATE OR REPLACE FUNCTION ent_internat_presence_fait_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    courante DATE := ent_nuit_etab(NEW.etablissement_id, now());
    a RECORD;
BEGIN
    IF NEW.motif_correction_code = 'reprise' THEN
        RETURN NEW;
    END IF;
    IF NEW.appel_id IS NOT NULL THEN
        SELECT nuit, etat INTO a FROM ent_internat_appel
         WHERE etablissement_id = NEW.etablissement_id AND id = NEW.appel_id;
        IF NOT FOUND OR a.nuit <> NEW.nuit THEN
            RAISE EXCEPTION 'fait hors de la nuit de l''appel' USING ERRCODE = '23514';
        END IF;
        IF NOT EXISTS (SELECT 1 FROM ent_internat_appel_attendu x
                        WHERE x.etablissement_id = NEW.etablissement_id AND x.appel_id = NEW.appel_id
                          AND x.eleve_id = NEW.eleve_id) THEN
            RAISE EXCEPTION 'élève hors du périmètre de l''appel (R-IT14)' USING ERRCODE = '23514';
        END IF;
        IF a.etat <> 'ouvert' AND NEW.motif_correction_code IS NULL THEN
            RAISE EXCEPTION 'appel clos ou validé : correction motivée obligatoire' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.nuit > courante THEN
        RAISE EXCEPTION 'pointage d''une nuit future refusé' USING ERRCODE = '22007';
    END IF;
    IF NEW.nuit < courante AND NEW.motif_correction_code IS NULL THEN
        RAISE EXCEPTION 'pointage d''une nuit passée : motif de correction obligatoire' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;

-- ───────────────────────── 6. Procédures et cas (§9.3, R-IT18, R-IT26) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_internat_procedure_version (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nature TEXT NOT NULL CHECK (nature IN ('non_retrouve')),
    version INTEGER NOT NULL CHECK (version >= 1),
    publiee_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    publiee_par_user_id UUID REFERENCES ent_user (id),
    CONSTRAINT ent_internat_procedure_version_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_procedure_version_uq UNIQUE (etablissement_id, nature, version)
);
CREATE TABLE IF NOT EXISTS ent_internat_procedure_etape (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    procedure_version_id UUID NOT NULL,
    rang SMALLINT NOT NULL CHECK (rang >= 1),
    code TEXT NOT NULL CHECK (code ~ '^[a-z_]{2,40}$'),
    libelle TEXT NOT NULL CHECK (char_length(btrim(libelle)) BETWEEN 1 AND 200),
    delai_minutes SMALLINT NOT NULL CHECK (delai_minutes BETWEEN 0 AND 720),
    appelle TEXT NOT NULL CHECK (appelle IN ('astreinte', 'titulaires', 'direction', 'secours', 'aucun')),
    CONSTRAINT ent_internat_procedure_etape_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_procedure_etape_rang_uq UNIQUE (etablissement_id, procedure_version_id, rang),
    CONSTRAINT ent_internat_procedure_etape_version_cfk FOREIGN KEY (etablissement_id, procedure_version_id)
        REFERENCES ent_internat_procedure_version (etablissement_id, id)
);
DROP TRIGGER IF EXISTS ent_internat_procedure_version_ajout_seul ON ent_internat_procedure_version;
CREATE TRIGGER ent_internat_procedure_version_ajout_seul BEFORE UPDATE OR DELETE ON ent_internat_procedure_version
    FOR EACH ROW EXECUTE FUNCTION ent_internat_i2_ajout_seul();
DROP TRIGGER IF EXISTS ent_internat_procedure_etape_ajout_seul ON ent_internat_procedure_etape;
CREATE TRIGGER ent_internat_procedure_etape_ajout_seul BEFORE UPDATE OR DELETE ON ent_internat_procedure_etape
    FOR EACH ROW EXECUTE FUNCTION ent_internat_i2_ajout_seul();

CREATE TABLE IF NOT EXISTS ent_internat_cas (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    appel_id UUID,
    nuit DATE NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN ('non_retrouve', 'non_rentre', 'non_recupere', 'retard_retour')),
    ouvert_le TIMESTAMPTZ NOT NULL,
    ouvert_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    procedure_version_id UUID NOT NULL,
    issue TEXT CHECK (issue IS NULL OR issue IN ('retrouve', 'rentre_famille', 'autorite_saisie', 'annule_erreur')),
    issue_le TIMESTAMPTZ,
    issue_par_user_id UUID REFERENCES ent_user (id),
    -- Où il a été retrouvé (code de lieu) ; qui l'a confirmé (personne du socle 4) ; numéro d'intervention.
    issue_lieu_code TEXT CHECK (issue_lieu_code IS NULL OR issue_lieu_code IN (
        'chambre', 'sanitaires', 'etude', 'infirmerie', 'autre_zone', 'exterieur')),
    issue_confirmee_par_personne_id UUID,
    numero_intervention TEXT CHECK (numero_intervention IS NULL OR char_length(numero_intervention) <= 40),
    transmis_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_cas_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_cas_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_internat_cas_appel_cfk FOREIGN KEY (etablissement_id, appel_id)
        REFERENCES ent_internat_appel (etablissement_id, id),
    CONSTRAINT ent_internat_cas_procedure_cfk FOREIGN KEY (etablissement_id, procedure_version_id)
        REFERENCES ent_internat_procedure_version (etablissement_id, id),
    CONSTRAINT ent_internat_cas_personne_cfk FOREIGN KEY (etablissement_id, issue_confirmee_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id),
    CONSTRAINT ent_internat_cas_issue_chk CHECK (
        (issue IS NULL) = (issue_le IS NULL) AND (issue IS NULL) = (issue_par_user_id IS NULL))
);
-- Un seul cas ouvert par élève et par nuit ; un cas clos peut être suivi d'un autre (CIT-36).
CREATE UNIQUE INDEX IF NOT EXISTS ent_internat_cas_ouvert_uq
    ON ent_internat_cas (etablissement_id, eleve_id, nuit) WHERE issue IS NULL;
CREATE INDEX IF NOT EXISTS ent_internat_cas_nuit_idx ON ent_internat_cas (etablissement_id, nuit);

CREATE OR REPLACE FUNCTION ent_internat_cas_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_internat_cas : suppression interdite (R-IT3)' USING ERRCODE = '42501';
    END IF;
    IF OLD.issue IS NOT NULL AND (OLD.issue, OLD.issue_le, OLD.issue_par_user_id, OLD.eleve_id, OLD.nuit,
                                  OLD.ouvert_le, OLD.nature)
       IS DISTINCT FROM (NEW.issue, NEW.issue_le, NEW.issue_par_user_id, NEW.eleve_id, NEW.nuit,
                         NEW.ouvert_le, NEW.nature) THEN
        RAISE EXCEPTION 'un cas clos ne se modifie pas (R-IT18) : seule sa transmission s''ajoute'
            USING ERRCODE = '42501';
    END IF;
    IF (OLD.eleve_id, OLD.nuit, OLD.ouvert_le, OLD.procedure_version_id)
       IS DISTINCT FROM (NEW.eleve_id, NEW.nuit, NEW.ouvert_le, NEW.procedure_version_id) THEN
        RAISE EXCEPTION 'un cas ne change ni d''élève, ni de nuit, ni de procédure' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_internat_cas_garde ON ent_internat_cas;
CREATE TRIGGER ent_internat_cas_garde BEFORE UPDATE OR DELETE ON ent_internat_cas
    FOR EACH ROW EXECUTE FUNCTION ent_internat_cas_garde();

-- Chaque étape ou vérification est un fait horodaté, en ajout seul (« fait à 22 h 41 par Y. M. »).
CREATE TABLE IF NOT EXISTS ent_internat_cas_etape (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    cas_id UUID NOT NULL,
    etape_id UUID,
    lieu_code TEXT CHECK (lieu_code IS NULL OR lieu_code IN (
        'chambre', 'sanitaires', 'etude', 'infirmerie', 'autre_zone', 'exterieur')),
    fait_le TIMESTAMPTZ NOT NULL,
    par_user_id UUID NOT NULL REFERENCES ent_user (id),
    note TEXT CHECK (note IS NULL OR char_length(note) <= 300),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_cas_etape_cas_cfk FOREIGN KEY (etablissement_id, cas_id)
        REFERENCES ent_internat_cas (etablissement_id, id),
    CONSTRAINT ent_internat_cas_etape_etape_cfk FOREIGN KEY (etablissement_id, etape_id)
        REFERENCES ent_internat_procedure_etape (etablissement_id, id),
    CONSTRAINT ent_internat_cas_etape_cible_chk CHECK (etape_id IS NOT NULL OR lieu_code IS NOT NULL)
);
CREATE INDEX IF NOT EXISTS ent_internat_cas_etape_cas_idx ON ent_internat_cas_etape (etablissement_id, cas_id, fait_le);
DROP TRIGGER IF EXISTS ent_internat_cas_etape_ajout_seul ON ent_internat_cas_etape;
CREATE TRIGGER ent_internat_cas_etape_ajout_seul BEFORE UPDATE OR DELETE ON ent_internat_cas_etape
    FOR EACH ROW EXECUTE FUNCTION ent_internat_i2_ajout_seul();

-- ───────────────────────── 7. Transmission du matin et rappels ─────────────────────────
-- Transmission : une par nuit et par nature (idempotente) ; ne porte que des compteurs.
CREATE TABLE IF NOT EXISTS ent_internat_transmission (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nuit DATE NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN ('matin_vie_scolaire', 'effectif_restauration')),
    transmis_le TIMESTAMPTZ NOT NULL,
    transmis_par_user_id UUID REFERENCES ent_user (id),
    nb_attendus INTEGER NOT NULL CHECK (nb_attendus >= 0),
    nb_presents INTEGER NOT NULL CHECK (nb_presents >= 0),
    nb_absents INTEGER NOT NULL CHECK (nb_absents >= 0),
    nb_cas INTEGER NOT NULL CHECK (nb_cas >= 0),
    CONSTRAINT ent_internat_transmission_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_transmission_uq UNIQUE (etablissement_id, nuit, nature)
);
DROP TRIGGER IF EXISTS ent_internat_transmission_ajout_seul ON ent_internat_transmission;
CREATE TRIGGER ent_internat_transmission_ajout_seul BEFORE UPDATE OR DELETE ON ent_internat_transmission
    FOR EACH ROW EXECUTE FUNCTION ent_internat_i2_ajout_seul();

-- Rappel d'appel non validé : une seule fois par niveau, nuit et type (NI-22).
CREATE TABLE IF NOT EXISTS ent_internat_appel_rappel (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nuit DATE NOT NULL,
    type TEXT NOT NULL CHECK (type IN (
        'arrivee', 'etude', 'soir', 'extinction', 'ronde', 'matin', 'rassemblement')),
    niveau TEXT NOT NULL CHECK (niveau IN ('surveillant', 'cpe')),
    emis_le TIMESTAMPTZ NOT NULL,
    CONSTRAINT ent_internat_appel_rappel_uq UNIQUE (etablissement_id, nuit, type, niveau)
);
DROP TRIGGER IF EXISTS ent_internat_appel_rappel_ajout_seul ON ent_internat_appel_rappel;
CREATE TRIGGER ent_internat_appel_rappel_ajout_seul BEFORE UPDATE OR DELETE ON ent_internat_appel_rappel
    FOR EACH ROW EXECUTE FUNCTION ent_internat_i2_ajout_seul();

-- ───────────────────────── 8. Journal des lectures sensibles (R-IT38, E7) ─────────────────────────
CREATE TABLE IF NOT EXISTS ent_internat_lecture_sensible (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    user_id UUID NOT NULL REFERENCES ent_user (id),
    objet TEXT NOT NULL CHECK (objet IN (
        'fiche_nuit', 'liaison', 'presents', 'registre', 'cas', 'appel', 'appel_pdf', 'attendu')),
    eleve_id UUID,
    appel_id UUID,
    a TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_lecture_sensible_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_internat_lecture_sensible_idx
    ON ent_internat_lecture_sensible (etablissement_id, a DESC);
DROP TRIGGER IF EXISTS ent_internat_lecture_sensible_ajout_seul ON ent_internat_lecture_sensible;
CREATE TRIGGER ent_internat_lecture_sensible_ajout_seul BEFORE UPDATE OR DELETE ON ent_internat_lecture_sensible
    FOR EACH ROW EXECUTE FUNCTION ent_internat_i2_ajout_seul();

SELECT ent_rls_poser(ARRAY[
    'ent_internat_nuit_exceptionnelle', 'ent_internat_retrait', 'ent_internat_liste',
    'ent_internat_liste_chambre', 'ent_internat_appel', 'ent_internat_appel_attendu',
    'ent_internat_procedure_version', 'ent_internat_procedure_etape', 'ent_internat_cas',
    'ent_internat_cas_etape', 'ent_internat_transmission', 'ent_internat_appel_rappel',
    'ent_internat_lecture_sensible']);
