-- 2582 — Socle 29, lot P2 : ce que l'établissement peut régler sur une nature de tâche (peu,
-- D-023). Réf. : 29_PORTAILS.md §22.2, §29.4. Les natures sont déclarées DANS LE CODE : cette
-- table ne porte que des réglages, jamais une définition. Migration additive et rejouable.

CREATE TABLE IF NOT EXISTS ent_afaire_nature_reglage (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    nature_code TEXT NOT NULL CHECK (length(nature_code) BETWEEN 3 AND 80),
    active BOOLEAN NOT NULL DEFAULT true,
    -- Pendant l'année de doublon (Q-PO4) : la tâche reste normale, devient simple information, ou
    -- n'est pas ouverte.
    pendant_doublon TEXT NOT NULL DEFAULT 'normal'
        CHECK (pendant_doublon IN ('normal', 'information', 'desactivee')),
    -- Délai d'escalade, dans les bornes déclarées par la nature (contrôlé par le code).
    delai_escalade_heures INTEGER CHECK (delai_escalade_heures IS NULL OR delai_escalade_heures BETWEEN 1 AND 720),
    -- R-PO37 : tâche/changement visible en consultation pendant la plage calme, sans notification.
    ajustement_plage_calme BOOLEAN NOT NULL DEFAULT false,
    modifie_par UUID,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (etablissement_id, nature_code)
);

SELECT ent_rls_poser(ARRAY['ent_afaire_nature_reglage']);
