-- Socle 2 (plage 3200–3219) : un seul « jour de l'établissement ».
--
-- `ent_jour_etablissement` (283, socle 2) lisait la table provisoire `ent_param_eleve_fuseau`, alors que le
-- réglage officiel du fuseau (`definir_fuseau_etablissement_for_actor`, socle 4) écrit dans
-- `ent_etablissement_parametre`, lu par `ent_aujourdhui_etab` (265). Un établissement réglé sur
-- Indian/Reunion, America/Cayenne ou à l'étranger avait donc deux jours différents selon le module
-- (constaté par le test `fuseau_jour_meme_autour_de_minuit`, rouge de 11 h à minuit UTC).
-- Désormais : le réglage officiel d'abord, la table provisoire ensuite (compatibilité), Europe/Paris sinon.
-- Rejouable ; aucune donnée modifiée.

CREATE OR REPLACE FUNCTION ent_jour_etablissement(p_etab UUID) RETURNS DATE
LANGUAGE sql STABLE AS $$
    SELECT (now() AT TIME ZONE coalesce(
        (SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = p_etab),
        (SELECT f.fuseau FROM ent_param_eleve_fuseau f WHERE f.etablissement_id = p_etab),
        'Europe/Paris'))::date
$$;
