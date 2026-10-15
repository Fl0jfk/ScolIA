-- Menus cantine par jour (publication famille).

CREATE TABLE IF NOT EXISTS ent_menu_cantine (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL,
    jour DATE NOT NULL,
    entree TEXT NOT NULL DEFAULT '',
    plat TEXT NOT NULL,
    dessert TEXT NOT NULL DEFAULT '',
    notes TEXT NOT NULL DEFAULT '',
    publie BOOLEAN NOT NULL DEFAULT false,
    updated_by_user_id UUID NOT NULL REFERENCES ent_user (id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ent_menu_cantine_jour_uq UNIQUE (etablissement_id, jour),
    CONSTRAINT ent_menu_cantine_plat_chk CHECK (char_length(trim(plat)) >= 1)
);

CREATE INDEX IF NOT EXISTS ent_menu_cantine_pub_idx
    ON ent_menu_cantine (etablissement_id, jour)
    WHERE publie = true;
