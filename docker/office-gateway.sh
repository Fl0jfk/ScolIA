#!/bin/sh
# Reverse-proxy Collabora sur le même hôte que l’ENT (iframe same-origin).
set -eu

NEXT_UPSTREAM_PORT="${NEXT_UPSTREAM_PORT:-3000}"
LISTEN_PORT="${PORT:-8080}"
CONF="/tmp/nginx-office.conf"

mkdir -p /tmp/nginx_client_body /tmp/nginx_proxy /tmp/nginx_fastcgi /tmp/nginx_uwsgi /tmp/nginx_scgi

COLLABORA_HOST=""
COLLABORA_SCHEME="https"
if [ -n "${COLLABORA_URL:-}" ]; then
  COLLABORA_HOST=$(printf '%s' "$COLLABORA_URL" | sed -E 's#^https?://##; s#/.*$##')
  case "$COLLABORA_URL" in
    http://*) COLLABORA_SCHEME="http" ;;
  esac
fi

LISTEN_PORT="$LISTEN_PORT" \
NEXT_UPSTREAM_PORT="$NEXT_UPSTREAM_PORT" \
COLLABORA_HOST="$COLLABORA_HOST" \
COLLABORA_SCHEME="$COLLABORA_SCHEME" \
node <<'JS' > "$CONF"
const fs = require("fs");
let tpl = fs.readFileSync("/app/docker/nginx-office.conf.template", "utf8");
const host = process.env.COLLABORA_HOST || "";
const scheme = process.env.COLLABORA_SCHEME || "https";
let office = "";
if (host) {
  office = fs
    .readFileSync("/app/docker/nginx-office-collabora.conf.template", "utf8")
    .replaceAll("__COLLABORA_SCHEME__", scheme)
    .replaceAll("__COLLABORA_HOST__", host);
}
process.stdout.write(
  tpl
    .replaceAll("__LISTEN_PORT__", process.env.LISTEN_PORT || "8080")
    .replaceAll("__NEXT_UPSTREAM_PORT__", process.env.NEXT_UPSTREAM_PORT || "3000")
    .replaceAll("__OFFICE_LOCATIONS__", office),
);
JS

export PORT="$NEXT_UPSTREAM_PORT"
export HOSTNAME=127.0.0.1
node /app/server.js &
NEXT_PID=$!

PORT="$NEXT_UPSTREAM_PORT" node -e "
const net=require('net');
const port=Number(process.env.PORT||3000);
const once=()=>new Promise((res,rej)=>{const s=net.connect({port,host:'127.0.0.1'},()=>{s.end();res();});
s.setTimeout(1000,()=>{s.destroy();rej(new Error('timeout'));});
s.on('error',rej);});
(async()=>{for(let i=0;i<90;i++){try{await once();process.exit(0);}catch{await new Promise(r=>setTimeout(r,1000));}}
process.exit(1);})();
"

nginx -c "$CONF" -g 'daemon off;' &
NGINX_PID=$!

term() {
  kill "$NEXT_PID" "$NGINX_PID" 2>/dev/null || true
  wait "$NEXT_PID" "$NGINX_PID" 2>/dev/null || true
}
trap term INT TERM
wait "$NGINX_PID" "$NEXT_PID"
term
exit 1
