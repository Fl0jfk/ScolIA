-- Messagerie, lot M0b (`docs/socles/16_MESSAGERIE.md` §27.1 et §33.2, `PERSONAS_16.md`) : sécurité
-- résiduelle (D21 à D28) et fiabilité (D8, D11 à D18, D20). Plage de repli 480–499 (Q-B2 non
-- tranchée) ; 480 = M0a. Aucune donnée en JSON (D-036) : colonnes typées uniquement. Aucun nouveau
-- contenu en BYTEA (PRA E5) : les pièces historiques restent en base jusqu'à la reprise par la
-- session de stockage objet.
--
-- 1. Fils : `foyer_lecteur_id` (D22) ; index de pagination de la boîte (D12).
-- 2. Messages : remise différée par la plage calme et dérogation tracée (D24) ; longueurs (D17).
-- 3. `ent_msg_reglage` : réglage typé de la plage calme, une ligne par établissement (D24).
-- 4. Liens créés après une interdiction par libellé : à vérifier avant d'être communicants (D25).
-- 5. Annonces : aperçu chiffré, confirmation, retrait, ciblage par `classe_id` (D8, D18).
-- 6. Envois S2-8 : statut « annulé » (retrait d'une annonce).
-- 7. RLS permissive des nouvelles tables (même politique que 480).

-- 1. Fils ----------------------------------------------------------------------------------------
-- Un fil ouvert par un personnel « aux responsables » est créé une fois par foyer de liens
-- communicants ; `foyer_lecteur_id` désigne ce foyer lecteur. Distinct de `foyer_id`, qui signifie
-- « payeurs du foyer » (relance de facture). Pas de clé étrangère : un foyer supprimé laisse un
-- identifiant orphelin que personne ne lit (échec fermé), au lieu de rouvrir le fil à tous.
ALTER TABLE ent_msg_fil ADD COLUMN IF NOT EXISTS foyer_lecteur_id UUID;

-- Reprise : un fil du personnel (ni privé, ni de foyer, ni de vie scolaire) d'un élève dont tous
-- les liens communicants sont dans un seul foyer est rattaché à ce foyer. Les fils d'un élève à
-- plusieurs foyers ne peuvent pas être départagés automatiquement : ils restent à `NULL` (point
-- signalé à la direction dans `PROJECT_STATUS.md`).
UPDATE ent_msg_fil f
   SET foyer_lecteur_id = u.foyer_id
  FROM (SELECT l.etablissement_id, l.eleve_id, min(l.foyer_id::text)::uuid AS foyer_id
          FROM ent_eleve_lien_communicant l
         GROUP BY l.etablissement_id, l.eleve_id
        HAVING count(DISTINCT COALESCE(l.foyer_id::text, l.id::text)) = 1
           AND count(l.foyer_id) = count(*)) u
 WHERE f.etablissement_id = u.etablissement_id AND f.eleve_id = u.eleve_id
   AND f.prive_user_id IS NULL AND f.foyer_id IS NULL AND NOT f.vie_scolaire
   AND f.audience = 'responsables' AND f.foyer_lecteur_id IS NULL;

CREATE INDEX IF NOT EXISTS ent_msg_fil_pagination_idx
    ON ent_msg_fil (etablissement_id, last_message_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS ent_msg_fil_foyer_lecteur_idx
    ON ent_msg_fil (etablissement_id, foyer_lecteur_id) WHERE foyer_lecteur_id IS NOT NULL;

-- 2. Messages ------------------------------------------------------------------------------------
-- `remise_prevue_le` : NULL = remis à la création. Dans le futur : message du personnel écrit
-- pendant la plage calme, invisible de tout autre que son auteur jusqu'à cet instant.
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS remise_prevue_le TIMESTAMPTZ;
-- Dérogation de la direction (`messagerie.urgence_hors_plage`) : motif et auteur tracés.
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS derogation_motif TEXT;
ALTER TABLE ent_msg ADD COLUMN IF NOT EXISTS derogation_par UUID;

CREATE INDEX IF NOT EXISTS ent_msg_remise_idx
    ON ent_msg (etablissement_id, fil_id, created_at) WHERE remise_prevue_le IS NOT NULL;

-- Longueurs maximales (R-M44) : sujet 200, message 20 000. `NOT VALID` : les lignes existantes
-- ne sont pas contrôlées, toute nouvelle écriture l'est.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_fil_sujet_long_chk') THEN
        ALTER TABLE ent_msg_fil ADD CONSTRAINT ent_msg_fil_sujet_long_chk
            CHECK (char_length(sujet) <= 200) NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_corps_long_chk') THEN
        ALTER TABLE ent_msg ADD CONSTRAINT ent_msg_corps_long_chk
            CHECK (char_length(corps) <= 20000) NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_msg_derogation_chk') THEN
        ALTER TABLE ent_msg ADD CONSTRAINT ent_msg_derogation_chk
            CHECK ((derogation_motif IS NULL) = (derogation_par IS NULL)
                   AND (derogation_motif IS NULL OR char_length(btrim(derogation_motif)) BETWEEN 1 AND 300));
    END IF;
END $$;

-- 3. Plage calme --------------------------------------------------------------------------------
-- Défaut national (référentiel du numérique responsable, exigences 8 et 9) : aucune nouvelle
-- information de 20 h à 7 h et du vendredi 20 h au lundi 7 h. Une ligne absente = défaut actif.
-- Désactiver ou réduire exige un motif (MSG-262), tracé avec l'auteur.
CREATE TABLE IF NOT EXISTS ent_msg_reglage (
    etablissement_id UUID PRIMARY KEY
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    plage_calme BOOLEAN NOT NULL DEFAULT true,
    plage_debut_heure SMALLINT NOT NULL DEFAULT 20 CHECK (plage_debut_heure BETWEEN 12 AND 23),
    plage_fin_heure SMALLINT NOT NULL DEFAULT 7 CHECK (plage_fin_heure BETWEEN 0 AND 11),
    plage_weekend BOOLEAN NOT NULL DEFAULT true,
    motif TEXT CHECK (motif IS NULL OR char_length(btrim(motif)) BETWEEN 1 AND 300),
    modifie_par UUID,
    modifie_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Une plage réduite (désactivée, plus courte, sans week-end) porte son motif.
    CONSTRAINT ent_msg_reglage_motif_chk CHECK (
        (plage_calme AND plage_debut_heure <= 20 AND plage_fin_heure >= 7 AND plage_weekend)
        OR motif IS NOT NULL)
);

-- Prochain instant de remise d'un message écrit à `instant`, ou NULL s'il peut être remis tout de
-- suite. Fuseau de l'établissement (265). Plage 12 h–23 h → 0 h–11 h, passant minuit.
CREATE OR REPLACE FUNCTION ent_msg_remise_calme(etab UUID, instant TIMESTAMPTZ DEFAULT now())
RETURNS TIMESTAMPTZ
LANGUAGE plpgsql STABLE
AS $$
DECLARE
    r ent_msg_reglage%ROWTYPE;
    tz TEXT;
    loc TIMESTAMP;
    heure INTEGER;
    jour INTEGER;
    cand DATE;
    fin_t TIME;
BEGIN
    SELECT * INTO r FROM ent_msg_reglage WHERE etablissement_id = etab;
    IF NOT FOUND THEN
        r.plage_calme := true; r.plage_debut_heure := 20; r.plage_fin_heure := 7;
        r.plage_weekend := true;
    END IF;
    IF NOT r.plage_calme THEN
        RETURN NULL;
    END IF;
    tz := COALESCE((SELECT p.fuseau FROM ent_etablissement_parametre p
                     WHERE p.etablissement_id = etab), 'Europe/Paris');
    loc := instant AT TIME ZONE tz;
    heure := extract(hour FROM loc)::int;
    jour := extract(isodow FROM loc)::int;     -- 1 = lundi … 7 = dimanche
    IF NOT (heure >= r.plage_debut_heure OR heure < r.plage_fin_heure
            OR (r.plage_weekend AND jour IN (6, 7))) THEN
        RETURN NULL;
    END IF;
    fin_t := make_time(r.plage_fin_heure, 0, 0);
    cand := loc::date + CASE WHEN loc::time >= fin_t THEN 1 ELSE 0 END;
    IF r.plage_weekend THEN
        WHILE extract(isodow FROM cand)::int IN (6, 7) LOOP
            cand := cand + 1;
        END LOOP;
    END IF;
    RETURN (cand + fin_t) AT TIME ZONE tz;
END
$$;

-- 4. Liens à vérifier (D25) -----------------------------------------------------------------------
-- Une interdiction de communication saisie par libellé (personne absente de la fiche) ne vise
-- aucun lien (480) : la personne, liée ensuite sous une autre identité, communiquait. Tout lien
-- créé après une telle interdiction est inactif dans la messagerie jusqu'à décision de la
-- direction (`confirme`) ; `maintenu_inactif` consigne un refus explicite.
CREATE TABLE IF NOT EXISTS ent_msg_lien_decision (
    etablissement_id UUID NOT NULL
        CHECK (etablissement_id <> '00000000-0000-0000-0000-000000000000'::uuid),
    lien_id UUID NOT NULL REFERENCES ent_eleve_lien (id) ON DELETE CASCADE,
    interdiction_id UUID NOT NULL REFERENCES ent_eleve_interdiction (id) ON DELETE CASCADE,
    decision TEXT NOT NULL CHECK (decision IN ('confirme', 'maintenu_inactif')),
    motif TEXT CHECK (motif IS NULL OR char_length(btrim(motif)) BETWEEN 1 AND 300),
    decide_par UUID NOT NULL,
    decide_le TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (lien_id, interdiction_id)
);
CREATE INDEX IF NOT EXISTS ent_msg_lien_decision_etab_idx
    ON ent_msg_lien_decision (etablissement_id, lien_id);

CREATE OR REPLACE VIEW ent_eleve_lien_communicant AS
SELECT l.*
FROM ent_eleve_lien l
JOIN ent_personne p
  ON p.id = l.personne_id AND p.etablissement_id = l.etablissement_id
WHERE l.suspendu_le IS NULL
  AND (l.date_debut IS NULL OR l.date_debut <= ent_aujourdhui_etab(l.etablissement_id))
  AND (l.date_fin IS NULL OR l.date_fin >= ent_aujourdhui_etab(l.etablissement_id))
  AND p.fusionnee_dans IS NULL
  AND NOT EXISTS (
    SELECT 1
      FROM ent_eleve_interdiction i
     WHERE i.etablissement_id = l.etablissement_id
       AND i.eleve_id = l.eleve_id
       AND i.levee_le IS NULL
       AND 'communication' = ANY (i.objet)
       AND i.du <= ent_aujourdhui_etab(l.etablissement_id)
       AND (i.au IS NULL OR i.au >= ent_aujourdhui_etab(l.etablissement_id))
       AND ((i.personne_id IS NOT NULL
             AND (i.personne_id = l.personne_id
                  OR i.personne_id IN (SELECT s.id FROM ent_personne s
                                        WHERE s.etablissement_id = l.etablissement_id
                                          AND s.fusionnee_dans = l.personne_id)))
            -- D25 : interdiction par libellé, lien créé après elle et pas confirmé.
            OR (i.personne_id IS NULL
                AND l.created_at >= i.created_at
                AND NOT EXISTS (SELECT 1 FROM ent_msg_lien_decision v
                                 WHERE v.lien_id = l.id AND v.interdiction_id = i.id
                                   AND v.decision = 'confirme'))));

-- 5. Annonces ------------------------------------------------------------------------------------
-- `apercu` : brouillon chiffré, rien n'est parti ; `publiee` : confirmée par l'auteur (ou
-- ancienne annonce) ; `retiree` : retirée, envois en attente annulés, plus visible des familles.
ALTER TABLE ent_annonce ADD COLUMN IF NOT EXISTS statut TEXT NOT NULL DEFAULT 'publiee';
ALTER TABLE ent_annonce ADD COLUMN IF NOT EXISTS classe_id UUID REFERENCES ent_classe (id);
ALTER TABLE ent_annonce ADD COLUMN IF NOT EXISTS apercu_nb_eleves INTEGER;
ALTER TABLE ent_annonce ADD COLUMN IF NOT EXISTS apercu_nb_responsables INTEGER;
ALTER TABLE ent_annonce ADD COLUMN IF NOT EXISTS confirmee_par UUID;
ALTER TABLE ent_annonce ADD COLUMN IF NOT EXISTS confirmee_le TIMESTAMPTZ;
ALTER TABLE ent_annonce ADD COLUMN IF NOT EXISTS retiree_par UUID;
ALTER TABLE ent_annonce ADD COLUMN IF NOT EXISTS retiree_le TIMESTAMPTZ;
ALTER TABLE ent_annonce ADD COLUMN IF NOT EXISTS retiree_motif TEXT;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_annonce_statut_chk') THEN
        ALTER TABLE ent_annonce ADD CONSTRAINT ent_annonce_statut_chk
            CHECK (statut IN ('apercu', 'publiee', 'retiree'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_annonce_retrait_chk') THEN
        ALTER TABLE ent_annonce ADD CONSTRAINT ent_annonce_retrait_chk
            CHECK ((statut = 'retiree') = (retiree_le IS NOT NULL)
                   AND (retiree_le IS NULL OR (retiree_par IS NOT NULL AND retiree_motif IS NOT NULL
                        AND char_length(btrim(retiree_motif)) BETWEEN 1 AND 300)));
    END IF;
END $$;

-- Reprise D18 : la classe ciblée par libellé est rattachée à `ent_classe` quand le libellé (ou
-- le code) désigne une seule classe de l'établissement ; sinon `classe_id` reste NULL et
-- l'annonce garde son libellé (lue comme avant).
UPDATE ent_annonce a
   SET classe_id = c.id
  FROM (SELECT etablissement_id, lower(btrim(libelle)) AS cle, min(id::text)::uuid AS id
          FROM ent_classe GROUP BY etablissement_id, lower(btrim(libelle))
        HAVING count(*) = 1) c
 WHERE a.audience = 'classe' AND a.classe_id IS NULL
   AND c.etablissement_id = a.etablissement_id AND c.cle = lower(btrim(a.cible_classe));
UPDATE ent_annonce a
   SET classe_id = c.id
  FROM (SELECT etablissement_id, lower(btrim(code)) AS cle, min(id::text)::uuid AS id
          FROM ent_classe GROUP BY etablissement_id, lower(btrim(code))
        HAVING count(*) = 1) c
 WHERE a.audience = 'classe' AND a.classe_id IS NULL
   AND c.etablissement_id = a.etablissement_id AND c.cle = lower(btrim(a.cible_classe));

CREATE INDEX IF NOT EXISTS ent_annonce_classe_idx
    ON ent_annonce (etablissement_id, classe_id) WHERE classe_id IS NOT NULL;

-- 6. Envois S2-8 : un envoi en attente d'une annonce retirée est annulé, pas supprimé ------------
ALTER TABLE ent_notification_envoi DROP CONSTRAINT IF EXISTS ent_notif_envoi_statut_chk;
ALTER TABLE ent_notification_envoi ADD CONSTRAINT ent_notif_envoi_statut_chk
    CHECK (statut IN ('en_attente', 'envoi', 'envoye', 'simule', 'echec', 'ignore', 'plafonne',
                      'regroupe', 'annule'));

-- 7. RLS permissive (politique de 480) -----------------------------------------------------------
DO $$
DECLARE
    t text;
    tables text[] := ARRAY['ent_msg_reglage', 'ent_msg_lien_decision'];
BEGIN
    FOREACH t IN ARRAY tables LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('DROP POLICY IF EXISTS ent_rls_etablissement ON %I', t);
        EXECUTE format(
            'CREATE POLICY ent_rls_etablissement ON %I FOR ALL '
            'USING (etablissement_id = (SELECT ent_rls_etablissement()) '
            '  OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict()))) '
            'WITH CHECK (etablissement_id = (SELECT ent_rls_etablissement()) '
            '  OR ((SELECT ent_rls_etablissement()) IS NULL AND NOT (SELECT ent_rls_strict())))',
            t);
    END LOOP;
END
$$;
