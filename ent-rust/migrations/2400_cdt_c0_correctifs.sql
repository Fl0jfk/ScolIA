-- Socle 20 (cahier de textes), lot C0 : correctifs urgents sur les tables du lot S2-6 (migration 150).
-- Additif et rejouable ; aucune migration existante n'est modifiée (PILOTAGE règle 15).
--
-- C-D1 (contenu détruit à la régénération des séances) est traité par le lot T0 du socle 10, codé en
-- parallèle sur `ent-socle-10-code-1` : rien ici (aucune clé, aucun déclencheur sur `ent_edt_seance`).
--
-- 1. Portée de saisie au cahier, calculée à la DATE de l'action par les interventions (socle 6).
-- 2. Séance -> cours.
-- 3. Fin de journée dans le fuseau de l'établissement (C-D13).
-- 4. Nom d'affichage de l'auteur (C-D15) : plus d'identifiant de compte vers les élèves et familles.
-- 5. Retrait d'une pièce (C-D12) ; retrait d'un travail par la direction, avec motif (C-D10).
-- 6. Historique : les retours (C-D8) rejoignent `ent_cdt_historique` (instantané figé, exception D-036 déjà
--    déclarée pour cette table).

-- 1. Saisie au cahier à la date ------------------------------------------------------------------------------
-- Comme `ent_prof_sur_cours(…, p_saisie => true)` (355) : seules les saisies EFFECTIVES ouvrent le droit (le
-- titulaire perd la saisie pendant une suppléance ; assistants et intervenants ne saisissent jamais) ; en plus,
-- la restriction facultative `droits` de l'intervention doit contenir `cahier` (NULL = aucune restriction).
-- Repli : un service sans aucune intervention (compte sans agent) garde son `prof_user_id`.
CREATE OR REPLACE FUNCTION ent_cdt_saisie_cours(p_etab UUID, p_cours UUID, p_user UUID, p_jour DATE)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
    SELECT EXISTS (
        SELECT 1
          FROM ent_service_enseignement s
         WHERE s.etablissement_id = p_etab AND s.cours_id = p_cours
           AND s.etat IN ('ouvert', 'clos')
           AND (
                EXISTS (
                    SELECT 1 FROM ent_service_intervenants_a(p_etab, s.id, p_jour) x
                      JOIN ent_agent ag ON ag.etablissement_id = p_etab AND ag.id = x.agent_id
                      JOIN ent_personne pe ON pe.etablissement_id = ag.etablissement_id AND pe.id = ag.personne_id
                     WHERE pe.user_id = p_user AND x.saisie_effective
                       AND (x.droits IS NULL OR 'cahier' = ANY (x.droits)))
                OR (s.prof_user_id = p_user
                    AND NOT EXISTS (SELECT 1 FROM ent_service_intervenant z
                                     WHERE z.etablissement_id = s.etablissement_id AND z.service_id = s.id))))
$$;

-- 2. Séance -> cours ------------------------------------------------------------------------------------------
-- Le cours du service de la séance ; à défaut celui du créneau ; NULL pour une séance « classe texte » sans
-- service (reprise historique : le code applique alors son repli par titulaire, jamais par classe entière).
CREATE OR REPLACE FUNCTION ent_cdt_cours_de_seance(p_etab UUID, p_seance UUID)
RETURNS UUID LANGUAGE sql STABLE AS $$
    SELECT COALESCE(
        (SELECT s.cours_id FROM ent_service_enseignement s
          WHERE s.etablissement_id = x.etablissement_id AND s.id = x.service_id),
        (SELECT c.cours_id FROM ent_edt_creneau c
          WHERE c.etablissement_id = x.etablissement_id AND c.id = x.creneau_id))
      FROM ent_edt_seance x
     WHERE x.etablissement_id = p_etab AND x.id = p_seance
$$;

-- 3. Fin de journée à l'heure de l'établissement ----------------------------------------------------------------
CREATE OR REPLACE FUNCTION ent_cdt_fin_de_journee(p_etab UUID, p_jour DATE)
RETURNS TIMESTAMPTZ LANGUAGE sql STABLE AS $$
    SELECT ((p_jour + 1)::timestamp - interval '1 second') AT TIME ZONE COALESCE(
        (SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = p_etab),
        'Europe/Paris')
$$;

-- 4. Nom d'affichage ----------------------------------------------------------------------------------------------
-- Prénom et nom de la fiche personne rattachée au compte ; à défaut le nom d'affichage du compte ; à défaut
-- un libellé neutre. Jamais l'identifiant ni l'adresse de messagerie.
CREATE OR REPLACE FUNCTION ent_cdt_nom_affichage(p_etab UUID, p_user UUID)
RETURNS TEXT LANGUAGE sql STABLE AS $$
    SELECT COALESCE(
        (SELECT NULLIF(btrim(coalesce(pe.prenom, '') || ' ' || coalesce(pe.nom, '')), '')
           FROM ent_personne pe
          WHERE pe.etablissement_id = p_etab AND pe.user_id = p_user
          ORDER BY pe.id LIMIT 1),
        (SELECT NULLIF(btrim(u.display_name), '') FROM ent_user u WHERE u.id = p_user),
        'Enseignant')
$$;

-- 5. Retraits ------------------------------------------------------------------------------------------------------
ALTER TABLE ent_cdt_piece ADD COLUMN IF NOT EXISTS retiree_le TIMESTAMPTZ;
ALTER TABLE ent_cdt_piece ADD COLUMN IF NOT EXISTS retiree_par UUID;

-- La pièce retirée n'est plus servie, mais la ligne et l'objet restent (jamais de suppression silencieuse).
CREATE INDEX IF NOT EXISTS ent_cdt_piece_actives_idx
    ON ent_cdt_piece (etablissement_id, travail_id, seance_contenu_id, remise_id) WHERE retiree_le IS NULL;

-- Retrait par la direction : le travail sort de la diffusion (`supprime_le` posé en même temps), avec auteur et
-- motif ; la direction ne réécrit jamais le travail d'un collègue.
ALTER TABLE ent_travail ADD COLUMN IF NOT EXISTS retire_par_user_id UUID;
ALTER TABLE ent_travail ADD COLUMN IF NOT EXISTS retire_motif TEXT;
ALTER TABLE ent_travail DROP CONSTRAINT IF EXISTS ent_travail_retrait_chk;
ALTER TABLE ent_travail ADD CONSTRAINT ent_travail_retrait_chk CHECK (
    (retire_par_user_id IS NULL) = (retire_motif IS NULL)
    AND (retire_motif IS NULL OR length(btrim(retire_motif)) BETWEEN 3 AND 500));

-- 6. Historique des retours ------------------------------------------------------------------------------------------
ALTER TABLE ent_cdt_historique DROP CONSTRAINT IF EXISTS ent_cdt_historique_objet_chk;
ALTER TABLE ent_cdt_historique ADD CONSTRAINT ent_cdt_historique_objet_chk
    CHECK (objet IN ('travail', 'seance_contenu', 'remise', 'visa'));
