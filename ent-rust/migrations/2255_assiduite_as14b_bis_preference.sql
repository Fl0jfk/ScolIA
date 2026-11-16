-- 2255 — Socle 12, lot AS14b-bis. Disposition personnelle du tableau de bord de la vie scolaire.
-- Une ligne par (utilisateur, panneau) : rang épinglé, panneau replié, panneau masqué. AUCUNE ligne = mode
-- « automatique » (disposition selon l'heure). « Revenir à automatique » = supprimer les lignes de l'utilisateur.
-- Mémorisé côté serveur pour que l'utilisateur retrouve sa disposition sur un autre poste. Le bandeau « Urgent »
-- n'est pas un panneau : il n'a ni ligne ni option, il ne se masque jamais.
-- Donnée personnelle d'organisation (aucune donnée d'élève, aucun texte libre : codes seulement).
-- Additive, rejouable (E8), sans colonne JSON, sans BYTEA ; RLS par `ent_rls_poser`.

CREATE TABLE IF NOT EXISTS ent_asd_tableau_pref (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    user_id UUID NOT NULL REFERENCES ent_user (id) ON DELETE CASCADE,
    panneau TEXT NOT NULL CHECK (panneau IN ('absents', 'exclus', 'appels', 'incoherences', 'familles')),
    rang SMALLINT NOT NULL CHECK (rang BETWEEN 1 AND 5),
    replie BOOLEAN NOT NULL DEFAULT false,
    masque BOOLEAN NOT NULL DEFAULT false,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_asd_tableau_pref_panneau_uq UNIQUE (etablissement_id, user_id, panneau),
    CONSTRAINT ent_asd_tableau_pref_rang_uq UNIQUE (etablissement_id, user_id, rang)
);

SELECT ent_rls_poser(ARRAY['ent_asd_tableau_pref']);
