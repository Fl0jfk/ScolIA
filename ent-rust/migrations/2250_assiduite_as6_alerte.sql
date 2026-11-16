-- 2250 — Socle 12, lot AS6 (vérification et file du matin). Réf. : 12_ASSIDUITE.md §9.5, §9.7, §17.6, R-AS37.
-- Deux tables : les ALERTES de la vie scolaire (tâches persistées : absence après présence, élève exclu non
-- arrivé) et le DRAPEAU de vigilance (« élève à surveiller », sans motif). Les appels non faits et les
-- incohérences d'appel restent CALCULÉS à la demande (ils disparaissent d'eux-mêmes quand l'appel est fait ou
-- corrigé) : on ne persiste que ce qu'une personne doit prendre en charge et dont on garde la trace.
-- Additive, rejouable (E8), sans colonne JSON (D-036), sans BYTEA ; RLS par `ent_rls_poser` (580), clés
-- étrangères composites (établissement, id). Aucun texte libre : codes et identifiants seulement (R-AS25).

CREATE TABLE IF NOT EXISTS ent_asd_alerte (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('absence_apres_presence', 'non_arrivee')),
    -- Jour métier de l'établissement (jamais la date UTC du serveur).
    jour DATE NOT NULL,
    -- Séance concernée (créneau) et appel qui a fait naître l'alerte.
    seance_ref UUID,
    appel_id UUID,
    -- Clé d'idempotence : une alerte par fait (rejeu d'un événement = aucune alerte de plus).
    cle TEXT NOT NULL CHECK (length(cle) BETWEEN 1 AND 160),
    statut TEXT NOT NULL DEFAULT 'ouverte' CHECK (statut IN ('ouverte', 'prise', 'traitee')),
    -- « Je prends » : verrou visible de 15 minutes, levé par un contact tracé ou par la clôture (AS-210).
    pris_par UUID,
    pris_le TIMESTAMPTZ,
    pris_jusqua TIMESTAMPTZ,
    traite_par UUID,
    traite_le TIMESTAMPTZ,
    -- Issue (liste fermée). `systeme` = clôture automatique par un fait (localisation, correction, arrivée).
    cloture_code TEXT CHECK (cloture_code IS NULL OR cloture_code IN (
        'localise', 'corrige', 'arrive', 'famille_jointe', 'famille_deja_informee', 'sans_suite', 'fausse_alerte')),
    cloture_par_systeme BOOLEAN NOT NULL DEFAULT false,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_alerte_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_asd_alerte_cle_uq UNIQUE (etablissement_id, kind, cle),
    CONSTRAINT ent_asd_alerte_prise_chk CHECK (statut <> 'prise' OR (pris_par IS NOT NULL AND pris_jusqua IS NOT NULL)),
    CONSTRAINT ent_asd_alerte_traitee_chk CHECK (
        (statut = 'traitee') = (traite_le IS NOT NULL AND cloture_code IS NOT NULL)),
    CONSTRAINT ent_asd_alerte_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id)
);
-- Index de vérification : les alertes à prendre en charge d'un jour, et celles d'un élève.
CREATE INDEX IF NOT EXISTS ent_asd_alerte_file_idx ON ent_asd_alerte (etablissement_id, jour, kind)
    WHERE statut <> 'traitee';
CREATE INDEX IF NOT EXISTS ent_asd_alerte_eleve_idx ON ent_asd_alerte (etablissement_id, eleve_id, jour);

-- Drapeau de vigilance (« élève signalé fragile ») : SANS motif, posé et levé par la vie scolaire ou la direction.
-- Il ne porte aucune information de santé ni de situation familiale ; il ne fait que classer plus haut une
-- absence non expliquée dans la file du matin. Historique en ajout seul : le drapeau n'est jamais supprimé,
-- il est levé (`leve_le`).
CREATE TABLE IF NOT EXISTS ent_asd_vigilance (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    pose_par UUID NOT NULL,
    pose_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    leve_par UUID,
    leve_le TIMESTAMPTZ,
    CONSTRAINT ent_asd_vigilance_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_asd_vigilance_leve_chk CHECK ((leve_le IS NULL) = (leve_par IS NULL)),
    CONSTRAINT ent_asd_vigilance_eleve_fk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id)
);
-- Un seul drapeau vivant par élève.
CREATE UNIQUE INDEX IF NOT EXISTS ent_asd_vigilance_vivant_uq ON ent_asd_vigilance (etablissement_id, eleve_id)
    WHERE leve_le IS NULL;

-- Le drapeau ne se modifie que pour être levé (une fois) et ne se supprime jamais.
CREATE OR REPLACE FUNCTION ent_asd_vigilance_garde() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_asd_vigilance : un drapeau est levé, jamais supprimé' USING ERRCODE = '23514';
    END IF;
    IF OLD.leve_le IS NULL AND NEW.leve_le IS NOT NULL
       AND NEW.id = OLD.id AND NEW.etablissement_id = OLD.etablissement_id AND NEW.eleve_id = OLD.eleve_id
       AND NEW.pose_par = OLD.pose_par AND NEW.pose_le = OLD.pose_le THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'ent_asd_vigilance : un drapeau levé ne se modifie plus' USING ERRCODE = '23514';
END
$$;
DROP TRIGGER IF EXISTS ent_asd_vigilance_garde_trg ON ent_asd_vigilance;
CREATE TRIGGER ent_asd_vigilance_garde_trg BEFORE UPDATE OR DELETE ON ent_asd_vigilance
    FOR EACH ROW EXECUTE FUNCTION ent_asd_vigilance_garde();

SELECT ent_rls_poser(ARRAY['ent_asd_alerte', 'ent_asd_vigilance']);
