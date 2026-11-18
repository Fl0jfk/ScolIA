-- Socle 21, lot V1 (fin) — 469 : première exécution de la reprise M2 pour les établissements qui ont déjà
-- des évaluations. Rejouable : un établissement qui a déjà une exécution n'est pas repris deux fois
-- (le rapport ne se répète pas à chaque démarrage). Appliquée en dernier : elle lit toutes les tables
-- de 460 à 468 et les anciennes tables.
SELECT eva_reprendre_tout();
