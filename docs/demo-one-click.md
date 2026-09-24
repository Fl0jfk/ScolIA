# Démo one-click — ENT du matin (25 %)

## URL publique (maintenant)

**https://lecture-settled-invitations-heated.trycloudflare.com/demo?dev_tenant=default**

Un clic parent ou staff. Tunnel Cloudflare → Cloud Agent (tant que l’agent tourne).

## Ouvrir (Cursor Desktop, sans tunnel)

1. Agents Window → **Forwarded Ports** → **3000** → Open in internal browser.
2. Page **`/demo`**.

## Un clic

| Bouton | Destination |
|--------|-------------|
| **Entrer comme parent** | Portail quotidien (`/quotidien`) — Leo JUSTIF 4B |
| **Entrer comme staff** | Intranet (`/dashboard`) — TOTP auto |

## Technique

- Runtime `SCOLA_ENV=lab` (bandeau ambre).
- `GET /api/demo/enter?as=parent|staff` — **refusé en prod**.
- Tunnels `*.trycloudflare.com` = hostname labo (comme localhost).
