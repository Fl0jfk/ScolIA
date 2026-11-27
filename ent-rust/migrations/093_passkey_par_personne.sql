-- Socle 1 : la passkey appartient à la personne (D-013), plus à l'établissement.
-- Toutes les lectures se font désormais par `user_id` seul ; `etablissement_id`
-- ne garde que la valeur informative « établissement depuis lequel elle a été
-- enrôlée ». Additif : aucune donnée modifiée.
CREATE INDEX IF NOT EXISTS ent_passkey_credential_user_seul_idx
    ON ent_passkey_credential (user_id);

COMMENT ON COLUMN ent_passkey_credential.etablissement_id IS
    'Établissement depuis lequel la passkey a été enrôlée (information seulement, D-013).';
