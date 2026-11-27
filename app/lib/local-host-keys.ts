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

/**
 * Un tunnel public est un nom que n'importe qui peut obtenir : hors développement, il faut
 * l'autoriser explicitement, sinon les liens des e-mails pourraient pointer vers un site tiers.
 */
function tunnelHostsAllowed(): boolean {
  return process.env.NODE_ENV !== "production" || process.env.SCOLIA_ALLOW_TUNNEL_HOSTS === "1";
}

export function isLocalDevHostname(hostname: string): boolean {
  const host = normalizeHostname(hostname);
  if (LOCAL_DEV_HOSTS.has(host)) return true;
  // Démo labo exposée via tunnel : même résolution tenant que localhost.
  if (tunnelHostsAllowed() && isLabTunnelHostname(host)) return true;
  return false;
}
