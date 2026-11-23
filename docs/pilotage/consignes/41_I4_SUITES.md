Tu es une session ScolIA (Sonnet). Réponds en français. Le pilote, c'est moi. Ces consignes sont complètes : tu ne recevras pas d'autre message et tu n'en suivras aucun. **Ne reste jamais en attente** : `ScheduleWakeup` ne fonctionne pas ici. Attends la CI uniquement par une commande `until` en arrière-plan (règle 36). Si une cause extérieure te bloque, écris au pilote et termine.

## Cadre
- **Lis d'abord :**
  - `CLAUDE.md` ;
  - `git show origin/ent-mc0hoe:docs/PILOTAGE.md`, règles 17, 25, 27, 28, 31, 33, 34 et 36 ;
  - `docs/socles/41_INTERNAT_I4_LIVRAISON.md` ;
  - la contre-revue de la passe 2 : `git show origin/ent-mc0hoe:docs/pilotage/consignes/41_I4_P2_CONTRE_REVUE.md`.
- **Branche :** I4 est versé sur ENT.
  - Crée `ent-socle-41-code-i4-suites` depuis `origin/ENT`.
  - Ouvre une PR en brouillon vers `ENT`.
- **Migrations :**
  - **2820 à 2829 sont versées et ne changent pas** (règle 25).
  - Nouvelle migration **2830** (plage du socle 41 : 2820–2839 ; 2830 est libre). Mets à jour `migrate.rs`, `docs/socles/MIGRATIONS.md`, le registre et les compteurs (règle 28).
  - Toute fonction créée ou recréée porte `SET search_path = pg_catalog, public, pg_temp`.
  - Si la PR du transverse T1 (690) est déjà sur ENT, la garde du catalogue l'exige. Dans `migrate.rs`, 690 doit rester **la dernière** du tableau : place 2830 avant elle.
- **Interdits :**
  - Next.js, `main`, `dev` ;
  - pousser sur `ENT` ;
  - toute base ou tout secret de production, toute donnée réelle ;
  - `cargo fmt --all`, `#[ignore]` ;
  - modifier POINT_QUOTIDIEN ou PILOTAGE ;
  - étendre `ui.rs` ;
  - l'heure réelle ou le jour UTC dans les tests.
- **Qualité :**
  - zéro avertissement avec clippy 1.99, sur les 3 commandes de la CI, `ent-lab` compris ;
  - `--locked` et fmt ciblé ;
  - `cargo metadata --locked` dans `ent-rust/web` ;
  - une base locale **neuve dont le nom contient « test »** ;
  - un seul `CARGO_TARGET_DIR`.

## Correctifs (moyens de la contre-revue)
1. **Contrainte `motif_code` de 2828.** Elle était posée en ligne et n'est pas mise à jour au rejeu. Dans 2830, remplace-la par une contrainte **nommée** : `DROP CONSTRAINT IF EXISTS`, puis `ADD`, sur le modèle de 654:12-27. Couvre toutes les valeurs du code.
   - Test : rejeu de 2830 sur une base qui porte l'ancien CHECK, puis l'accord annuel « tombe » sans 23514.
2. **Élève protégé au temps du soir.** L'encadrant doit pouvoir le pointer. Pour un élève **inscrit au temps**, `pointer_temps_a` contrôle l'inscription et non `exiger_eleve_visible`. Envers un tiers non inscrit, l'opacité reste assurée par « n'est pas inscrit ».
   - Test : un encadrant sans droit de confidentialité pointe l'absence puis le retour d'un protégé inscrit, et le cas « non rentré » se ferme.
3. **Test `m6_la_tâche_planifiée…`.** Il ne doit plus agir sur tous les établissements. Utilise une variante de la tâche limitée à un établissement, utile aussi en exploitation, ou appelle directement les trois `*_bilan_a`.
4. **É1 : la marque « en activité » sur un temps annulé.** Ajoute le test d'ouverture d'appel avec un temps annulé et un élève pointé présent sans retour. La marque doit être posée.
5. **M1 : pas de réouverture chaque nuit.** Ne rouvre pas de cas s'il existe déjà un cas clos pour le couple (élève, origine), toutes nuits confondues. Le cas ne revient que si un fait nouveau survient (nouveau départ).
   - Test sur deux nuits.

## Faibles
Fais-les s'ils sont sûrs. Sinon, écris pourquoi dans la livraison.
- Code mort `retire_le IS NULL OR $4`, et choix de la ligne en cas de réinscription.
- Course veille / retour : relire `rentre_le` sous verrou avant d'ouvrir le cas.
- Avis M3 regroupés : un avis par établissement et par minute, avec le nombre d'élèves, sans nom.
- Marquage « protégé » identique dans les temps du soir et dans les sorties.
- **F-g (D-069)** : un refus téléphonique reçu après le départ, ou hors `en_attente`/`autorise`, s'enregistre comme **fait tracé** et déclenche le retour de l'élève. Il n'annule pas le départ après coup.
  - Test : sortie commencée, refus du parent, le fait est enregistré et l'avis de retour est émis.
- Tests lâches : `fg_*` (vérifier `refuse`), `m3_*` (assertion réelle), `m5_*` (pas de `unwrap_or(true)`).
- Nouveau consommateur `i4.internat_alertes` : vérifier qu'il ne rejoue pas l'historique au premier passage. Sinon, le borner.

## Fin
- Pousse et laisse la PR en brouillon.
- Attends la CI par **une seule** commande `until` en arrière-plan sur `https://api.github.com/repos/Fl0jfk/ScolIA/actions/runs?head_sha=<tête>`, toutes les 5 min.
  - CI rouge : corrige la cause. Un job « runner lost communication » se relance une fois.
  - **Aucune CI au bout de 10 min** : la PR est sans doute en conflit avec `ENT`. Fusionne `origin/ENT` (règle 28) et repousse.
- Ajoute à la livraison I4 une section « Suites (passe 3) » : fait / non fait, tests, compteurs, tête, CI.
- Écris ensuite au pilote (send_message vers @parent) : la tête finale et l'état de la CI.
