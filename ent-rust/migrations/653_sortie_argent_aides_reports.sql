-- 653 — Socle 42, lot V3 « argent relié » (1/2) : aides appliquées à la part d'un payeur et reports de versement
-- d'un participant à un autre. Réf. : 42_SORTIES_VOYAGES.md §12.4, §12.5, VOY-224, VOY-286, D-068 (la comptabilité
-- des voyages est épaissie et reliée au reste de l'ENT, pas refondue) ; écart n° 9 du rapport V2a.
--
-- Mêmes exigences que 650 : `etablissement_id` NOT NULL et clés étrangères COMPOSITES, UUID applicatifs, aucun
-- BYTEA ni JSON, montants en CENTIMES entiers bornés, AJOUT SEUL (une erreur s'annule par une écriture inverse
-- motivée), migration rejouable, aucune migration versée modifiée, RLS en fin de fichier. Aucune écriture dans les
-- tables `ent_fac_*` ni `ent_fin_*` : ScolIA reste en mode suivi (R-SV44, D-034).

-- Décision d'argent (D-069, règle 3) : toute aide et tout report passent par une DEMANDE motivée, qui s'applique tout de
-- suite quand la somme reste sous le seuil de l'établissement, ou attend la validation d'UNE SECONDE personne (au-delà
-- du seuil, ou toute aide du fonds social). Le décideur, comme le valideur, n'est jamais lié à l'élève ni payeur : un
-- pouvoir sur l'argent ne s'exerce pas pour son propre enfant. Ajout seul : la demande, puis son issue (une seule).
-- `cle` : clé d'idempotence fournie par le client (double clic = une seule écriture).
CREATE TABLE IF NOT EXISTS ent_sortie_argent_demande (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN ('aide', 'report')),
    -- Aide : le participant aidé. Report : celui qui cède.
    participant_id UUID NOT NULL,
    vers_participant_id UUID,
    compte_id UUID NOT NULL,
    source TEXT CHECK (source IS NULL OR source IN ('apel', 'fonds_social', 'collectivite', 'entreprise', 'don_affecte', 'autre')),
    montant_centimes BIGINT NOT NULL CHECK (montant_centimes BETWEEN 1 AND 10000000),
    reference TEXT CHECK (reference IS NULL OR length(btrim(reference)) BETWEEN 1 AND 40),
    motif TEXT NOT NULL CHECK (length(btrim(motif)) BETWEEN 1 AND 500),
    seconde_validation BOOLEAN NOT NULL,
    cle TEXT NOT NULL CHECK (length(cle) BETWEEN 8 AND 64),
    demande_par_user_id UUID NOT NULL,
    demande_le TIMESTAMPTZ NOT NULL,
    CONSTRAINT ent_sortie_argent_demande_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_argent_demande_cle_uq UNIQUE (etablissement_id, sortie_id, cle),
    CONSTRAINT ent_sortie_argent_demande_nature_chk CHECK (
        (nature = 'aide' AND source IS NOT NULL AND vers_participant_id IS NULL)
        OR (nature = 'report' AND source IS NULL AND vers_participant_id IS NOT NULL
            AND vers_participant_id <> participant_id)),
    CONSTRAINT ent_sortie_argent_demande_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_argent_demande_participant_fk FOREIGN KEY (etablissement_id, participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id),
    CONSTRAINT ent_sortie_argent_demande_vers_fk FOREIGN KEY (etablissement_id, vers_participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id),
    CONSTRAINT ent_sortie_argent_demande_compte_fk FOREIGN KEY (etablissement_id, compte_id)
        REFERENCES ent_fin_compte (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_argent_demande_sortie_idx
    ON ent_sortie_argent_demande (etablissement_id, sortie_id, demande_le);

-- Aide accordée sur la part d'UN payeur d'un participant (APEL, fonds social, collectivité, entreprise, don
-- affecté…). Elle réduit ce que la famille doit ; elle n'est JAMAIS un versement de la famille : un avoir ne la
-- rembourse donc pas au payeur (CF-69). Seule la comptabilité l'inscrit ; l'organisateur n'en voit jamais le détail.
CREATE TABLE IF NOT EXISTS ent_sortie_aide (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    participant_id UUID NOT NULL,
    compte_id UUID NOT NULL,
    sens TEXT NOT NULL CHECK (sens IN ('aide', 'annulation')),
    source TEXT NOT NULL CHECK (source IN ('apel', 'fonds_social', 'collectivite', 'entreprise', 'don_affecte', 'autre')),
    montant_centimes BIGINT NOT NULL CHECK (montant_centimes BETWEEN 1 AND 10000000),
    -- Référence de la décision ou du courrier d'attribution (jamais un justificatif de revenus).
    reference TEXT CHECK (reference IS NULL OR length(btrim(reference)) BETWEEN 1 AND 40),
    annule_id UUID,
    -- Motif OBLIGATOIRE à l'attribution comme à l'annulation (D-069) : jamais de détail de situation sociale.
    motif TEXT NOT NULL CHECK (length(btrim(motif)) BETWEEN 1 AND 500),
    -- La demande qui a produit l'aide (une annulation n'en a pas).
    demande_id UUID,
    decide_par_user_id UUID NOT NULL,
    decide_le TIMESTAMPTZ NOT NULL,
    CONSTRAINT ent_sortie_aide_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_aide_sens_chk CHECK (
        (sens = 'aide' AND annule_id IS NULL AND demande_id IS NOT NULL)
        OR (sens = 'annulation' AND annule_id IS NOT NULL AND demande_id IS NULL)),
    CONSTRAINT ent_sortie_aide_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_aide_participant_fk FOREIGN KEY (etablissement_id, participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id),
    CONSTRAINT ent_sortie_aide_compte_fk FOREIGN KEY (etablissement_id, compte_id)
        REFERENCES ent_fin_compte (etablissement_id, id),
    CONSTRAINT ent_sortie_aide_annule_fk FOREIGN KEY (etablissement_id, annule_id)
        REFERENCES ent_sortie_aide (etablissement_id, id),
    CONSTRAINT ent_sortie_aide_demande_fk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_sortie_argent_demande (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_aide_part_idx
    ON ent_sortie_aide (etablissement_id, sortie_id, participant_id, compte_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_aide_annulation_uq
    ON ent_sortie_aide (annule_id) WHERE annule_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_aide_demande_uq
    ON ent_sortie_aide (demande_id) WHERE demande_id IS NOT NULL;

-- Report d'un versement d'un participant à un autre de la MÊME sortie (un frère ou une sœur qui prend la place,
-- un enfant remplacé depuis la liste d'attente) : l'argent reste celui du MÊME payeur (même compte des deux côtés),
-- jamais d'un payeur à un autre. Une annulation reprend exactement le report qu'elle défait.
CREATE TABLE IF NOT EXISTS ent_sortie_report (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    sortie_id UUID NOT NULL,
    de_participant_id UUID NOT NULL,
    vers_participant_id UUID NOT NULL,
    compte_id UUID NOT NULL,
    sens TEXT NOT NULL CHECK (sens IN ('report', 'annulation')),
    montant_centimes BIGINT NOT NULL CHECK (montant_centimes BETWEEN 1 AND 10000000),
    annule_id UUID,
    motif TEXT NOT NULL CHECK (length(btrim(motif)) BETWEEN 1 AND 500),
    demande_id UUID,
    decide_par_user_id UUID NOT NULL,
    decide_le TIMESTAMPTZ NOT NULL,
    CONSTRAINT ent_sortie_report_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_report_distincts_chk CHECK (de_participant_id <> vers_participant_id),
    CONSTRAINT ent_sortie_report_sens_chk CHECK (
        (sens = 'report' AND annule_id IS NULL AND demande_id IS NOT NULL)
        OR (sens = 'annulation' AND annule_id IS NOT NULL AND demande_id IS NULL)),
    CONSTRAINT ent_sortie_report_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_report_de_fk FOREIGN KEY (etablissement_id, de_participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id),
    CONSTRAINT ent_sortie_report_vers_fk FOREIGN KEY (etablissement_id, vers_participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id),
    CONSTRAINT ent_sortie_report_compte_fk FOREIGN KEY (etablissement_id, compte_id)
        REFERENCES ent_fin_compte (etablissement_id, id),
    CONSTRAINT ent_sortie_report_annule_fk FOREIGN KEY (etablissement_id, annule_id)
        REFERENCES ent_sortie_report (etablissement_id, id),
    CONSTRAINT ent_sortie_report_demande_fk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_sortie_argent_demande (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_report_de_idx
    ON ent_sortie_report (etablissement_id, sortie_id, de_participant_id, compte_id);
CREATE INDEX IF NOT EXISTS ent_sortie_report_vers_idx
    ON ent_sortie_report (etablissement_id, sortie_id, vers_participant_id, compte_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_report_annulation_uq
    ON ent_sortie_report (annule_id) WHERE annule_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_report_demande_uq
    ON ent_sortie_report (demande_id) WHERE demande_id IS NOT NULL;

-- Issue d'une demande (une seule, ajout seul) : appliquée (par le demandeur si le seuil n'est pas atteint, sinon par
-- la seconde personne), refusée ou retirée. `aide_id` / `report_id` : l'écriture produite.
CREATE TABLE IF NOT EXISTS ent_sortie_argent_issue (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    demande_id UUID NOT NULL,
    issue TEXT NOT NULL CHECK (issue IN ('appliquee', 'refusee', 'retiree')),
    motif TEXT CHECK (motif IS NULL OR length(btrim(motif)) BETWEEN 1 AND 500),
    aide_id UUID,
    report_id UUID,
    par_user_id UUID NOT NULL,
    le TIMESTAMPTZ NOT NULL,
    CONSTRAINT ent_sortie_argent_issue_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_argent_issue_demande_uq UNIQUE (etablissement_id, demande_id),
    CONSTRAINT ent_sortie_argent_issue_forme_chk CHECK (
        (issue = 'appliquee' AND ((aide_id IS NOT NULL) <> (report_id IS NOT NULL)))
        OR (issue IN ('refusee', 'retiree') AND aide_id IS NULL AND report_id IS NULL AND motif IS NOT NULL)),
    CONSTRAINT ent_sortie_argent_issue_demande_fk FOREIGN KEY (etablissement_id, demande_id)
        REFERENCES ent_sortie_argent_demande (etablissement_id, id),
    CONSTRAINT ent_sortie_argent_issue_aide_fk FOREIGN KEY (etablissement_id, aide_id)
        REFERENCES ent_sortie_aide (etablissement_id, id),
    CONSTRAINT ent_sortie_argent_issue_report_fk FOREIGN KEY (etablissement_id, report_id)
        REFERENCES ent_sortie_report (etablissement_id, id)
);

-- ───────────────────────── Garde-fous de la base ─────────────────────────
DROP TRIGGER IF EXISTS ent_sortie_aide_immuable ON ent_sortie_aide;
CREATE TRIGGER ent_sortie_aide_immuable BEFORE UPDATE OR DELETE ON ent_sortie_aide
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_aide_vidage ON ent_sortie_aide;
CREATE TRIGGER ent_sortie_aide_vidage BEFORE TRUNCATE ON ent_sortie_aide
    FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_report_immuable ON ent_sortie_report;
CREATE TRIGGER ent_sortie_report_immuable BEFORE UPDATE OR DELETE ON ent_sortie_report
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_report_vidage ON ent_sortie_report;
CREATE TRIGGER ent_sortie_report_vidage BEFORE TRUNCATE ON ent_sortie_report
    FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul();

DROP TRIGGER IF EXISTS ent_sortie_argent_demande_immuable ON ent_sortie_argent_demande;
CREATE TRIGGER ent_sortie_argent_demande_immuable BEFORE UPDATE OR DELETE ON ent_sortie_argent_demande
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_argent_demande_vidage ON ent_sortie_argent_demande;
CREATE TRIGGER ent_sortie_argent_demande_vidage BEFORE TRUNCATE ON ent_sortie_argent_demande
    FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_argent_issue_immuable ON ent_sortie_argent_issue;
CREATE TRIGGER ent_sortie_argent_issue_immuable BEFORE UPDATE OR DELETE ON ent_sortie_argent_issue
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_ajout_seul();
DROP TRIGGER IF EXISTS ent_sortie_argent_issue_vidage ON ent_sortie_argent_issue;
CREATE TRIGGER ent_sortie_argent_issue_vidage BEFORE TRUNCATE ON ent_sortie_argent_issue
    FOR EACH STATEMENT EXECUTE FUNCTION ent_sortie_ajout_seul();

-- Règle 3 (D-069) : un pouvoir sur l'argent ne s'exerce jamais pour son propre enfant. `true` quand `usr` est lié à l'un
-- des élèves (lien familial, ou l'élève lui-même s'il a un compte) ou payeur du compte (foyer payeur, personne du
-- compte, élève majeur désigné). Même fonction pour l'application (message clair) et pour les déclencheurs (garde).
CREATE OR REPLACE FUNCTION ent_sortie_argent_decideur_interdit(etab UUID, usr UUID, compte UUID, eleves UUID[])
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
    SELECT EXISTS (SELECT 1 FROM ent_eleve_lien l
                    WHERE l.etablissement_id = etab AND l.user_id = usr AND l.eleve_id = ANY(eleves))
        OR EXISTS (SELECT 1 FROM ent_eleve_compte ec
                    WHERE ec.etablissement_id = etab AND ec.user_id = usr AND ec.eleve_id = ANY(eleves))
        OR EXISTS (SELECT 1 FROM ent_fin_compte c
                    WHERE c.etablissement_id = etab AND c.id = compte
                      AND (EXISTS (SELECT 1 FROM ent_eleve_lien l
                                    WHERE l.etablissement_id = etab AND l.user_id = usr AND l.foyer_id = c.foyer_id)
                        OR EXISTS (SELECT 1 FROM ent_personne p
                                    WHERE p.etablissement_id = etab AND p.id = c.personne_id AND p.user_id = usr)
                        OR EXISTS (SELECT 1 FROM ent_personne p
                                    JOIN ent_eleve_compte ec ON ec.etablissement_id = etab AND ec.eleve_id = p.eleve_id
                                   WHERE p.etablissement_id = etab AND p.id = c.personne_id AND ec.user_id = usr)))
$$;

-- Garde de la base : ni une demande, ni une aide, ni un report, ni leur annulation, ni une issue « appliquée » ne
-- peuvent être l'œuvre d'une personne liée à l'élève ou payeuse (le contrôle de l'application donne le message ; ce
-- déclencheur interdit le contournement). La seconde personne n'est jamais le demandeur.
CREATE OR REPLACE FUNCTION ent_sortie_argent_garde_decideur() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    dem ent_sortie_argent_demande%ROWTYPE;
    qui UUID;
    eleves UUID[];
    cpt UUID;
BEGIN
    IF TG_TABLE_NAME = 'ent_sortie_argent_demande' THEN
        qui := NEW.demande_par_user_id; cpt := NEW.compte_id;
        SELECT array_agg(p.eleve_id) INTO eleves FROM ent_sortie_participant p
         WHERE p.etablissement_id = NEW.etablissement_id AND p.id IN (NEW.participant_id, COALESCE(NEW.vers_participant_id, NEW.participant_id));
    ELSIF TG_TABLE_NAME = 'ent_sortie_aide' THEN
        qui := NEW.decide_par_user_id; cpt := NEW.compte_id;
        SELECT array_agg(p.eleve_id) INTO eleves FROM ent_sortie_participant p
         WHERE p.etablissement_id = NEW.etablissement_id AND p.id = NEW.participant_id;
    ELSIF TG_TABLE_NAME = 'ent_sortie_report' THEN
        qui := NEW.decide_par_user_id; cpt := NEW.compte_id;
        SELECT array_agg(p.eleve_id) INTO eleves FROM ent_sortie_participant p
         WHERE p.etablissement_id = NEW.etablissement_id AND p.id IN (NEW.de_participant_id, NEW.vers_participant_id);
    ELSE
        -- Issue : seule une issue « appliquée » exerce le pouvoir ; la seconde personne n'est pas le demandeur.
        SELECT * INTO dem FROM ent_sortie_argent_demande WHERE etablissement_id = NEW.etablissement_id AND id = NEW.demande_id;
        IF NEW.issue <> 'appliquee' THEN
            RETURN NEW;
        END IF;
        IF dem.seconde_validation AND NEW.par_user_id = dem.demande_par_user_id THEN
            RAISE EXCEPTION 'une seconde personne valide : le demandeur ne se valide pas lui-même' USING ERRCODE = '23514';
        END IF;
        qui := NEW.par_user_id; cpt := dem.compte_id;
        SELECT array_agg(p.eleve_id) INTO eleves FROM ent_sortie_participant p
         WHERE p.etablissement_id = NEW.etablissement_id AND p.id IN (dem.participant_id, COALESCE(dem.vers_participant_id, dem.participant_id));
    END IF;
    IF ent_sortie_argent_decideur_interdit(NEW.etablissement_id, qui, cpt, COALESCE(eleves, ARRAY[]::uuid[])) THEN
        RAISE EXCEPTION 'un pouvoir sur l''argent ne s''exerce pas pour son propre enfant ni pour son propre compte'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_sortie_argent_demande_garde_tg ON ent_sortie_argent_demande;
CREATE TRIGGER ent_sortie_argent_demande_garde_tg BEFORE INSERT ON ent_sortie_argent_demande
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_argent_garde_decideur();
DROP TRIGGER IF EXISTS ent_sortie_aide_garde_tg ON ent_sortie_aide;
CREATE TRIGGER ent_sortie_aide_garde_tg BEFORE INSERT ON ent_sortie_aide
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_argent_garde_decideur();
DROP TRIGGER IF EXISTS ent_sortie_report_garde_tg ON ent_sortie_report;
CREATE TRIGGER ent_sortie_report_garde_tg BEFORE INSERT ON ent_sortie_report
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_argent_garde_decideur();
DROP TRIGGER IF EXISTS ent_sortie_argent_issue_garde_tg ON ent_sortie_argent_issue;
CREATE TRIGGER ent_sortie_argent_issue_garde_tg BEFORE INSERT ON ent_sortie_argent_issue
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_argent_garde_decideur();

-- Aide : le participant est de la sortie, le payeur porte une part de ce participant ; une annulation reprend
-- EXACTEMENT l'aide annulée (même sortie, participant, payeur, source, montant) et ne vise qu'une aide.
CREATE OR REPLACE FUNCTION ent_sortie_aide_controle() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    x ent_sortie_aide%ROWTYPE;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM ent_sortie_participant p
                    WHERE p.etablissement_id = NEW.etablissement_id AND p.id = NEW.participant_id
                      AND p.sortie_id = NEW.sortie_id)
       OR NOT EXISTS (SELECT 1 FROM ent_sortie_part q
                       WHERE q.etablissement_id = NEW.etablissement_id AND q.participant_id = NEW.participant_id
                         AND q.compte_id = NEW.compte_id AND q.sortie_id = NEW.sortie_id) THEN
        RAISE EXCEPTION 'l''aide vise une part de cette sortie' USING ERRCODE = '23514';
    END IF;
    IF NEW.sens = 'annulation' THEN
        SELECT * INTO x FROM ent_sortie_aide WHERE etablissement_id = NEW.etablissement_id AND id = NEW.annule_id;
        IF NOT FOUND OR x.sens <> 'aide' OR x.sortie_id <> NEW.sortie_id OR x.participant_id <> NEW.participant_id
           OR x.compte_id <> NEW.compte_id OR x.source <> NEW.source OR x.montant_centimes <> NEW.montant_centimes THEN
            RAISE EXCEPTION 'une annulation reprend exactement l''aide annulée' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_sortie_aide_controle_tg ON ent_sortie_aide;
CREATE TRIGGER ent_sortie_aide_controle_tg BEFORE INSERT ON ent_sortie_aide
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_aide_controle();

-- Report : les deux participants sont de la sortie et le payeur porte une part de CHACUN (l'argent ne change jamais
-- de payeur) ; une annulation reprend exactement le report défait, dans le même sens.
CREATE OR REPLACE FUNCTION ent_sortie_report_controle() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    x ent_sortie_report%ROWTYPE;
BEGIN
    IF (SELECT count(*) FROM ent_sortie_participant p
         WHERE p.etablissement_id = NEW.etablissement_id AND p.sortie_id = NEW.sortie_id
           AND p.id IN (NEW.de_participant_id, NEW.vers_participant_id)) <> 2
       OR (SELECT count(DISTINCT q.participant_id) FROM ent_sortie_part q
            WHERE q.etablissement_id = NEW.etablissement_id AND q.sortie_id = NEW.sortie_id
              AND q.compte_id = NEW.compte_id
              AND q.participant_id IN (NEW.de_participant_id, NEW.vers_participant_id)) <> 2 THEN
        RAISE EXCEPTION 'un report reste chez le même payeur, entre deux participants de la sortie' USING ERRCODE = '23514';
    END IF;
    IF NEW.sens = 'annulation' THEN
        SELECT * INTO x FROM ent_sortie_report WHERE etablissement_id = NEW.etablissement_id AND id = NEW.annule_id;
        IF NOT FOUND OR x.sens <> 'report' OR x.sortie_id <> NEW.sortie_id
           OR x.de_participant_id <> NEW.de_participant_id OR x.vers_participant_id <> NEW.vers_participant_id
           OR x.compte_id <> NEW.compte_id OR x.montant_centimes <> NEW.montant_centimes THEN
            RAISE EXCEPTION 'une annulation reprend exactement le report défait' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_sortie_report_controle_tg ON ent_sortie_report;
CREATE TRIGGER ent_sortie_report_controle_tg BEFORE INSERT ON ent_sortie_report
    FOR EACH ROW EXECUTE FUNCTION ent_sortie_report_controle();

SELECT ent_rls_poser(ARRAY['ent_sortie_argent_demande', 'ent_sortie_aide', 'ent_sortie_report', 'ent_sortie_argent_issue']);
