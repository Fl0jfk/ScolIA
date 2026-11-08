-- Socle 4, lot A1 (§12.2, droit d'accès) : qui a déclaré un contact d'urgence. Un titulaire ne
-- voit que les contacts qu'il a déclarés lui-même ou ceux saisis par l'établissement (déclarant
-- inconnu : reprise de l'existant, saisie par le secrétariat). Migration additive et rejouable.

ALTER TABLE ent_eleve_contact_urgence
    ADD COLUMN IF NOT EXISTS declare_par_personne_id UUID;
CREATE INDEX IF NOT EXISTS ent_eleve_contact_urgence_declarant_idx
    ON ent_eleve_contact_urgence (etablissement_id, declare_par_personne_id)
    WHERE declare_par_personne_id IS NOT NULL;
