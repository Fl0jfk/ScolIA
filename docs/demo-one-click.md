# Démo one-click — ENT du matin (25 %)

## Ouvrir

`npm run dev` puis **http://localhost:3000/demo?dev_tenant=default**

## Un clic

| Bouton | Destination |
|--------|-------------|
| **Entrer comme parent** | Portail quotidien (`/quotidien`) — Leo JUSTIF 4B |
| **Entrer comme staff** | Intranet (`/dashboard`) — TOTP auto |

## Technique

- Runtime `SCOLA_ENV=lab` (bandeau ambre).
- `GET /api/demo/enter?as=parent|staff` — **refusé en prod**.
- Tunnels `*.trycloudflare.com` = hostname labo (comme localhost).
- Avec un serveur en mode production (`next start`, image Docker), les tunnels sont refusés sauf si
  `SCOLIA_ALLOW_TUNNEL_HOSTS=1`. **Ne jamais poser cette variable sur le site réel.**
