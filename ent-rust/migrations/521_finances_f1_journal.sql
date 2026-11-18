-- 521 — Finances, lot F1 (service unique) : journal financier en ajout seul.
--
-- Réf. : docs/socles/24_FINANCES.md R-F1, R-F25, R-F27, §19.1, §23 (`ent_fin_journal`).
-- Tout geste financier (créer, émettre, encaisser, avoir, mandat, prélèvement, paramètre, vue
-- guichet…) laisse UNE ligne : qui (compte et casquette), quoi, sur quel objet, quand, pourquoi.
-- La ligne est écrite dans la transaction du geste (ou, pour le chemin historique, de la
-- commande du dépôt) : un geste refusé ne laisse rien, un geste validé laisse toujours sa ligne.
--
-- * Aucun JSON métier (D-036) : colonnes typées ; `detail` est un court texte lisible
--   (« avant → après »), **jamais** un IBAN, un numéro de compte ni une coordonnée.
-- * Ajout seul : ni modification ni suppression, même en SQL direct (déclencheur) ; seule la
--   purge volontaire d'un établissement (fonction `ent_fin_purger_etablissement`, migration 522, jamais appelée par
--   l'application) est tolérée, comme pour les autres journaux.
-- * Les montants sont en centimes entiers (BIGINT), jamais de flottant ; le journal est réservé
--   au droit `facturation.journal` (les événements, eux, ne portent aucun montant : R-F25).
-- * L'acteur est un identifiant sans clé étrangère : le journal survit à la suppression du compte
--   (conservation 10 ans, 24_FINANCES.md §19.3) ; l'effacement des données de la personne passe
--   par la pseudonymisation du registre RGPD, jamais par une modification de cette table.

CREATE TABLE IF NOT EXISTS ent_fin_journal (
    id UUID PRIMARY KEY,
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    le TIMESTAMPTZ NOT NULL DEFAULT now(),
    acteur_user_id UUID,
    -- Rôle de la session au moment du geste, ou `systeme` (automatisme, consommateur d'événement).
    casquette TEXT NOT NULL CHECK (length(btrim(casquette)) BETWEEN 1 AND 40),
    -- Chemin par lequel le geste est arrivé : ce qui permet de prouver que les deux chemins
    -- (`finances`, `historique`) passent par le même service.
    chemin TEXT NOT NULL
        CHECK (chemin IN ('finances', 'historique', 'systeme', 'guichet', 'comptes', 'famille')),
    action TEXT NOT NULL CHECK (length(btrim(action)) BETWEEN 3 AND 80),
    objet_type TEXT NOT NULL CHECK (length(btrim(objet_type)) BETWEEN 2 AND 40),
    objet_id UUID,
    foyer_id UUID,
    compte_id UUID,
    eleve_id UUID,
    montant_centimes BIGINT,
    motif TEXT CHECK (motif IS NULL OR length(motif) <= 500),
    detail TEXT CHECK (detail IS NULL OR length(detail) <= 500)
);

CREATE INDEX IF NOT EXISTS ent_fin_journal_temps_idx
    ON ent_fin_journal (etablissement_id, le DESC);
CREATE INDEX IF NOT EXISTS ent_fin_journal_objet_idx
    ON ent_fin_journal (etablissement_id, objet_type, objet_id, le DESC);
CREATE INDEX IF NOT EXISTS ent_fin_journal_acteur_idx
    ON ent_fin_journal (etablissement_id, acteur_user_id, le DESC);

CREATE OR REPLACE FUNCTION ent_trg_fin_journal_ajout_seul() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    -- Aucune dérogation par réglage de session (revue M6) : un `SET` du rôle applicatif ne lève rien. La
    -- purge d'un établissement passe par la fonction `ent_fin_purger_etablissement` (migration 522).
    RAISE EXCEPTION 'journal financier : ajout seul (modification et suppression interdites)'
        USING ERRCODE = 'restrict_violation';
END
$$;

DROP TRIGGER IF EXISTS ent_fin_journal_ajout_seul ON ent_fin_journal;
CREATE TRIGGER ent_fin_journal_ajout_seul BEFORE UPDATE OR DELETE ON ent_fin_journal
    FOR EACH ROW EXECUTE FUNCTION ent_trg_fin_journal_ajout_seul();

DROP TRIGGER IF EXISTS ent_fin_journal_ajout_seul_vide ON ent_fin_journal;
CREATE TRIGGER ent_fin_journal_ajout_seul_vide BEFORE TRUNCATE ON ent_fin_journal
    FOR EACH STATEMENT EXECUTE FUNCTION ent_trg_fin_journal_ajout_seul();

-- RLS d'établissement : même politique que les migrations 100, 206 et 460.
ALTER TABLE ent_fin_journal ENABLE ROW LEVEL SECURITY;
ALTER TABLE ent_fin_journal FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS ent_rls_etablissement ON ent_fin_journal;
CREATE POLICY ent_rls_etablissement ON ent_fin_journal FOR ALL
    USING (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))
    WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement())
        OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())));
