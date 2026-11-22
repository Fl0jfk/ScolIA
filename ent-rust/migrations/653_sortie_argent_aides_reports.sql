-- 653 — Socle 42, lot V3 « argent relié » (1/2) : aides appliquées à la part d'un payeur et reports de versement
-- d'un participant à un autre. Réf. : 42_SORTIES_VOYAGES.md §12.4, §12.5, VOY-224, VOY-286, D-068 (la comptabilité
-- des voyages est épaissie et reliée au reste de l'ENT, pas refondue) ; écart n° 9 du rapport V2a.
--
-- Mêmes exigences que 650 : `etablissement_id` NOT NULL et clés étrangères COMPOSITES, UUID applicatifs, aucun
-- BYTEA ni JSON, montants en CENTIMES entiers bornés, AJOUT SEUL (une erreur s'annule par une écriture inverse
-- motivée), migration rejouable, aucune migration versée modifiée, RLS en fin de fichier. Aucune écriture dans les
-- tables `ent_fac_*` ni `ent_fin_*` : ScolIA reste en mode suivi (R-SV44, D-034).

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
    motif TEXT CHECK (motif IS NULL OR length(btrim(motif)) BETWEEN 1 AND 500),
    decide_par_user_id UUID NOT NULL,
    decide_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_aide_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_aide_sens_chk CHECK (
        (sens = 'aide' AND annule_id IS NULL)
        OR (sens = 'annulation' AND annule_id IS NOT NULL AND motif IS NOT NULL)),
    CONSTRAINT ent_sortie_aide_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_aide_participant_fk FOREIGN KEY (etablissement_id, participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id),
    CONSTRAINT ent_sortie_aide_compte_fk FOREIGN KEY (etablissement_id, compte_id)
        REFERENCES ent_fin_compte (etablissement_id, id),
    CONSTRAINT ent_sortie_aide_annule_fk FOREIGN KEY (etablissement_id, annule_id)
        REFERENCES ent_sortie_aide (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_aide_part_idx
    ON ent_sortie_aide (etablissement_id, sortie_id, participant_id, compte_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_aide_annulation_uq
    ON ent_sortie_aide (annule_id) WHERE annule_id IS NOT NULL;

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
    decide_par_user_id UUID NOT NULL,
    decide_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_sortie_report_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_sortie_report_distincts_chk CHECK (de_participant_id <> vers_participant_id),
    CONSTRAINT ent_sortie_report_sens_chk CHECK (
        (sens = 'report' AND annule_id IS NULL) OR (sens = 'annulation' AND annule_id IS NOT NULL)),
    CONSTRAINT ent_sortie_report_sortie_fk FOREIGN KEY (etablissement_id, sortie_id)
        REFERENCES ent_sortie (etablissement_id, id),
    CONSTRAINT ent_sortie_report_de_fk FOREIGN KEY (etablissement_id, de_participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id),
    CONSTRAINT ent_sortie_report_vers_fk FOREIGN KEY (etablissement_id, vers_participant_id)
        REFERENCES ent_sortie_participant (etablissement_id, id),
    CONSTRAINT ent_sortie_report_compte_fk FOREIGN KEY (etablissement_id, compte_id)
        REFERENCES ent_fin_compte (etablissement_id, id),
    CONSTRAINT ent_sortie_report_annule_fk FOREIGN KEY (etablissement_id, annule_id)
        REFERENCES ent_sortie_report (etablissement_id, id)
);
CREATE INDEX IF NOT EXISTS ent_sortie_report_de_idx
    ON ent_sortie_report (etablissement_id, sortie_id, de_participant_id, compte_id);
CREATE INDEX IF NOT EXISTS ent_sortie_report_vers_idx
    ON ent_sortie_report (etablissement_id, sortie_id, vers_participant_id, compte_id);
CREATE UNIQUE INDEX IF NOT EXISTS ent_sortie_report_annulation_uq
    ON ent_sortie_report (annule_id) WHERE annule_id IS NOT NULL;

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

SELECT ent_rls_poser(ARRAY['ent_sortie_aide', 'ent_sortie_report']);
