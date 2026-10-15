# ENT Rust — tests, vérifs, définition de « livré »

Branche de travail : `feature/ent-rust`.  
**Jamais** de validation contre la RDB / conteneur **prod**.

---

## 1. Pyramide de confiance

| Niveau | Quoi | Quand | Où |
|---|---|---|---|
| **L0 — Unitaire domaine** | Invariants, commandes, événements émis (sans I/O) | Chaque PR / chaque `cargo test` | crates `domain_*`, `ent-events` |
| **L1 — Intégration Postgres local** | Persistance + bus d’événements sur `127.0.0.1` | Avant merge vers `dev` si migration SQL | `ENT_DATABASE_URL` locale uniquement (jamais la RDB prod / `.env.local` distant) |
| **L2 — API** | HTTP (axum…) : auth, tenant, cas nominaux + erreurs | Quand le crate API existe | tests `#[tokio::test]` + éventuellement `cargo nextest` |
| **L3 — Front Rust** | Geste utilisateur observable (écran) | Quand UI existe | tests UI / e2e locaux |
| **L4 — Brain** | Handlers de répercussion (changement classe → domaines) | Dès le bus d’événements | tests de scénario « given event → expect side effects » |

Charlemagne / HFSQL = **référence de couverture**, pas oracle de tests automatiques.

---

## 2. Définition de livrable (DoD) — verticale

Un lot n’est « livré » que si :

1. **Code complet** (pas de TODO / stub qui prétend faire le métier).
2. **Annonce** `PRET:` ou `GROS_PRET:` au Pilote (voir `ent-rust-strategy.md`).
3. **Rôle Tests déclenché** dès « prêt » (pas optionnel).
4. **Tests L0 verts** (+ L1 si SQL) — nécessaires **mais pas suffisants**.
5. **Checklist sécurité multi-tenant** (section 2bis) OK.
6. **Geste / preuve observable** : commande domaine, curl API, ou parcours UI rejouable — pas « ça compile ».
7. **Événements métier** documentés si mutation socle élève.
8. **Scénario Brain** si impact cross-domaine.
9. **RGPD** : pas de nouvelle catégorie sensible sans oui Florian ; payloads minimisés.
10. Si `GROS_PRET:` : vague **Perf** et/ou **UI-Garant** / checklist parcours ; écarts corrigés ou tickets explicites.
11. **Pas d’accès prod** ; migrations local / labo seulement.
12. Doc à jour si le contrat change.

Tant que L2/L3 n’existent pas : crates + L0 (+ L1) + sécu + **preuve de comportement** (test d’intégration / scénario) = minimum « vrai ».

### 2bis. Checklist sécurité (chaque livrable données / API)

- [ ] Toute lecture/écriture filtrée par `etablissement_id`
- [ ] Test négatif cross-tenant (autre établissement → refus / NotFound)
- [ ] Events portent `etablissement_id`
- [ ] Pas de secret en dur
- [ ] Scripts test/migrate refusent une URL non locale
- [ ] (API) le client ne choisit pas le tenant librement sans session

---

## 3. Commandes garde-fou

```bash
cd ent-rust
cargo test
cargo fmt --check   # dès que rustfmt dispo
cargo clippy -- -D warnings   # dès que clippy dispo
```

Postgres local (L1) — même discipline que ScolIA :

- URL uniquement `127.0.0.1`
- Refus explicite d’URL distante Scaleway dans les scripts de test

---

## 4. Ce qu’on vérifie métier (exemples P0)

| Scénario | Attendu |
|---|---|
| Création élève | Snapshot cohérent + event `eleve.created` |
| Changement de classe | Event `scolarite.classe_changed` + hook Brain enregistré |
| Changement de statut (sortie) | Event `eleve.status_changed` |
| Tenant manquant / mauvais `etablissement_id` | Erreur domaine, pas d’event |

---

## 5. Lien avec la validation ScolIA actuelle

Les règles workspace (MCP browser, seed admin…) s’appliquent surtout à l’**intranet Next**.  
Pour `feature/ent-rust`, la validation autonome = **cargo test (+ Postgres local)** jusqu’à ce qu’une UI Rust expose un geste utilisateur.

Quand le front Rust existe : même exigence « geste bout-en-bout local », toujours hors prod.
