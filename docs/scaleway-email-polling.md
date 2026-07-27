# Mail plateforme — TEM (envoi) + OVH (réception)

Sur **Serverless Containers**, Scaleway bloque le SMTP sortant vers OVH
(`Connection timeout` sur `ssl0.ovh.net:465`). Seul
`smtp.tem.scaleway.com` (Transactional Email) est autorisé.

Architecture :

| Canal | Service | Variables |
|-------|---------|-----------|
| **Envoi SMTP** | Scaleway TEM | `MAILER_SMTP_*` + `MAILER_EMAIL` (From) |
| **Réception IMAP** | OVH (`ssl0.ovh.net`) | `MAILER_HOST` + `MAILER_PASS` + `MAILER_EMAIL` |
| **Reply-To** | alias boîte OVH | `mailer+{slug}@scolia.fr` |

## Prérequis console Scaleway (TEM)

1. **Transactional Email** → ajouter le domaine **`scolia.fr`**.
2. Publier les DNS (SPF / DKIM / éventuellement MX) chez le registrar.
3. Attendre le statut **checked / verified**.
4. Noter le **Project ID** (username SMTP TEM).
5. Créer une **clé API IAM** avec permission Transactional Email (secret = password SMTP).

## Secrets runtime conteneur

### Envoi (TEM)

| Variable | Exemple | Rôle |
|----------|---------|------|
| `MAILER_EMAIL` | `mailer@scolia.fr` | From + boîte IMAP + Reply-To base |
| `MAILER_SMTP_HOST` | `smtp.tem.scaleway.com` | Host SMTP autorisé depuis Containers |
| `MAILER_SMTP_PORT` | `587` | STARTTLS (alt. `2465` TLS) |
| `MAILER_SMTP_SECURE` | `false` | Obligatoire si port 587 |
| `MAILER_SMTP_USER` | `02328746-5ac6-4fb9-af80-baa1591d5d2d` | Project ID (username SMTP TEM) |
| `MAILER_SMTP_PASS` | `<secret_key IAM>` | Password SMTP TEM (clé secrète API) |

### Réception (OVH IMAP)

| Variable | Exemple | Rôle |
|----------|---------|------|
| `MAILER_HOST` | `ssl0.ovh.net` | Host IMAP (pas le host TEM) |
| `MAILER_PASS` | … | Mot de passe **boîte** OVH |
| `MAILER_IMAP_PORT` | `993` | Polling (défaut 993) |

### Autres

| Variable | Exemple | Rôle |
|----------|---------|------|
| `MAILER_ALERT_TO` | `toi@…` | Alerte mail non rattaché |
| `TRAVEL_EMAIL_INGEST_SECRET` | aléatoire | Auth du cron poll-email |

Optionnel : `MAILER_IMAP_HOST` si l’IMAP diffère de `MAILER_HOST`.

> Ne jamais mettre le secret IAM TEM dans `MAILER_PASS` : IMAP casserait.
> Ne jamais mettre `ssl0.ovh.net` dans `MAILER_SMTP_HOST` sur Containers : timeout.

## Cron

```http
POST https://scolia.fr/api/travels/poll-email
x-travel-email-ingest-secret: <TRAVEL_EMAIL_INGEST_SECRET>
```

Reply-To sorties : `mailer+{slug}@scolia.fr`
