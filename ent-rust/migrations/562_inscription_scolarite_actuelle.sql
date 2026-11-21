-- D-036 — `ent_inscription_dossier.scolarite_actuelle` (JSONB « affichée, jamais requêtée »,
-- migration 304) devient cinq colonnes typées. Aucun code Rust ne lit ni n'écrit l'ancienne
-- colonne (constat de l'inventaire D-036) : elle est conservée en lecture seule et sera retirée
-- dans un lot ultérieur (lot D-036 « retrait des colonnes reprises »).

ALTER TABLE ent_inscription_dossier
    ADD COLUMN IF NOT EXISTS scol_etablissement_libelle TEXT
        CHECK (scol_etablissement_libelle IS NULL OR length(scol_etablissement_libelle) <= 200),
    ADD COLUMN IF NOT EXISTS scol_uai TEXT
        CHECK (scol_uai IS NULL OR scol_uai ~ '^[0-9]{7}[A-Za-z]$'),
    ADD COLUMN IF NOT EXISTS scol_type TEXT
        CHECK (scol_type IS NULL OR length(scol_type) <= 80),
    ADD COLUMN IF NOT EXISTS scol_niveau TEXT
        CHECK (scol_niveau IS NULL OR length(scol_niveau) <= 40),
    ADD COLUMN IF NOT EXISTS scol_annee TEXT
        CHECK (scol_annee IS NULL OR length(scol_annee) <= 20);

-- Reprise idempotente : seuls les dossiers dont les colonnes typées sont encore toutes vides.
UPDATE ent_inscription_dossier d
   SET scol_etablissement_libelle = left(nullif(btrim(d.scolarite_actuelle ->> 'etablissement_libelle'), ''), 200),
       scol_uai = CASE WHEN d.scolarite_actuelle ->> 'uai' ~ '^[0-9]{7}[A-Za-z]$'
                       THEN d.scolarite_actuelle ->> 'uai' END,
       scol_type = left(nullif(btrim(d.scolarite_actuelle ->> 'type'), ''), 80),
       scol_niveau = left(nullif(btrim(d.scolarite_actuelle ->> 'niveau'), ''), 40),
       scol_annee = left(nullif(btrim(d.scolarite_actuelle ->> 'annee'), ''), 20)
 WHERE jsonb_typeof(d.scolarite_actuelle) = 'object'
   AND d.scolarite_actuelle <> '{}'::jsonb
   AND d.scol_etablissement_libelle IS NULL AND d.scol_uai IS NULL AND d.scol_type IS NULL
   AND d.scol_niveau IS NULL AND d.scol_annee IS NULL;
