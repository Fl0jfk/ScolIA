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
cargo run -p ent-lab
cargo test -p ent-db -- scenarios
```

Le rapport liste **gestes** + **TROU** (prochaine vague module).

---

## Suite logique modules (pas encore montés en entier)

Documents + grants · repas · bourses/dispositifs · continuum N±1 · sanctions · cantine · messagerie · factu foyer · santé.
