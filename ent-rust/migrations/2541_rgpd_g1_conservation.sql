-- 2541 — Socle 27, lot G1 : référentiel de conservation par établissement et désignation du DPO.
-- Réf. : 27_RGPD_ARCHIVES.md §8.2, §17.1, §17.2. Migration additive et rejouable, sans JSON ni tableau.
--
-- * Le CATALOGUE des catégories (durées proposées, sources) vit dans le code (`ent-db/src/conservation.rs`,
--   R-RG13) : il n'est pas dupliqué en base (`cons_categorie` et `cons_categorie_proposition` de la
--   conception ne sont pas créées, pour éviter une dérive entre code et base). Ces tables ne portent que
--   ce que décide l'établissement.
-- * `rgp_gouvernance` : créée ici (la conception la plaçait en 2542) parce que « un DPO est-il nommé ? »
--   conditionne l'affichage « en attente de DPO » du G1. Le DPO peut ne pas exister : toutes les colonnes
--   du DPO sont alors nulles ; rien d'autre que la purge réelle n'en dépend.
-- * `cons_regle` : une règle par catégorie et par version ; une seule règle VALIDÉE en vigueur et une seule
--   proposition ouverte par catégorie (index partiels). Tant qu'aucune règle n'est validée : aucun sort final.
-- * `cons_regle_journal` : transitions en AJOUT SEUL, écrites dans la transaction du fait (R-RG40).

CREATE TABLE IF NOT EXISTS rgp_gouvernance (
    etablissement_id UUID PRIMARY KEY
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    -- Profil : fixe la proposition de visa (privé sous contrat traité comme le public, R-RG5).
    profil TEXT NOT NULL DEFAULT 'prive_sous_contrat'
        CHECK (profil IN ('public', 'prive_sous_contrat', 'prive_hors_contrat')),
    dpo_nom TEXT CHECK (dpo_nom IS NULL OR length(btrim(dpo_nom)) BETWEEN 1 AND 200),
    dpo_contact TEXT CHECK (dpo_contact IS NULL OR length(btrim(dpo_contact)) BETWEEN 3 AND 254),
    dpo_externe BOOLEAN,
    dpo_personne_id UUID,
    dpo_designe_le TIMESTAMPTZ,
    dpo_designe_par_compte_id UUID REFERENCES ent_user (id),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT rgp_gouvernance_dpo_coherent CHECK (
        (dpo_nom IS NULL AND dpo_contact IS NULL AND dpo_externe IS NULL
            AND dpo_personne_id IS NULL AND dpo_designe_le IS NULL)
        OR (dpo_nom IS NOT NULL AND dpo_contact IS NOT NULL AND dpo_externe IS NOT NULL
            AND dpo_designe_le IS NOT NULL)),
    CONSTRAINT rgp_gouvernance_dpo_personne_fk FOREIGN KEY (etablissement_id, dpo_personne_id)
        REFERENCES ent_personne (etablissement_id, id)
);

CREATE TABLE IF NOT EXISTS cons_regle (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    categorie_code TEXT NOT NULL CHECK (categorie_code ~ '^[a-z]{2,8}\.[a-z_]{2,40}$'),
    version INTEGER NOT NULL CHECK (version >= 1),
    utilite_mois INTEGER CHECK (utilite_mois IS NULL OR utilite_mois BETWEEN 0 AND 1200),
    intermediaire_mois INTEGER CHECK (intermediaire_mois IS NULL OR intermediaire_mois BETWEEN 0 AND 1200),
    sort_final TEXT NOT NULL CHECK (sort_final IN (
        'destruction', 'tri', 'conservation_definitive', 'effacement_dossier', 'suit_objet',
        'anonymisation_compte', 'suit_contrat', 'autre_socle')),
    critere_tri TEXT CHECK (critere_tri IS NULL OR length(btrim(critere_tri)) BETWEEN 1 AND 500),
    visa_exige BOOLEAN NOT NULL,
    motif_ajustement TEXT CHECK (motif_ajustement IS NULL OR length(btrim(motif_ajustement)) BETWEEN 3 AND 500),
    version_catalogue TEXT NOT NULL CHECK (length(version_catalogue) BETWEEN 1 AND 40),
    etat TEXT NOT NULL CHECK (etat IN ('proposee', 'validee', 'remplacee')),
    propose_par_compte_id UUID REFERENCES ent_user (id),
    propose_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    valide_par_compte_id UUID REFERENCES ent_user (id),
    valide_par_role TEXT CHECK (valide_par_role IS NULL OR length(valide_par_role) <= 60),
    valide_le TIMESTAMPTZ,
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT cons_regle_version_uq UNIQUE (etablissement_id, categorie_code, version),
    CONSTRAINT cons_regle_id_etab_uq UNIQUE (etablissement_id, id),
    -- Une règle validée dit qui l'a validée et quand ; une autre ne le dit pas.
    CONSTRAINT cons_regle_validation_coherente CHECK (
        (etat = 'proposee' AND valide_le IS NULL AND valide_par_compte_id IS NULL)
        OR (etat IN ('validee', 'remplacee') AND valide_le IS NOT NULL AND valide_par_compte_id IS NOT NULL)
        OR (etat = 'remplacee' AND valide_le IS NULL))
);
CREATE UNIQUE INDEX IF NOT EXISTS cons_regle_une_validee_uq
    ON cons_regle (etablissement_id, categorie_code) WHERE etat = 'validee';
CREATE UNIQUE INDEX IF NOT EXISTS cons_regle_une_proposee_uq
    ON cons_regle (etablissement_id, categorie_code) WHERE etat = 'proposee';
CREATE INDEX IF NOT EXISTS cons_regle_etab_idx ON cons_regle (etablissement_id, categorie_code, version DESC);

CREATE TABLE IF NOT EXISTS cons_regle_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    regle_id UUID NOT NULL,
    categorie_code TEXT NOT NULL CHECK (length(categorie_code) BETWEEN 3 AND 60),
    evenement TEXT NOT NULL CHECK (evenement IN (
        'proposee', 'ajustee', 'validee', 'remplacee', 'dpo_designe', 'dpo_retire')),
    par_compte_id UUID,
    par_role TEXT CHECK (par_role IS NULL OR length(par_role) <= 60),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    cree_le TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS cons_regle_journal_etab_idx ON cons_regle_journal (etablissement_id, le DESC);

-- Ajout seul : ni UPDATE, ni DELETE, ni TRUNCATE (même schéma que 091 et 2540).
CREATE OR REPLACE FUNCTION cons_regle_journal_ajout_seul() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'cons_regle_journal est en ajout seul (% interdit)', TG_OP
        USING ERRCODE = '42501';
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER cons_regle_journal_ajout_seul_trg
    BEFORE UPDATE OR DELETE ON cons_regle_journal
    FOR EACH ROW EXECUTE FUNCTION cons_regle_journal_ajout_seul();
CREATE OR REPLACE TRIGGER cons_regle_journal_sans_truncate_trg
    BEFORE TRUNCATE ON cons_regle_journal
    FOR EACH STATEMENT EXECUTE FUNCTION cons_regle_journal_ajout_seul();

SELECT ent_rls_poser(ARRAY['rgp_gouvernance', 'cons_regle', 'cons_regle_journal']);
