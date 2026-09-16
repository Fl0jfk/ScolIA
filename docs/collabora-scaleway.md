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
| `WOPI_HOST` | Hôte tenant TLS valide joignable depuis Collabora (ex. `https://lpnb.scolia.fr`). Éviter l’apex `scolia.fr` si le cert est invalide ; le token WOPI embarque aussi `dataBucket` pour ne plus dépendre du Host. |
| `WOPI_SIGNING_SECRET` | Secret HMAC (souvent = `BETTER_AUTH_SECRET`) |

## Santé

```bash
curl -sS -o /dev/null -w "%{http_code}\n" \
  https://containerscolia1c9956df-collabora-code.functions.fnc.fr-par.scw.cloud/hosting/discovery
# attendu : 200
```

## Notes ops

- Image : `docker.io/collabora/code:latest` (CODE gratuit).
- Mémoire : 6 Go, `min_scale=1` (évite un cold start trop long ; exit 137 = OOM → remonter la RAM).
- SSL terminé par Scaleway ; Collabora en HTTP interne (`ssl.enable=false`, `ssl.termination=true`).
- `domain` / `aliasgroup1` / `frame_ancestors` doivent inclure tous les hôtes tenant (`lpnb.scolia.fr`, `www.scolia.fr`, …).
- CSP app : `frame-src` autorise `https://*.functions.fnc.fr-par.scw.cloud` + `COLLABORA_URL`.
- CNAME `office.scolia.fr` : uniquement si DNS Domains & DNS est accessible (sinon rester sur l’URL functions).
