#!/usr/bin/env bash
# Copie AWS S3 → Scaleway (NE SUPPRIME RIEN côté AWS).
# rclone copy uniquement.
#
#   brew install rclone
#   export AWS_ACCESS_KEY_ID=... AWS_SECRET_ACCESS_KEY=... AWS_REGION=eu-west-3
#   export SCW_ACCESS_KEY=... SCW_SECRET_KEY=...
#   ./scripts/sync-aws-to-scaleway.sh --dry-run
#   ./scripts/sync-aws-to-scaleway.sh
#   ./scripts/sync-aws-to-scaleway.sh --skip-images   # si IAM sans droit sur scola-image

set -euo pipefail

DRY_RUN=0
SKIP_IMAGES=0
for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    --skip-images) SKIP_IMAGES=1 ;;
  esac
done

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "==> DRY-RUN (aucune écriture)"
fi

AWS_REGION="${AWS_REGION:-eu-west-3}"
SCW_REGION="${SCW_REGION:-fr-par}"
SCW_ENDPOINT="${SCW_ENDPOINT:-https://s3.fr-par.scw.cloud}"
SCW_REGISTRY="${SCW_REGISTRY:-scolia-registry}"
SCW_DATA="${SCW_DATA:-scolia-data}"
SCW_IMAGES="${SCW_IMAGES:-scolia-images}"

: "${AWS_ACCESS_KEY_ID:?Définis AWS_ACCESS_KEY_ID}"
: "${AWS_SECRET_ACCESS_KEY:?Définis AWS_SECRET_ACCESS_KEY}"
: "${SCW_ACCESS_KEY:?Définis SCW_ACCESS_KEY}"
: "${SCW_SECRET_KEY:?Définis SCW_SECRET_KEY}"

if ! command -v rclone >/dev/null 2>&1; then
  echo "rclone manquant → brew install rclone"
  exit 1
fi

TMP_CONF="$(mktemp)"
trap 'rm -f "$TMP_CONF"' EXIT

cat >"$TMP_CONF" <<EOF
[aws]
type = s3
provider = AWS
env_auth = false
access_key_id = ${AWS_ACCESS_KEY_ID}
secret_access_key = ${AWS_SECRET_ACCESS_KEY}
region = ${AWS_REGION}

[scw]
type = s3
provider = Scaleway
env_auth = false
access_key_id = ${SCW_ACCESS_KEY}
secret_access_key = ${SCW_SECRET_KEY}
region = ${SCW_REGION}
endpoint = ${SCW_ENDPOINT}
acl = private
EOF

rcopy() {
  if [[ "$DRY_RUN" -eq 1 ]]; then
    rclone --config "$TMP_CONF" copy --dry-run --progress --transfers=4 --checkers=8 "$@"
  else
    rclone --config "$TMP_CONF" copy --progress --transfers=4 --checkers=8 "$@"
  fi
}

echo "IMPORTANT : AWS n'est jamais modifié ni vidé — copie seule vers Scaleway."
echo ""

echo "==> 1/3 Registry : fl0jfk-tenant-registry/tenants → ${SCW_REGISTRY}/tenants"
rcopy aws:fl0jfk-tenant-registry/tenants "scw:${SCW_REGISTRY}/tenants"

if [[ "$SKIP_IMAGES" -eq 1 ]]; then
  echo "==> 2/3 Images : IGNORÉ (--skip-images)"
else
  echo "==> 2/3 Images : scola-image → ${SCW_IMAGES}"
  if ! rcopy aws:scola-image "scw:${SCW_IMAGES}"; then
    echo ""
    echo "⚠️  Pas d'accès IAM à scola-image (403 ListBucket)."
    echo "    Options :"
    echo "    a) Ajouter s3:ListBucket + s3:GetObject sur arn:aws:s3:::scola-image et scola-image/*"
    echo "       pour l'utilisateur docslapro-platform-master"
    echo "    b) Relancer avec : ./scripts/sync-aws-to-scaleway.sh --skip-images"
    echo "    (on continue quand même avec les données métier)"
    echo ""
  fi
fi

echo "==> 3/3 Données LP : docslapro → ${SCW_DATA}"
PREFIXES=(
  settings travels devis-incoming absences documents internat
  domain-planning reservation-rooms certificates requests
  dashboard toolbox rgpd channels chat news photocopies-couleur
  uploads convocations demandes-hse
)
for p in "${PREFIXES[@]}"; do
  echo "    — $p/"
  rcopy "aws:docslapro/${p}" "scw:${SCW_DATA}/${p}"
done
echo "    — eleves.json"
rcopy aws:docslapro/eleves.json "scw:${SCW_DATA}/"

echo ""
echo "Copie terminée. AWS intact."
echo "Registry secrets vus : tenants/secrets/la-providence-nicolas-barre.json"
echo "Dans ${SCW_REGISTRY}/tenants/index.json vérifier :"
echo "  slug cohérent avec le fichier secrets"
echo "  dataBucket: ${SCW_DATA}"
