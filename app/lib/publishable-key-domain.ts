/** Décode le domaine Frontend API embarqué dans une clé publishable legacy (pk_live_…). */
export function frontendDomainFromPublishableKey(publishableKey: string): string | null {
  const pk = publishableKey.trim();
  if (!pk.startsWith("pk_")) return null;

  const parts = pk.split("_");
  if (parts.length < 3) return null;

  const b64 = parts.slice(2).join("_");
  try {
    const padded = b64 + "=".repeat((4 - (b64.length % 4)) % 4);
    const decoded = Buffer.from(padded, "base64").toString("utf8").replace(/\$$/, "");
    return decoded || null;
  } catch {
    return null;
  }
}
