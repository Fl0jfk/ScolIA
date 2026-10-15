# Vision ENT — Rust partout, socle élève, Brain

> Document de référence produit / technique.  
> Complète `docs/ent-metier-checklist.md` (couverture métier Charlemagne → ENT).

---

## 1. Ce qu’on construit

Un **ENT intelligent** à nous — pas Charlemagne Host, pas un clone UI, pas un import 1:1.

| Couche | Choix |
|---|---|
| Front web ENT | **Rust — Leptos** (SSR/CSR) |
| Apps natives familles / élèves / profs | **SwiftUI** + **Kotlin / Compose** (déjà décidé — pas Expo, pas Rust mobile) |
| API HTTP | **Rust — axum** + **tokio** + **tower** |
| Domaine / Brain | **crates Rust** (`domain-*`, `ent-brain`, `ent-events`) |
| Accès SQL | **sqlx** (Postgres, typé, pas d’ORM lourd) |
| BDD | **PostgreSQL** (France / Scaleway) — multi-tenant `etablissement_id` |
| Auth | **100 % Rust dès maintenant** — Argon2id + sessions Postgres (`ent_user` / `ent_membership` / `ent_session`). Pas Better-Auth, pas NextAuth, pas de pont temporaire. |
| Secrets | `process.env` / variables d’env uniquement |

ScolIA (Next.js) = labo / immigration / référence — **n’est pas** l’auth du vrai ENT.  
Le chantier `ent-rust` construit le **produit final** (hors prod ScolIA actuelle) : auth et métier prêts pour la prod ENT quand on basculera.

---

## 2. Différence vs Charlemagne (et vs un ENT classique)

Charlemagne = modules en silos (Admin, VS, Notes…) qui partagent une base, mais la logique reste **éparpillée**.

Notre ENT :

### Modèle de données — règle pour **tout** l’ENT

Ce n’est **pas** une règle « absences ». C’est la façon de construire **toutes** les données réutilisables.

1. **Ce qui doit être partagé vit au centre**  
   Élève, absences, classes, notes, régimes, motifs… → tables / faits **canoniques**.  
   Pas une copie par module.

2. **Les modules piochent le centre**  
   Admin, cantine, notes, séjour, un module ajouté dans 2 ans… → **lisent / écrivent** les tables centrales (selon droits).  
   On ne leur demande pas de « communiquer entre eux » pour resynchroniser la même info.

3. **Élève au-dessus de tout**  
   L’élève n’est pas un sous-objet de l’administratif. Les faces viennent chercher dans le noyau élève (+ faits liés à `eleve_id`).

4. **Une écriture, partout visible**  
   Un geste métier écrit **une fois** au bon endroit. Le reste lit.  
   *Exemple (un parmi d’autres) :* séjour → ligne dans `ent_absence` (motif `sejour`, période) — pas `absence_sejour`. Cantine et écran absences lisent la même table.

5. **Brain** — orchestre le geste ; ne crée pas de miroirs d’état déjà écrits.

```
     tables / faits CENTRAUX (réutilisables)
              ▲    ▲    ▲
              │    │    │
         module  module  module …  (faces + droits)
```

Amorce déjà dans ScolIA (TypeScript) : table `metier_event`, pack clôture appel, `app/lib/brain-ai/`.  
À **porter / reconcevoir en Rust** dans le vrai ENT — pas seulement coller une couche chat.

---

## 3. Contraintes d’exécution

- **Ne pas taper la prod** (Scaleway RDB / conteneur prod) pour développer ce chantier.
- Travail sur branche dédiée + Postgres **local** (`127.0.0.1`) / labo.
- `main` = production protégée ; intégration via `dev` après validation humaine.

---

## 4. Stratégie de branches (conseil)

| Option | Quand |
|---|---|
| **Nouvelle branche** `feature/ent-rust` depuis `dev` | **Recommandé** pour le vrai ENT Rust (vertfield contrôlé) |
| Reprendre un WIP Rust ancien | Seulement s’il existe encore du `Cargo.toml` / crates utiles |

État constaté (oct 2026) :

- Pas de `Cargo.toml` dans le dépôt actuel après pull `dev`.
- Amorce « cerveau » côté TS : `metier_event`, `brain-ai`, `eleve-core`.
- Branche liée événements : `cursor/metier-event-pack-a-91ad` (TS / VS) — **inspiration**, pas base du vrai ENT Rust.

→ On part sur **`feature/ent-rust`** : doc + squelette crates, sans mélanger avec les lots métier Next.js du quotidien.

---

## 5. Ordre de construction (rappel)

1. Socle élève + événements métier (`metier_event` Rust)  
2. Brain (bus / handlers de répercussion) — IA branchée ensuite  
3. Domaines P0 : VS (appel/absences) + notes cœur  
4. API Rust + front Rust (intranet puis faces familles/élèves/profs)  
5. Inscriptions, factu, cantine… selon checklist keep/drop  

Charlemagne = **checklist de couverture**, pas schéma ni règles à recopier.

---

## 6. Qualité / tests / livrables

Voir **[`ent-rust-quality.md`](./ent-rust-quality.md)** : pyramide L0→L4, DoD par verticale, commandes `cargo test`, hors prod.

Amorce code : dossier **`ent-rust/`** (crates `ent-events`, `domain-eleve`).

---

## 7. Non-objectifs

- Reverse WinDev pour extraire toutes les formules  
- Parité écran par écran Charlemagne  
- Paie / compta OGEC dans le cœur ENT (sauf décision produit contraire)  
- Déploiements / migrations sur la RDB prod sans validation humaine explicite  
