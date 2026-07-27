# Mail plateforme — `mailer@scolia.fr` (OVH Starter)

Une seule famille de variables **`MAILER_*`** pour envoi SMTP + polling IMAP + Reply-To.

## Secrets Scaleway

| Variable | Exemple | Rôle |
|----------|---------|------|
| `MAILER_EMAIL` | `mailer@scolia.fr` | From + boîte + Reply-To base |
| `MAILER_PASS` | … | Mot de passe OVH |
| `MAILER_HOST` | `ssl0.ovh.net` | Host SMTP **et** IMAP |
| `MAILER_SMTP_PORT` | `465` | Envoi (défaut 465) |
| `MAILER_IMAP_PORT` | `993` | Polling (défaut 993) |
| `MAILER_ALERT_TO` | `toi@…` | Alerte mail non rattaché |
| `TRAVEL_EMAIL_INGEST_SECRET` | aléatoire | Auth du cron poll-email |

Optionnel (rare) : `MAILER_SMTP_HOST` / `MAILER_IMAP_HOST` si SMTP et IMAP diffèrent.

## Cron

```http
POST https://scolia.fr/api/travels/poll-email
x-travel-email-ingest-secret: <TRAVEL_EMAIL_INGEST_SECRET>
```

Reply-To sorties : `mailer+{slug}@scolia.fr`
