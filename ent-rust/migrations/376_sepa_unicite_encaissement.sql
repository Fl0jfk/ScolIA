-- RS3-02 : garde-fou en base contre le double encaissement SEPA.
-- Un prélèvement ne peut référencer qu'un encaissement, et un identifiant de
-- bout en bout (référence) ne peut donner lieu qu'à un seul encaissement par
-- prélèvement dans un établissement.
CREATE UNIQUE INDEX IF NOT EXISTS ent_fac_sepa_prelevement_encaissement_uq
    ON ent_fac_sepa_prelevement (encaissement_id)
    WHERE encaissement_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS ent_fac_encaissement_prelevement_ref_uq
    ON ent_fac_encaissement (etablissement_id, reference)
    WHERE moyen = 'prelevement' AND reference IS NOT NULL;
