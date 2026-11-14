-- Socle 20, lot C3 — migration 2414 : agenda, cahier de l'élève et de la famille, résumé hebdomadaire, abonnement calendrier.
-- Réf. : 20_CAHIER_DE_TEXTES.md §9.2, §9.3, §20.7, §20.8 (CDT-08, 09, 160–163, 176–180), R-C14, R-C37, R-C42 ; D-036 (aucune
-- colonne JSON) ; jour et heure de l'établissement (jamais le jour UTC).

-- 1. Abonnement au calendrier (CDT-176) : jeton révocable, **haché** (SHA-256). Le jeton en clair n'est montré qu'une fois,
-- à la création ; l'adresse d'abonnement porte l'établissement (pour poser le contexte de ligne) et le secret. Le flux ne
-- contient jamais de consigne détaillée (socle 9) : matière, nature, titre, échéance.
CREATE TABLE IF NOT EXISTS ent_cdt_abonnement (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    user_id UUID NOT NULL REFERENCES ent_user (id),
    eleve_id UUID NOT NULL,
    jeton_hash CHAR(64) NOT NULL CHECK (jeton_hash ~ '^[0-9a-f]{64}$'),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoque_le TIMESTAMPTZ,
    CONSTRAINT ent_cdt_abonnement_etab_id_uq UNIQUE (etablissement_id, id),
    CONSTRAINT ent_cdt_abonnement_jeton_uq UNIQUE (jeton_hash),
    CONSTRAINT ent_cdt_abonnement_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS ent_cdt_abonnement_user_idx ON ent_cdt_abonnement (etablissement_id, user_id) WHERE revoque_le IS NULL;

-- 2. Résumé hebdomadaire « la semaine de mon enfant » (CDT-180) : un envoi par élève et par semaine (lundi de la semaine
-- concernée, jour de l'établissement). Ce qui est stocké est un décompte, jamais un contenu.
CREATE TABLE IF NOT EXISTS ent_cdt_resume_envoi (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    eleve_id UUID NOT NULL,
    semaine_debut DATE NOT NULL CHECK (EXTRACT(ISODOW FROM semaine_debut) = 1),
    nb_travaux SMALLINT NOT NULL CHECK (nb_travaux >= 0),
    nb_evaluations SMALLINT NOT NULL CHECK (nb_evaluations >= 0),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (eleve_id, semaine_debut),
    CONSTRAINT ent_cdt_resume_envoi_eleve_fk FOREIGN KEY (eleve_id, etablissement_id)
        REFERENCES ent_eleve (id, etablissement_id) ON DELETE RESTRICT
);

SELECT ent_rls_poser(ARRAY['ent_cdt_abonnement', 'ent_cdt_resume_envoi']);

-- 3. Lecture de l'agenda : travaux d'un élève, par échéance effective (la prolongation individuelle la plus récente fait
-- foi). Index des travaux publiés par échéance.
CREATE INDEX IF NOT EXISTS ent_cdt_travail_agenda_idx ON ent_cdt_travail (etablissement_id, echeance_le)
    WHERE etat = 'publie';
CREATE INDEX IF NOT EXISTS ent_cdt_travail_donne_idx ON ent_cdt_travail (etablissement_id, seance_donnee_id)
    WHERE seance_donnee_id IS NOT NULL;

-- 4. Élève majeur au jour de l'établissement : les responsables ne lisent plus rien de lui (le lien de l'élève majeur avec
-- ses responsables est fermé tant que l'élève ne l'a pas rouvert, socle 4 ; l'ouverture n'existe pas encore).
CREATE OR REPLACE FUNCTION ent_cdt_eleve_majeur(p_etab UUID, p_eleve UUID)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
    SELECT COALESCE((SELECT e.date_naissance IS NOT NULL
                            AND e.date_naissance + INTERVAL '18 years' <= ent_aujourdhui_etab(p_etab)
                       FROM ent_eleve e WHERE e.etablissement_id = p_etab AND e.id = p_eleve), false)
$$;

-- 5. Statut **constaté** d'un travail pour un élève (R-C12 : un fait, jamais un zéro). Ordre de priorité : le constat du
-- professeur (le plus récent), puis la remise (retour rendu, dépôt en retard, dépôt), puis, pour un rendu en ligne, l'absence
-- de dépôt (« à faire » avant l'échéance, « non rendu » après). Un rendu en classe sans constat n'a pas de statut.
CREATE OR REPLACE FUNCTION ent_cdt_statut_eleve(p_etab UUID, p_travail UUID, p_eleve UUID, p_echeance TIMESTAMPTZ, p_rendu TEXT)
RETURNS TEXT LANGUAGE sql STABLE AS $$
    SELECT COALESCE(
        (SELECT CASE s.statut WHEN 'rendu_papier' THEN 'rendu' WHEN 'non_rendu' THEN 'non_rendu'
                              WHEN 'dispense' THEN 'dispense' WHEN 'excuse' THEN 'excuse' ELSE NULL END
           FROM ent_cdt_statut s
          WHERE s.etablissement_id = p_etab AND s.travail_id = p_travail AND s.eleve_id = p_eleve
          ORDER BY s.cree_le DESC, s.id DESC LIMIT 1),
        (SELECT CASE WHEN rt.etat = 'rendu' THEN 'retour_disponible'
                     WHEN r.etat = 'deposee' AND r.en_retard THEN 'en_retard'
                     WHEN r.etat = 'deposee' THEN 'rendu' ELSE NULL END
           FROM ent_cdt_remise r
           LEFT JOIN ent_cdt_retour rt ON rt.etablissement_id = r.etablissement_id AND rt.remise_id = r.id
          WHERE r.etablissement_id = p_etab AND r.travail_id = p_travail
            AND (r.eleve_id = p_eleve
                 OR r.groupe_id IN (SELECT m.groupe_id FROM ent_cdt_groupe_membre m
                                     WHERE m.etablissement_id = p_etab AND m.eleve_id = p_eleve AND m.jusqu_au IS NULL))
          ORDER BY r.cree_le LIMIT 1),
        CASE WHEN p_rendu = 'en_ligne' THEN CASE WHEN p_echeance < now() THEN 'non_rendu' ELSE 'a_faire' END END)
$$;

-- 6. Échéance effective d'un élève : la prolongation individuelle la plus récente, sinon l'échéance du travail.
CREATE OR REPLACE FUNCTION ent_cdt_echeance_effective(p_etab UUID, p_travail UUID, p_eleve UUID, p_echeance TIMESTAMPTZ)
RETURNS TIMESTAMPTZ LANGUAGE sql STABLE AS $$
    SELECT COALESCE((SELECT x.echeance_le FROM ent_cdt_echeance_eleve x
                      WHERE x.etablissement_id = p_etab AND x.travail_id = p_travail AND x.eleve_id = p_eleve
                      ORDER BY x.cree_le DESC, x.id DESC LIMIT 1), p_echeance)
$$;
