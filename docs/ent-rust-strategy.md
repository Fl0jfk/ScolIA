# Stratégie A→Z — chantier ENT Rust (v3.1)

> Organisation du chantier.  
> Complète : `ent-vision-rust.md`, `ent-rust-quality.md`, `ent-metier-checklist.md`.

**Priorité n°1 :** sécurité multi-tenant + données élèves (RGPD).  
**Interdit absolu :** toucher la **base prod** / RDB distante Scaleway (y compris la `DATABASE_URL` « classique » du `.env.local` si elle n’est pas `127.0.0.1`). Chantier ENT Rust = **Postgres local only** (`ENT_DATABASE_URL` → loopback).  
**Priorité chantier :** **vrais livrables** (geste + sécu + preuve), pas du « cargo test vert = fini ».  
Revue **après « j’ai fini »** — noyau sur chaque `PRET:`, **vague qualité** (UI / Perf / Challenger…) dès qu’un **gros morceau / fin de couche** est prêt.

---

## 1. Verdict

**Circuit adopté :**

```
Coder : « tranche prête » (PRET: ou GROS_PRET:)
    → Pilote ouvre un panel de revue
        → Sécurité + Tests (+ RGPD si besoin)     ← toujours
        → si GROS_PRET / fin de couche :
              + UI-Garant (si UI) + Perf + Challenger (option) + anti-régression élargie
    → Coder peut avancer ailleurs en parallèle
    → retours → Pilote
        → OK → LIVRÉ (vrai)
        → KO → rappel Coder → re-revue
```

**Pas** 12 bots non-stop. **Oui** : dès qu’un gros lot est prêt, les renforts **passent dessus** pour améliorer / challenger — c’est ça un livrable de qualité.

---

## 2. Circuit livrable (détail)

### 2.1 Deux tailles d’annonce

| Annonce | Quand | Qui passe derrière |
|---|---|---|
| `PRET:` | Petit lot / correctif / amorce | **Noyau** : Sécu + Tests (+ RGPD si besoin) |
| `GROS_PRET:` ou fin de couche (ex. « VS appel+absences OK », « première API auth+tenant ») | Gros morceau métier / couche terminée | **Noyau + vague qualité** : Perf, UI-Garant (si UI), Challenger (décisions / approche), anti-régression élargie |

Le Coder (ou le Pilote) **étiquette** explicitement `PRET:` vs `GROS_PRET:` — pas de flou.

### 2.2 Étapes

| Étape | Qui | Règle |
|---|---|---|
| 1. Implémentation | **Coder** | Ticket borné |
| 2. Annonce | **Coder → Pilote** | `PRET:` ou `GROS_PRET:` + fichiers / events / geste utilisateur attendu |
| 3. Dispatch | **Pilote** | Noyau toujours ; renforts si gros |
| 4. Parallélisme | **Coder** | Peut enchaîner ailleurs *sans dépendance* au lot en revue |
| 5. Rapports | Revues → **Pilote** | `OK` / `KO` + écarts **actionnables** (pas du blabla) |
| 6a. Livré | **Pilote** | Tampon **LIVRÉ** seulement si critères « vrai livrable » (ci-dessous) |
| 6b. Correctifs | **Pilote → Coder** | Rappel même ticket |

**Blocage intelligent :** la revue bloque le tampon **livré**, pas le Coder sur autre chose.

---

## 3. Priorités

| Rang | Priorité |
|---:|---|
| 1 | Isolation tenant + sécu générale (une Postgres, N établissements) |
| 2 | RGPD / minimisation (élèves, familles) |
| 3 | Socle élève + Brain |
| 4 | Verticales métier |
| 5 | IA sur Brain |

Rust = aussi choix de **robustesse / surface d’attaque**, pas seulement perf.

---

## 4. Équipe v3 — noyau vs renfort

### 4.1 Noyau (toujours sur un livrable)

| Rôle | Mission | Déclencheur |
|---|---|---|
| **Pilote** | Tickets, dispatch revue, arbitrage, rappel Coder | Permanent |
| **Coder** | Implémente la tranche Rust | Ticket ouvert / rappel correctifs |
| **Sécurité** | Tenant, authz, secrets, anti-prod, tests isolation | Dès `PRET:` |
| **Tests (QA lot)** | DoD L0–L4, non-régression du lot, scénario Brain | Dès `PRET:` |
| **RGPD / Conformité** | Nouveau traitement / donnée sensible → questions oui/non Florian | Dès `PRET:` **si** déclencheur RGPD |

### 4.2 Renforts — vague qualité sur `GROS_PRET:` / fin de couche

Pas non-stop. **Dès qu’un gros morceau est prêt**, ils passent **par-dessus** le livrable pour trouver ce qui cloche / ce qu’on peut améliorer.

| Rôle | Mission sur un gros livrable |
|---|---|
| **Challenger** | Conteste l’approche (sécu, modèle, simplicité) — peu cher, utile sur les gros choix ; OK aussi en fin de couche si le Pilote a tranché vite |
| **Perf** | Latences, démarrage, requêtes N+1, budgets — dès qu’il y a API ou UI mesurable |
| **UI-Garant** | Le geste / l’UI demandée est bien là et cohérente — dès qu’il y a front (tests UI fragiles → compléter par checklist manuelle + snapshots / parcours scripté local) |
| **Anti-régression** | Suite élargie + scénarios des tranches précédentes toujours verts |
| **Spec** | Met à jour le contrat court du lot (events, API) pour la suite |
| **CI-Fix** | Si la CI casse sur la branche |
| **Veille ENT** | Hors chemin critique |

### 4.3 Vrai livrable ≠ « tests verts »

Un tampon **LIVRÉ** (surtout `GROS_PRET:`) exige :

1. Geste ou contrat **observable** (même CLI/API curl en attendant le front) — pas seulement une lib qui compile  
2. Noyau Sécu + Tests **OK**  
3. Preuve d’isolation tenant si données  
4. Sur gros lot : au moins un passage **Perf** et/ou **UI-Garant** / parcours manuel documenté si UI  
5. Écarts des renforts soit **corrigés**, soit **explicitement reportés** (ticket, pas balayés sous le tapis)

Les tests UI cassent souvent : on ne fait pas semblant. On combine **parcours scripté local** + **checklist geste** + correctifs jusqu’à ce que ce soit **rejouable**, pas du pipeau.

### 4.4 Stack Grokbot

Catalogue **oui**. Activation permanente **non**.  
Noyau à chaque `PRET:` ; vague qualité à chaque `GROS_PRET:` / fin de couche.

---

## 5. Sécurité multi-tenant (rappel)

- Toute commande / requête filtrée par `etablissement_id`
- Events toujours tenantés
- Tests négatifs cross-tenant obligatoires sur livrable données/API
- Scripts : refus URL non-`127.0.0.1`
- Secrets via env seulement

Voir checklist dans `ent-rust-quality.md` §2bis.

---

## 6. RGPD (oui/non Florian, pas un mur)

| Cas | Action |
|---|---|
| Données scolaires déjà prévues | OK documenté, pas de ping |
| Santé, tracking fin, export massif, nouvelle finalité | **Question oui/non à Florian** avant code |
| Payload d’event | Minimiser |

---

## 7. Local vs Cloud

| Local | Cloud Agent |
|---|---|
| Pilote, contrats, sécu socle, Postgres local, Alcuin | Coder sur slice isolée *après* contrat figé |
| Revue Sécu/Tests sur `PRET:` | Possible en revue parallèle |
| Jamais prod sans validation humaine | Idem |

---

## 8. Roadmap A→Z

```
A. Contrats events + Eleve (+ isolation)     ← fait
B. L1 Postgres local + garde URL             ← fait
C. Brain unifié (répercussions all-in-one)   ← fait (carte : docs/ent-rust-transverse.md)
D. API Rust axum + auth PG finale            ← fait
E. Noyau enrichi : dates d’effet, foyers/liens, matières/notes, appel→absence
F. Labo multi-personas (vie sur base vierge) ← amorcé (`seed_labo_cast` + scenarios)
G. Front Leptos (Natif) ; Lite = apps natives plus tard
H. IA sur Brain                              ← amorcé : Copilote déterministe (intentions sur données centrales), `POST /copilote/ask` + UI staff ; LLM optionnel plus tard
I. Labo → prod ENT (validation humaine) — hors touch prod ScolIA Next
```

**Continuité métier :** `docs/metier-*.md`, `mission.md`, journal, cycles = **déjà travaillé** — on ne redécouvre pas de zéro ; on code dessus.

**Rappel produit (tout l’ENT) :** données réutilisables au centre ; modules piochent ; élève au-dessus ; dates d’effet ; Natif/Lite.

**Labo :** prioriser des vies autonomes sur Postgres local (personas) plutôt que seulement des unitaires. Les exemples de conversation Florian = éclairage, **pas** backlog figé.

Chaque lettre → `PRET:` / panel revue.

---

## 9. Anti-patterns

- Skip Tests/Sécu « c’est petit »  
- Challenger sur chaque rename  
- 3 Coders sur le même socle  
- Coder bloqué à attendre la revue alors qu’il pourrait avancer ailleurs  
- 12 bots actifs en permanence  
- `DATABASE_URL` prod  
- **Repartir de zéro** en ignorant les dictées / journal déjà dans `docs/`  
- Copier un exemple oral dans le code sans jugement produit  

---

## 10. Prochaine action

1. Élargir le labo (vies croisées) sur le modèle déjà là — observer les trous.  
2. Continuer **E** selon checklist métier + imagination produit (pas recopier des illustrations orales).  
3. Auth Rust reste le chemin final (pas de Better-Auth).
