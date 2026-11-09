/**
 * Liste blanche des services de push des navigateurs.
 * Sans elle, un utilisateur pourrait enregistrer une URL arbitraire et faire émettre
 * au serveur des requêtes vers le réseau interne (SSRF).
 */
const EXACT_HOSTS = new Set(["fcm.googleapis.com", "web.push.apple.com"]);
const HOST_SUFFIXES = [".push.services.mozilla.com", ".notify.windows.com", ".push.apple.com"];

export function isAllowedPushEndpoint(raw: string): boolean {
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    return false;
  }
  if (url.protocol !== "https:") return false;
  if (url.username || url.password) return false;
  if (url.port && url.port !== "443") return false;
  const host = url.hostname.toLowerCase();
  if (EXACT_HOSTS.has(host)) return true;
  return HOST_SUFFIXES.some((suffix) => host.endsWith(suffix) && host.length > suffix.length);
}
