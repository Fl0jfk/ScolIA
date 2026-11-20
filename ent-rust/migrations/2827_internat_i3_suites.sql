-- Socle 41 (internat), lot I3, « suites » — migration 2827.
-- Réf. : docs/socles/41_I3_SUITES.md ; règles générales de 41_I3_CORRECTIFS.md (passes 1 à 4).
-- 2824 à 2826 sont versées : on ne les modifie pas (règle 25). Additive et rejouable (CREATE OR REPLACE,
-- IF NOT EXISTS, DROP … IF EXISTS avant recréation). Aucune colonne JSON, aucun BYTEA, aucune donnée de santé.
--
-- Contenu :
--   1. M2 — la perte d'un droit est datée par un FAIT IMMUABLE (table en ajout seul alimentée par un déclencheur sur
--      `ent_eleve_lien`), plus par `updated_at` que d'autres écritures (payeur, fusion, réactivation) font avancer.
--   2. M1 — un auteur du personnel (aucun rôle parent ou élève dans l'établissement) n'est pas un « auteur sans lien ».
--   3. M3 — « tiers autorisé à récupérer » : proche sans autorité parentale, posé par le personnel à la demande écrite
--      d'un titulaire, daté, motivé, historisé, révocable ; une porte de plus dans l'unique fonction de remise.
--   4. M4 — les états « remis » et « parti_seul_autorise » d'un participant de sortie scolaire ne s'écrivent pas :
--      garde en base (ils n'existent que dans la contrainte de la table, aucun code ne les écrit).

-- ───────────────────────── 1. M2 — faits immuables de perte de droit ─────────────────────────
-- Un fait est posé UNE FOIS, au moment où la perte se produit (autorité parentale retirée, garde « aucune », suspension,
-- fin du lien). `a` = instant de l'écriture (jamais modifié) ; `effet_le` = instant où la perte prend effet : `a`, sauf
-- fin de lien (lendemain de `date_fin` dans le fuseau de l'établissement, si ce jour n'est pas déjà passé). Aucune autre
-- écriture sur le lien (payeur, fusion, réactivation, correction d'un libellé) n'en pose un, donc ne repousse la date.
CREATE TABLE IF NOT EXISTS ent_internat_perte_droit_fait (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    lien_id UUID NOT NULL,
    nature TEXT NOT NULL CHECK (nature IN ('autorite_retiree', 'garde_aucune', 'suspendu', 'fin')),
    date_fin DATE,
    a TIMESTAMPTZ NOT NULL DEFAULT now(),
    effet_le TIMESTAMPTZ NOT NULL,
    CONSTRAINT ent_internat_perte_droit_fait_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_perte_droit_fait_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_internat_perte_droit_fait_fin_chk CHECK ((nature = 'fin') = (date_fin IS NOT NULL))
);
CREATE INDEX IF NOT EXISTS ent_internat_perte_droit_fait_lien_idx
    ON ent_internat_perte_droit_fait (etablissement_id, lien_id, nature, effet_le);
DROP TRIGGER IF EXISTS ent_internat_perte_droit_fait_ajout_seul ON ent_internat_perte_droit_fait;
CREATE TRIGGER ent_internat_perte_droit_fait_ajout_seul BEFORE UPDATE OR DELETE ON ent_internat_perte_droit_fait
    FOR EACH ROW EXECUTE FUNCTION ent_internat_i2_ajout_seul();

-- Reprise de l'existant (une seule fois par lien et nature, rejouable) : les pertes DÉJÀ en vigueur sont datées comme
-- le faisait la règle précédente (suspension, lendemain de la date de fin, dernière modification du lien), seule
-- information disponible ; une levée ou une requalification déjà faite reste donc postérieure ou antérieure comme avant.
INSERT INTO ent_internat_perte_droit_fait (id, etablissement_id, eleve_id, lien_id, nature, date_fin, a, effet_le)
SELECT gen_random_uuid(), l.etablissement_id, l.eleve_id, l.id, x.nature, x.date_fin,
       LEAST(x.effet, now()), x.effet
  FROM ent_eleve_lien l
  CROSS JOIN LATERAL (
        SELECT 'autorite_retiree'::text AS nature, NULL::date AS date_fin, l.updated_at AS effet WHERE NOT l.autorite_parentale
        UNION ALL
        SELECT 'garde_aucune', NULL, l.updated_at WHERE COALESCE(l.garde, '') = 'aucune'
        UNION ALL
        SELECT 'suspendu', NULL, l.suspendu_le WHERE l.suspendu_le IS NOT NULL
        UNION ALL
        SELECT 'fin', l.date_fin,
               ((l.date_fin + 1)::timestamp AT TIME ZONE ent_internat_fuseau(l.etablissement_id))
         WHERE l.date_fin IS NOT NULL) x
 WHERE NOT EXISTS (SELECT 1 FROM ent_internat_perte_droit_fait f
                    WHERE f.etablissement_id = l.etablissement_id AND f.lien_id = l.id AND f.nature = x.nature);

-- Déclencheur sur le lien : seules les TRANSITIONS vers la perte posent un fait (retirer l'autorité parentale, passer la
-- garde à « aucune », suspendre, poser ou changer la date de fin). Rétablir (réactivation) ou toute autre écriture : aucun.
CREATE OR REPLACE FUNCTION ent_internat_perte_droit_poser() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    maintenant TIMESTAMPTZ := now();
BEGIN
    IF (TG_OP = 'INSERT' AND NOT NEW.autorite_parentale)
       OR (TG_OP = 'UPDATE' AND OLD.autorite_parentale AND NOT NEW.autorite_parentale) THEN
        INSERT INTO ent_internat_perte_droit_fait (id, etablissement_id, eleve_id, lien_id, nature, a, effet_le)
        VALUES (gen_random_uuid(), NEW.etablissement_id, NEW.eleve_id, NEW.id, 'autorite_retiree', maintenant, maintenant);
    END IF;
    IF (TG_OP = 'INSERT' AND COALESCE(NEW.garde, '') = 'aucune')
       OR (TG_OP = 'UPDATE' AND COALESCE(OLD.garde, '') <> 'aucune' AND COALESCE(NEW.garde, '') = 'aucune') THEN
        INSERT INTO ent_internat_perte_droit_fait (id, etablissement_id, eleve_id, lien_id, nature, a, effet_le)
        VALUES (gen_random_uuid(), NEW.etablissement_id, NEW.eleve_id, NEW.id, 'garde_aucune', maintenant, maintenant);
    END IF;
    IF (TG_OP = 'INSERT' AND NEW.suspendu_le IS NOT NULL)
       OR (TG_OP = 'UPDATE' AND OLD.suspendu_le IS NULL AND NEW.suspendu_le IS NOT NULL) THEN
        -- L'instant d'écriture, pas la date saisie : une suspension antidatée ne remonte pas dans le temps.
        INSERT INTO ent_internat_perte_droit_fait (id, etablissement_id, eleve_id, lien_id, nature, a, effet_le)
        VALUES (gen_random_uuid(), NEW.etablissement_id, NEW.eleve_id, NEW.id, 'suspendu', maintenant, maintenant);
    END IF;
    IF NEW.date_fin IS NOT NULL AND (TG_OP = 'INSERT' OR NEW.date_fin IS DISTINCT FROM OLD.date_fin) THEN
        INSERT INTO ent_internat_perte_droit_fait (id, etablissement_id, eleve_id, lien_id, nature, date_fin, a, effet_le)
        VALUES (gen_random_uuid(), NEW.etablissement_id, NEW.eleve_id, NEW.id, 'fin', NEW.date_fin, maintenant,
                GREATEST(maintenant,
                         ((NEW.date_fin + 1)::timestamp AT TIME ZONE ent_internat_fuseau(NEW.etablissement_id))));
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_internat_perte_droit_poser ON ent_eleve_lien;
CREATE TRIGGER ent_internat_perte_droit_poser AFTER INSERT OR UPDATE OF autorite_parentale, garde, suspendu_le, date_fin
    ON ent_eleve_lien FOR EACH ROW EXECUTE FUNCTION ent_internat_perte_droit_poser();

-- Le personnel « seul » (M1, revu E2) : compte du personnel SANS AUCUN lien avec l'élève visé. Il n'a alors pas de lien à
-- perdre : ses actes (désignation d'un correspondant, décision d'autorisation annuelle prise ou saisie par le CPE ou la
-- direction) ne tombent pas pour « absence de lien ». Ce n'est PLUS un laissez-passer : un agent qui est aussi proche de
-- l'élève (lien direct ou par une fiche de la chaîne de fusions, compte de l'élève, casquette parent ou élève, rôle parent
-- ou élève) est jugé sur son lien, comme tout titulaire. Une interdiction qui le viserait vaut toujours.
DROP FUNCTION IF EXISTS ent_internat_personnel_seul(UUID, UUID);
CREATE OR REPLACE FUNCTION ent_internat_personnel_seul(p_etab UUID, p_eleve UUID, p_user UUID)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
    SELECT EXISTS (SELECT 1 FROM ent_membership m
                    WHERE m.etablissement_id = p_etab AND m.user_id = p_user AND m.role NOT IN ('parent', 'eleve'))
       AND NOT EXISTS (SELECT 1 FROM ent_membership m
                        WHERE m.etablissement_id = p_etab AND m.user_id = p_user AND m.role IN ('parent', 'eleve'))
       AND NOT EXISTS (SELECT 1 FROM ent_membership_casquette c
                        WHERE c.etablissement_id = p_etab AND c.user_id = p_user AND c.casquette IN ('parent', 'eleve'))
       AND NOT EXISTS (SELECT 1 FROM ent_eleve_compte k
                        WHERE k.etablissement_id = p_etab AND k.eleve_id = p_eleve AND k.user_id = p_user)
       AND NOT EXISTS (SELECT 1 FROM ent_eleve_lien l
                        WHERE l.etablissement_id = p_etab AND l.eleve_id = p_eleve
                          AND (l.user_id = p_user
                               OR l.personne_id IN (
                                    SELECT ent_internat_personne_chaine(p_etab, pe.id)
                                      FROM ent_personne pe
                                     WHERE pe.etablissement_id = p_etab AND pe.user_id = p_user)))
$$;

-- Perte de droit (R1, passe 4) : mêmes règles, datées par les FAITS ci-dessus (2827) et non plus par `updated_at`.
-- Repli (jamais attendu après la reprise de l'existant) : suspendu_le, lendemain de date_fin, `created_at` du lien.
CREATE OR REPLACE FUNCTION ent_internat_perte_droit(
    p_etab UUID, p_eleve UUID, p_personnes UUID[], p_user UUID, p_jour DATE, p_sans_lien_perdu BOOLEAN)
RETURNS TIMESTAMPTZ LANGUAGE sql STABLE AS $$
    WITH liens AS (
        SELECT l.* FROM ent_eleve_lien l
         WHERE l.etablissement_id = p_etab AND l.eleve_id = p_eleve
           AND ((p_user IS NOT NULL AND l.user_id = p_user)
                OR l.personne_id IN (SELECT ent_internat_personne_chaine(p_etab, x) FROM unnest(p_personnes) x))
    ),
    etat AS (
        SELECT (SELECT count(*) FROM liens) AS n,
               EXISTS (SELECT 1 FROM liens l
                        WHERE l.suspendu_le IS NULL AND l.autorite_parentale AND COALESCE(l.garde, '') <> 'aucune'
                          AND (l.date_debut IS NULL OR l.date_debut <= p_jour)
                          AND (l.date_fin IS NULL OR l.date_fin >= p_jour)) AS actif,
               (p_user IS NOT NULL AND EXISTS (SELECT 1 FROM ent_eleve_compte k
                                                WHERE k.etablissement_id = p_etab AND k.eleve_id = p_eleve
                                                  AND k.user_id = p_user)) AS eleve_lui_meme
    )
    SELECT CASE WHEN e.eleve_lui_meme OR e.actif THEN NULL
                WHEN e.n = 0 THEN (CASE WHEN p_sans_lien_perdu THEN '1970-01-01'::timestamptz END)
                ELSE (SELECT max(COALESCE(GREATEST(
                                CASE WHEN l.suspendu_le IS NOT NULL THEN COALESCE(
                                    (SELECT max(f.effet_le) FROM ent_internat_perte_droit_fait f
                                      WHERE f.etablissement_id = p_etab AND f.lien_id = l.id AND f.nature = 'suspendu'),
                                    l.suspendu_le) END,
                                CASE WHEN l.date_fin IS NOT NULL AND l.date_fin < p_jour THEN COALESCE(
                                    (SELECT max(f.effet_le) FROM ent_internat_perte_droit_fait f
                                      WHERE f.etablissement_id = p_etab AND f.lien_id = l.id AND f.nature = 'fin'),
                                    ((l.date_fin + 1)::timestamp AT TIME ZONE ent_internat_fuseau(p_etab))) END,
                                CASE WHEN NOT l.autorite_parentale THEN
                                    (SELECT max(f.effet_le) FROM ent_internat_perte_droit_fait f
                                      WHERE f.etablissement_id = p_etab AND f.lien_id = l.id
                                        AND f.nature = 'autorite_retiree') END,
                                CASE WHEN COALESCE(l.garde, '') = 'aucune' THEN
                                    (SELECT max(f.effet_le) FROM ent_internat_perte_droit_fait f
                                      WHERE f.etablissement_id = p_etab AND f.lien_id = l.id
                                        AND f.nature = 'garde_aucune') END),
                                l.created_at))
                        FROM liens l) END
      FROM etat e
$$;

-- Compte « visé » (passe 4, R1) : interdiction OU perte de droit. E2 : le personnel SEUL (aucun lien avec l'élève) n'a pas
-- de lien à perdre ; tout autre compte est jugé sur son lien (`p_sans_lien_perdu := NOT personnel_seul`), jamais exempté.
CREATE OR REPLACE FUNCTION ent_internat_user_vise(p_etab UUID, p_eleve UUID, p_user UUID, p_jour DATE)
RETURNS TIMESTAMPTZ LANGUAGE sql STABLE AS $$
    SELECT GREATEST(
        (SELECT max(ent_internat_personne_visee(p_etab, p_eleve, x.pid, p_jour))
           FROM (SELECT id AS pid FROM ent_personne WHERE etablissement_id = p_etab AND user_id = p_user
                 UNION
                 SELECT personne_id FROM ent_eleve_lien
                  WHERE etablissement_id = p_etab AND eleve_id = p_eleve AND user_id = p_user) x),
        ent_internat_perte_droit(
            p_etab, p_eleve,
            ARRAY(SELECT id FROM ent_personne WHERE etablissement_id = p_etab AND user_id = p_user),
            p_user, p_jour, NOT ent_internat_personnel_seul(p_etab, p_eleve, p_user)))
$$;

-- ───────────────────────── 3. M3 — tiers autorisé à récupérer ─────────────────────────
-- Un grand-parent, beau-parent ou proche SANS autorité parentale peut être remis à l'élève s'il porte ce statut explicite,
-- posé par le personnel (CPE, direction : droit `internat.sorties.decider`) à la DEMANDE ÉCRITE d'un titulaire de l'autorité
-- parentale (`demande_par_personne_id`, `demande_ecrite_le`), daté (du / au, un an au plus : 365 jours), motivé, historisé (faits
-- `tiers_*` du journal I3) et révocable. Il obéit à toutes les règles générales : interdiction (toujours prioritaire, chaîne
-- de fusions comprise), acte qui tombe avec son auteur (le demandeur devenu visé ou sans droit ferme la porte), refus par
-- défaut, désaccord entre titulaires (« conteste » : le plus restrictif jusqu'à la décision de la direction).
CREATE TABLE IF NOT EXISTS ent_internat_tiers_autorise (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    personne_id UUID NOT NULL,
    demande_par_personne_id UUID NOT NULL,
    demande_ecrite_le DATE NOT NULL,
    du DATE NOT NULL,
    au DATE NOT NULL,
    motif TEXT NOT NULL CHECK (char_length(btrim(motif)) BETWEEN 1 AND 300),
    statut TEXT NOT NULL DEFAULT 'actif' CHECK (statut IN ('actif', 'conteste', 'revoque')),
    pose_par_user_id UUID NOT NULL REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_internat_tiers_autorise_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_internat_tiers_autorise_eleve_cfk FOREIGN KEY (etablissement_id, eleve_id)
        REFERENCES ent_eleve (etablissement_id, id),
    CONSTRAINT ent_internat_tiers_autorise_personne_cfk FOREIGN KEY (etablissement_id, personne_id)
        REFERENCES ent_personne (etablissement_id, id),
    CONSTRAINT ent_internat_tiers_autorise_demandeur_cfk FOREIGN KEY (etablissement_id, demande_par_personne_id)
        REFERENCES ent_personne (etablissement_id, id),
    CONSTRAINT ent_internat_tiers_autorise_dates_chk CHECK (au >= du AND au <= du + 365),
    CONSTRAINT ent_internat_tiers_autorise_distinct_chk CHECK (personne_id <> demande_par_personne_id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ent_internat_tiers_autorise_actif_uq
    ON ent_internat_tiers_autorise (etablissement_id, eleve_id, personne_id) WHERE statut <> 'revoque';
CREATE INDEX IF NOT EXISTS ent_internat_tiers_autorise_eleve_idx
    ON ent_internat_tiers_autorise (etablissement_id, eleve_id) WHERE statut <> 'revoque';

CREATE OR REPLACE FUNCTION ent_internat_tiers_autorise_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'ent_internat_tiers_autorise : suppression interdite (R-IT3)' USING ERRCODE = '42501';
    END IF;
    IF (OLD.id, OLD.etablissement_id, OLD.eleve_id, OLD.personne_id, OLD.demande_ecrite_le, OLD.du, OLD.au, OLD.motif,
        OLD.pose_par_user_id, OLD.cree_le)
       IS DISTINCT FROM (NEW.id, NEW.etablissement_id, NEW.eleve_id, NEW.personne_id, NEW.demande_ecrite_le, NEW.du,
        NEW.au, NEW.motif, NEW.pose_par_user_id, NEW.cree_le) THEN
        RAISE EXCEPTION 'tiers autorisé : élève, personne, dates, motif et demande écrite ne se modifient pas'
            USING ERRCODE = '42501';
    END IF;
    IF NEW.statut <> OLD.statut AND NOT (
           (OLD.statut = 'actif' AND NEW.statut IN ('conteste', 'revoque'))
        OR (OLD.statut = 'conteste' AND NEW.statut IN ('actif', 'revoque'))) THEN
        RAISE EXCEPTION 'tiers autorisé : transition % → % refusée', OLD.statut, NEW.statut USING ERRCODE = '42501';
    END IF;
    IF OLD.statut = 'revoque' AND (OLD.statut, OLD.demande_par_personne_id)
       IS DISTINCT FROM (NEW.statut, NEW.demande_par_personne_id) THEN
        RAISE EXCEPTION 'tiers autorisé : révoqué, plus de modification' USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_internat_tiers_autorise_garde ON ent_internat_tiers_autorise;
CREATE TRIGGER ent_internat_tiers_autorise_garde BEFORE UPDATE OR DELETE ON ent_internat_tiers_autorise
    FOR EACH ROW EXECUTE FUNCTION ent_internat_tiers_autorise_garde();

-- Faits du tiers autorisé dans le journal I3 (ajout seul) : colonne, actes, un seul objet visé par fait.
ALTER TABLE ent_internat_i3_fait ADD COLUMN IF NOT EXISTS tiers_id UUID;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_internat_i3_fait_tiers_cfk') THEN
        ALTER TABLE ent_internat_i3_fait ADD CONSTRAINT ent_internat_i3_fait_tiers_cfk
            FOREIGN KEY (etablissement_id, tiers_id) REFERENCES ent_internat_tiers_autorise (etablissement_id, id);
    END IF;
END
$$;
CREATE INDEX IF NOT EXISTS ent_internat_i3_fait_tiers_idx
    ON ent_internat_i3_fait (etablissement_id, tiers_id, a) WHERE tiers_id IS NOT NULL;
ALTER TABLE ent_internat_i3_fait DROP CONSTRAINT IF EXISTS ent_internat_i3_fait_acte_check;
ALTER TABLE ent_internat_i3_fait ADD CONSTRAINT ent_internat_i3_fait_acte_check CHECK (acte IN (
    'annonce', 'demande', 'regularisation_demandee', 'accord_titulaire', 'acceptation', 'refus', 'blocage',
    'opposition', 'tranchee_maintenue', 'tranchee_levee', 'regularisation', 'annulation', 'depart', 'retour',
    'retard_annonce', 'revocation', 'suspension', 'reprise', 'expiration', 'designation', 'fin',
    'depart_refuse', 'accord_remis_a_zero', 'retard_demande', 'retard_refuse', 'signalement', 'requalification',
    'tiers_pose', 'tiers_opposition', 'tiers_levee', 'tiers_maintenu', 'tiers_revocation', 'tiers_reprise',
    'tiers_echeance'));
ALTER TABLE ent_internat_i3_fait DROP CONSTRAINT IF EXISTS ent_internat_i3_fait_objet_chk;
ALTER TABLE ent_internat_i3_fait ADD CONSTRAINT ent_internat_i3_fait_objet_chk CHECK (
    (acte <> 'requalification'
     AND (absence_id IS NOT NULL)::int + (sortie_id IS NOT NULL)::int + (correspondant_id IS NOT NULL)::int
         + (tiers_id IS NOT NULL)::int = 1)
    OR (acte = 'requalification' AND eleve_id IS NOT NULL AND personne_id IS NOT NULL
        AND absence_id IS NULL AND sortie_id IS NULL AND correspondant_id IS NULL AND tiers_id IS NULL));

-- Un lien de l'élève (chaîne de fusions comprise) écarte la personne : suspendu, garde « aucune », terminé ou pas encore
-- commencé, OU sans autorité parentale pour un lien de type `mere`, `pere` ou `tuteur` (un parent ou tuteur privé de
-- l'autorité ne se rétablit pas comme tiers : décision de justice). Un proche de lien `autre` sans autorité reste posable
-- (c'est le cas même du grand-parent). DÉFINITION UNIQUE : lue par la porte « contact » ET la porte « tiers autorisé » de
-- `ent_internat_remise`, et par la pose du statut (service) ; une autorité retirée après coup ferme donc les deux portes.
CREATE OR REPLACE FUNCTION ent_internat_lien_ecarte(p_etab UUID, p_eleve UUID, p_personne UUID, p_jour DATE)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
    SELECT EXISTS (
        SELECT 1 FROM ent_eleve_lien sl
         WHERE sl.etablissement_id = p_etab AND sl.eleve_id = p_eleve
           AND sl.personne_id IN (SELECT ent_internat_personne_chaine(p_etab, p_personne))
           AND (sl.suspendu_le IS NOT NULL OR COALESCE(sl.garde, '') = 'aucune'
                OR (sl.date_fin IS NOT NULL AND sl.date_fin < p_jour)
                OR (sl.date_debut IS NOT NULL AND sl.date_debut > p_jour)
                OR (NOT sl.autorite_parentale AND lower(btrim(sl.lien)) IN ('mere', 'pere', 'tuteur'))))
$$;

-- La porte unique de remise (2826), avec la porte « tiers autorisé » : statut « actif » aujourd'hui, personne non fusionnée,
-- aucun lien écarté, et le titulaire qui l'a demandé n'est NI visé (interdiction) NI privé de son droit (« un acte tombe
-- avec son auteur »). Une interdiction qui vise la personne reste prioritaire (`visee`).
CREATE OR REPLACE FUNCTION ent_internat_remise(
    p_etab UUID, p_eleve UUID, p_personne UUID, p_jour DATE, p_sans_cible_levee BOOLEAN)
RETURNS TABLE (code TEXT, relation TEXT) LANGUAGE sql STABLE AS $$
    WITH base AS (
        SELECT ent_internat_personne_visee(p_etab, p_eleve, p_personne, p_jour) IS NOT NULL AS visee,
               (NOT p_sans_cible_levee
                AND ent_internat_interdiction_sans_cible(p_etab, p_eleve, p_jour, false) IS NOT NULL) AS sans_cible,
               (SELECT x.rel FROM (
                    SELECT 1 AS ord, COALESCE(NULLIF(btrim(l.lien), ''), 'titulaire') AS rel
                      FROM ent_eleve_lien l
                      JOIN ent_personne p ON p.etablissement_id = l.etablissement_id AND p.id = l.personne_id
                     WHERE l.etablissement_id = p_etab AND l.eleve_id = p_eleve AND l.personne_id = p_personne
                       AND l.suspendu_le IS NULL AND l.autorite_parentale AND COALESCE(l.garde, '') <> 'aucune'
                       AND (l.date_debut IS NULL OR l.date_debut <= p_jour)
                       AND (l.date_fin IS NULL OR l.date_fin >= p_jour)
                       AND p.fusionnee_dans IS NULL
                    UNION ALL
                    SELECT 2, COALESCE(NULLIF(btrim(c.relation), ''), 'personne autorisée')
                      FROM ent_eleve_contact_urgence c
                      JOIN ent_personne p ON p.etablissement_id = c.etablissement_id AND p.id = c.personne_id
                     WHERE c.etablissement_id = p_etab AND c.eleve_id = p_eleve AND c.personne_id = p_personne
                       AND c.peut_recuperer AND c.statut = 'valide' AND p.fusionnee_dans IS NULL
                       AND NOT ent_internat_lien_ecarte(p_etab, p_eleve, p_personne, p_jour)
                       AND NOT ent_internat_contact_suspect(p_etab, p_eleve, p_personne, p_jour)
                    UNION ALL
                    SELECT 3, 'tiers autorisé'
                      FROM ent_internat_tiers_autorise t
                      JOIN ent_personne p ON p.etablissement_id = t.etablissement_id AND p.id = t.personne_id
                     WHERE t.etablissement_id = p_etab AND t.eleve_id = p_eleve AND t.personne_id = p_personne
                       AND t.statut = 'actif' AND t.du <= p_jour AND t.au >= p_jour AND p.fusionnee_dans IS NULL
                       AND NOT ent_internat_lien_ecarte(p_etab, p_eleve, p_personne, p_jour)
                       AND GREATEST(
                            ent_internat_personne_visee(p_etab, p_eleve, t.demande_par_personne_id, p_jour),
                            ent_internat_perte_droit(p_etab, p_eleve, ARRAY[t.demande_par_personne_id], NULL,
                                                     p_jour, true)) IS NULL
                ) x ORDER BY x.ord, x.rel LIMIT 1) AS rel
    )
    SELECT CASE WHEN visee THEN 'interdite'
                WHEN sans_cible THEN 'sans_cible'
                WHEN rel IS NOT NULL THEN 'autorisee'
                ELSE 'non_autorisee' END,
           CASE WHEN visee OR sans_cible THEN NULL ELSE rel END
      FROM base
$$;

-- ───────────────────────── 4. M4 — remise d'un participant de sortie scolaire ─────────────────────────
-- Les états « remis » et « parti_seul_autorise » de `ent_sortie_participant` n'existent que dans la contrainte de la table
-- (migration 641) : aucun code ne les écrit. La garde passe en BASE : elle refuse toute écriture de ces états, quel que soit
-- le code qui s'y essaie, jusqu'à ce qu'un lot les écrive par `ent_internat_remise` (il remplacera alors cette garde).
CREATE OR REPLACE FUNCTION ent_internat_remise_participant_garde() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.etat IN ('remis', 'parti_seul_autorise') AND (TG_OP = 'INSERT' OR NEW.etat IS DISTINCT FROM OLD.etat) THEN
        RAISE EXCEPTION 'ent_sortie_participant : l''état % ne s''écrit pas sans passer par la fonction de remise de l''internat (ent_internat_remise)',
            NEW.etat USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END
$$;
DROP TRIGGER IF EXISTS ent_internat_remise_participant_garde ON ent_sortie_participant;
CREATE TRIGGER ent_internat_remise_participant_garde BEFORE INSERT OR UPDATE OF etat ON ent_sortie_participant
    FOR EACH ROW EXECUTE FUNCTION ent_internat_remise_participant_garde();

-- ───────────────────────── Registres communs ─────────────────────────
DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['ent_internat_perte_droit_fait', 'ent_internat_tiers_autorise']
    LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', t || '_sans_truncate', t);
        EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT '
                       'EXECUTE FUNCTION ent_internat_sans_truncate()', t || '_sans_truncate', t);
    END LOOP;
END
$$;

SELECT ent_rls_poser(ARRAY['ent_internat_perte_droit_fait', 'ent_internat_tiers_autorise']);

-- Journal des lectures sensibles (R-IT38) : l'objet « tiers » (liste des tiers autorisés d'un élève).
ALTER TABLE ent_internat_lecture_sensible DROP CONSTRAINT IF EXISTS ent_internat_lecture_sensible_objet_check;
ALTER TABLE ent_internat_lecture_sensible ADD CONSTRAINT ent_internat_lecture_sensible_objet_check CHECK (objet IN (
    'fiche_nuit', 'liaison', 'presents', 'registre', 'cas', 'appel', 'appel_pdf', 'attendu',
    'liste_appels', 'procedure', 'effectif_repas', 'listes', 'journal',
    'liste_remise', 'sorties', 'correspondants', 'tiers'));
