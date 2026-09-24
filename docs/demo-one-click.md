# Démo one-click — ENT du matin (25 %)

Pour Florian : **voir** le matin sans taper de mot de passe.

## Ouvrir (Cursor Desktop)

1. Ouvre cet agent : [Continuer plan ScolIA](https://cursor.com/agents/bc-69846bf6-a5b9-5ec7-aedf-a8b98676b9f3) (Cloud Agent en cours).
2. Dans la fenêtre Agents → menu **Forwarded Ports** (éditeur).
3. Port **3000** (Next.js) → **Open in internal browser**.
4. Tu arrives sur **`/demo`** (redirect auto en labo).

Sinon : `http://localhost:3000/demo` une fois le port forwardé.

## Un clic

| Bouton | Destination |
|--------|-------------|
| **Entrer comme parent** | Portail quotidien (`/quotidien`) — Leo JUSTIF 4B |
| **Entrer comme staff** | Intranet (`/dashboard`) — TOTP géré auto |

À balader côté parent : EDT, notes, cahier, messages, carnet, sanctions.  
Côté staff : vie scolaire / appel, notes, cahier.

## Technique

- Runtime `SCOLA_ENV=lab` (bandeau ambre).
- `GET /api/demo/enter?as=parent|staff` — **refusé en prod**.
- Seed : `npm run seed:labo` / `seed:dev`.
- Commit démo : à jour sur `dev`.
