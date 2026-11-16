-- 448 — Socle 10 (emploi du temps), lot T5a : matérialisation des séances depuis les placements
-- (docs/socles/10_EMPLOI_DU_TEMPS.md §8.1, §22 M3).
--
-- Jusqu'ici les séances venaient des créneaux historiques (`ent_edt_creneau`). Elles viennent désormais des
-- PLACEMENTS de la version en vigueur à chaque date (base, surchargée par les versions temporaires). Les lecteurs
-- historiques (appel, cahier, notes, absences) s'attachent encore à `ent_edt_seance.creneau_id` (NOT NULL) : un
-- placement qui n'a pas de créneau historique reçoit donc un créneau **projeté** (« pont », comme le journal de T3
-- projette vers les exceptions) ; la projection est retirée au lot T11, avec la bascule des lecteurs.
-- * `projete` distingue ces créneaux : la reprise (M2) les ignore (un créneau projeté n'est pas un fait historique) ;
-- * un créneau projeté est RÉUTILISÉ par la version suivante quand le placement est identique (même cours, même jour,
--   même plage, même semaine type) : la séance garde son identifiant, donc l'appel, le cahier et le travail à faire.
-- Additive, rejouable (E8). Aucune donnée supprimée.

ALTER TABLE ent_edt_creneau ADD COLUMN IF NOT EXISTS projete BOOLEAN NOT NULL DEFAULT false;
CREATE INDEX IF NOT EXISTS ent_edt_creneau_projete_idx ON ent_edt_creneau (etablissement_id) WHERE projete;
