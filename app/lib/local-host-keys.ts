import { normalizeHostname } from "@/app/lib/tenant-registry";

const LOCAL_DEV_HOSTS = new Set(["localhost", "127.0.0.1"]);

/** Tunnels publics de démo (Cloudflare / ngrok / localtunnel). */
export function isLabTunnelHostname(hostname: string): boolean {
  const host = normalizeHostname(hostname);
  if (!host) return false;
  return (
    host.endsWith(".trycloudflare.com") ||
    host.endsWith(".cloudflared.net") ||
    host.endsWith(".loca.lt") ||
    host.endsWith(".ngrok-free.app") ||
    host.endsWith(".ngrok.io") ||
    host.endsWith(".ngrok.app")
  );
}

export function isLocalDevHostname(hostname: string): boolean {
  const host = normalizeHostname(hostname);
  if (LOCAL_DEV_HOSTS.has(host)) return true;
  // Démo labo exposée via tunnel : même résolution tenant que localhost.
  if (isLabTunnelHostname(host)) return true;
  return false;
}
