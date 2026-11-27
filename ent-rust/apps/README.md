# Apps natives ScolIA / ENT

Deux faces, un cerveau : l’API Rust (`ent-api`) est la seule source de vérité.
Ces dossiers sont des **squelettes** (SwiftUI + Kotlin / Jetpack Compose) — pas Expo.
Ils compilent sur Mac (Xcode) / Android Studio quand disponibles ; le code source
est versionné ici pour avancer sans machine de build native.

| App | Cible | Auth | Surfaces |
|-----|-------|------|----------|
| `ios-famille/` | Parents | `POST /auth/login` + Bearer | Enfants, résumé, sorties, docs, cantine, VS, notes, messagerie, push… |
| `android-famille/` | Idem | Idem | Idem |
| `ios-eleve/` | Élèves | login → `GET /auth/me` (`eleve_id`) | Notes, moyennes, cahier, EDT, messagerie, push |
| `android-eleve/` | Idem | Idem | Idem |

Variables :

- Base API — défaut `http://127.0.0.1:3001` (iOS) / `http://10.0.2.2:3001` (émulateur Android)
- Push : enregistrement après login (`platform`: `ios` \| `android`)

Compte lab : seed `ent-lab` (`parent0-…@ent.lab` / `DevLocalPass1!` ; face élève Harry liée).
