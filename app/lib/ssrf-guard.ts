/** Garde anti-SSRF (NJ-10, D-053) : n'autorise que des URL https vers un hôte public nommé. */
export function isSafePublicHttpsUrl(raw: string): boolean {
  let u: URL;
  try {
    u = new URL(raw);
  } catch {
    return false;
  }
  if (u.protocol !== "https:") return false;
  if (u.username || u.password) return false;
  const host = u.hostname.toLowerCase().replace(/\.$/, "");
  if (!host || !host.includes(".")) return false; // pas de nom interne (« metadata », « localhost »)
  if (host.startsWith("[") || /^[0-9.]+$/.test(host)) return false; // pas d'adresse IP littérale
  if (/(^|\.)(localhost|local|internal|intranet|lan|home|corp)$/.test(host)) return false;
  return true;
}
