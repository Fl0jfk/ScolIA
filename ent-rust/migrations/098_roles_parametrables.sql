-- D-017 : rôles paramétrables par l'établissement (docs/ROLES_ET_DROITS.md).
-- Additif : sans rôle attribué, une personne garde les droits de son profil de
-- base (`ent_membership.role`). Les codes d'action et de portée sont validés
-- par le serveur (catalogue `ent_auth::Action` / `ent_auth::Portee`).

CREATE TABLE IF NOT EXISTS ent_role (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    nom TEXT NOT NULL,
    description TEXT,
    -- Modèle d'origine (profil historique copié), information seulement.
    modele TEXT,
    created_by_user_id UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_role_nom_chk CHECK (length(btrim(nom)) BETWEEN 1 AND 80)
);

CREATE UNIQUE INDEX IF NOT EXISTS ent_role_nom_uq
    ON ent_role (etablissement_id, lower(btrim(nom)));

CREATE TABLE IF NOT EXISTS ent_role_droit (
    role_id UUID NOT NULL REFERENCES ent_role (id) ON DELETE CASCADE,
    etablissement_id UUID NOT NULL,
    action TEXT NOT NULL,
    portee TEXT NOT NULL,
    -- Portée « designes » : classes (`ent_classe`) ou groupes (`ent_groupe`).
    cibles UUID[] NOT NULL DEFAULT '{}',
    PRIMARY KEY (role_id, action, portee)
);

CREATE TABLE IF NOT EXISTS ent_role_attribution (
    role_id UUID NOT NULL REFERENCES ent_role (id) ON DELETE CASCADE,
    etablissement_id UUID NOT NULL,
    user_id UUID NOT NULL REFERENCES ent_user (id),
    debut DATE NOT NULL DEFAULT CURRENT_DATE,
    fin DATE,
    attribue_par_user_id UUID REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (role_id, user_id),
    CONSTRAINT ent_role_attribution_dates_chk CHECK (fin IS NULL OR fin >= debut)
);

CREATE INDEX IF NOT EXISTS ent_role_attribution_user_idx
    ON ent_role_attribution (etablissement_id, user_id);

-- Responsables des groupes et ateliers (théâtre, chorale…).
CREATE TABLE IF NOT EXISTS ent_groupe_responsable (
    groupe_id UUID NOT NULL REFERENCES ent_groupe (id) ON DELETE CASCADE,
    etablissement_id UUID NOT NULL,
    user_id UUID NOT NULL REFERENCES ent_user (id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (groupe_id, user_id)
);

CREATE INDEX IF NOT EXISTS ent_groupe_responsable_user_idx
    ON ent_groupe_responsable (etablissement_id, user_id);
