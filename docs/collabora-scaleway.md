# Collabora CODE sur Scaleway

Stack **100 % open source** (MPL), **0 € de licence**. Coût = uniquement le compute Scaleway.

## Conteneurs (namespace `container-scolia`)

| Conteneur | Rôle | URL |
|-----------|------|-----|
| `container-app` | Next.js Scola | https://scolia.fr |
| `collabora-code` | Collabora Online Development Edition | https://containerscolia1c9956df-collabora-code.functions.fnc.fr-par.scw.cloud |

## Variables runtime sur `container-app`

| Variable | Valeur |
|----------|--------|
| `COLLABORA_URL` | URL HTTPS du conteneur Collabora |
| `WOPI_HOST` | `https://scolia.fr` (URL publique de Scola, joignable depuis Collabora) |
| `WOPI_SIGNING_SECRET` | Secret HMAC (souvent = `BETTER_AUTH_SECRET`) |

## Santé

```bash
curl -sS -o /dev/null -w "%{http_code}\n" \
  https://containerscolia1c9956df-collabora-code.functions.fnc.fr-par.scw.cloud/hosting/discovery
# attendu : 200
```

## Notes ops

- Image : `docker.io/collabora/code:latest` (CODE gratuit).
- Mémoire : 4 Go, `min_scale=1` (évite un cold start trop long).
- SSL terminé par Scaleway ; Collabora en HTTP interne (`ssl.enable=false`, `ssl.termination=true`).
- Optionnel plus tard : CNAME `office.scolia.fr` → domaine functions, puis mettre à jour `COLLABORA_URL`.
