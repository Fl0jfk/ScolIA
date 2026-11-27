# Lot « base modèle des tests » : préparé le 2026-11-16, validé par Florian

**État** : prêt à lancer à la prochaine place libre (règle 32 : 3 sessions qui codent au plus). Après le lot de ménage des avertissements, ou avant s'il se libère une place plus tôt.

## Pourquoi
- Sur ENT 68ec8bda (run 306), la tranche 1 des tests avec labo a duré 59 min 10 s pour 161 tests, et a été coupée par la limite de 60 minutes.
- La plupart des tests de base prennent 25 à 60 s, alors que le test lui-même ne dure que quelques millisecondes.
- Cause constatée dans le code : chaque test passe par `labo_selon_mode` (`crates/ent-db/src/tests_rls_stricte.rs`). Il relance `migrate()` (plus de 350 migrations à vérifier), puis `seed_labo_cast()`, qui recrée un établissement de démonstration complet.
- **On ne sait pas encore lequel des deux coûte le plus.** Le lot commence donc par mesurer.
- Le coût grossit à chaque lot, même pour les anciens tests, puisque les migrations s'accumulent.

## Mission de la session (Sonnet), à copier dans la consigne de lancement
1. **Mesurer**, sans rien changer : sur 10 tests représentatifs (ent-db et ent-api), chronométrer séparément `connect`, `migrate()`, `seed_labo_cast()` et le corps du test, en mode normal et en mode RLS strict. Consigner les chiffres dans le rapport.
2. **Base modèle PostgreSQL**, avec `CREATE DATABASE <test> TEMPLATE <modele>` (fonction native et gratuite) :
   - une base modèle par tranche ou par processus, migrée une seule fois ;
   - si la mesure le justifie, une base modèle qui contient aussi le jeu de démonstration, à condition que les tests restent indépendants : chacun garde sa propre copie, ou des identifiants aléatoires comme aujourd'hui ;
   - précautions : aucune connexion ouverte sur la base modèle pendant la copie ; un verrou pour qu'un seul processus la prépare (nextest lance un processus par test) ; nettoyage des copies ;
   - si le schéma change (nouvelle migration), la base modèle est reconstruite, avec une empreinte des migrations dans son nom ou dans une table.
3. **Exceptions** : les tests qui éprouvent les migrations elles-mêmes (`migrate::`, `tests_d036`, rejouabilité, MIGRATION_TAG) partent toujours d'une base vide.
4. **Ne rien affaiblir** :
   - aucun test retiré ni `#[ignore]` ;
   - mode RLS strict inchangé (rôle non propriétaire, NOBYPASSRLS) ;
   - garde `url_guard` (base locale uniquement) conservée.
   - Le nombre de tests exécutés doit rester identique, contrôlé par l'inventaire de la CI.
5. **CI** : adapter `.github/workflows/ent-rust.yml` si nécessaire (règle 27), sans changer le découpage en tranches. Rapporter le temps d'une tranche avant et après.
6. **Zéro nouvel avertissement** (règle 31). Une seule CI en fin de lot, après fusion d'`origin/ENT` (règle 28).
7. **Rapport au pilote** : SHA, numéro de run, mesures avant et après (par test et par tranche), limites.

## Gain attendu (hypothèse)
Temps des tests de base divisé par 5 à 10 si `migrate()` et `seed_labo_cast()` dominent, ce que la mesure de l'étape 1 dira. Ce sont des CI nettement plus courtes, et une marge durable pour les lots à venir.
