# Labo multi-personas — vie autonome

**Méthode (dictée Florian)**  

1. **Monter le module** d’abord (docs métier + REA / Charlemagne filtré) — domaine, tables, authz, seed cohérent.  
2. **Ensuite** balancer le lab vivant dessus : personas dans tous les sens → découvrir ce qui manque.  

Pas l’inverse. Les exemples oraux éclairent ; ils ne fixent pas le backlog.

Hors prod ScolIA. `ENT_DATABASE_URL` → `127.0.0.1` uniquement.

---

## Modules déjà montés puis stressés

| Module | Contenu « propre » avant lab |
|---|---|
| **Dossier élève** | Identité, scolarité datée, options, foyer/payeur 100 %, vue authzée, recherche |
| **Documents + dispositifs** | Tiroirs, confidentialité (restreint par défaut), grants datés, PAP lié au doc — circuit prof sans grant → avec grant |
| Absences / appel | Appel→`ent_absence`, soumission ≠ tranchage VS, motifs etab, seuil→signal |
| Notes / matières | Nomenclature admini, notes prof, authz parent |
| Auth / comptes | Sessions Rust, face élève liée (`ent_eleve_compte`) |

---

## Cast (`seed_labo_cast`)

École dense : 10 élèves/parents, 5 comptes face, 8 profs, 2 CPE / direction / admini / compta / infirmière / psy.

Mot de passe : `DevLocalPass1!`.

---

## Runner

```bash
cd ent-rust
set ENT_DATABASE_URL=postgres://scola:scola_dev_pwd@127.0.0.1:5432/scola
cargo run -p ent-lab       # exploration : gestes + TROU
cargo test -p ent-db       # scénarios rejouables (par défaut sans DB → skip)
cargo test --workspace     # tout, y compris API
```

Le rapport liste **gestes** + **TROU** (prochaine vague module). Le seed est
rejoué à chaque run (école jetable, nouveau `etablissement_id`).

---

## Vies croisées (élargissement du labo)

Au-delà de la « journée type » par module, le labo enchaîne des **vies** où une
écriture d'un persona se propage (ou pas) chez les autres — ce qui révèle les
vrais trous de bout en bout :

| Vie croisée | Ce qui est prouvé | Reste (décision produit) |
|---|---|---|
| **Changement de classe en cours d'année** (`explorer_vie_croisee_changement_classe`, Cedric 3C→5A) | Effectif daté (avant/après), EDT élève qui suit la classe, dossier staff reflète 5A, `scolarite.classe_changed` publié (Brain), journal `classe_change`, parent non lié → refusé | Tarif facturation par niveau à la bascule (prorata) — appels/EDT sont dérivés datés, donc suivent seuls |
| **Sortie élève** (`explorer_vie_croisee_sortie_eleve`, Neville) | Statut `sorti` + date, effectif daté (présent la veille, absent le jour de sortie), `eleve.sortie`, journal `eleve_sorti`, **compte face détaché + sessions révoquées** (prouvé aussi en HTTP : 401), **grilles repas closes**, **documents à signer annulés**, historique de périodes conservé | Prorata facturation/tarif au départ — EDT/appel dérivés, plus rien côté élève |

Ces deux vies sont aussi couvertes par des tests rejouables dans
`crates/ent-db/src/scenarios.rs` (`changement_classe_date_visible_partout`,
`sortie_eleve_revoque_compte_et_grille`).

Cycle de vie élève (sortie effective `date_sortie <= today`) : le service
`sortir` **détache** `ent_eleve_compte`, **révoque** `ent_session`
(`eleve.compte_revoque`), **clôt** les grilles repas la veille, **annule** les
docs à signer en attente, **ferme** les périodes ouvertes (historique daté
conservé) et **journalise** (`eleve_sorti`). Les bascules datées
(`classe`/`regime`/`mef`/`site`/`statut`, identité) alimentent aussi le journal
`ent_journal_modif` (`classe_change`, `statut_change`, `identite_modifiee`, …).

---

## Suite logique modules (pas encore montés en entier)

Documents + grants · repas · bourses/dispositifs · continuum N±1 · sanctions · cantine · messagerie · factu foyer · santé.
