/**
 * Fusion des pièces jointes séjour : jamais d’écrasement silencieux.
 * Union serveur ∪ client ; retrait uniquement via `removedAttachmentKeys`.
 */

export type TravelAttachment = {
  name: string;
  url: string;
  s3Key?: string;
  source?: string;
  gmailMessageId?: string;
};

/** Clé stable pour dédupliquer / retirer une PJ. */
export function travelAttachmentIdentityKey(
  att: Pick<TravelAttachment, "s3Key" | "url" | "name"> | null | undefined,
): string | null {
  if (!att) return null;
  const s3 = String(att.s3Key || "").trim();
  if (s3) return `s3:${s3}`;
  const url = String(att.url || "").trim();
  if (url) {
    try {
      const u = new URL(url);
      const path = decodeURIComponent(u.pathname.replace(/^\/+/, ""));
      if (path.includes("attachments/")) return `s3:${path.includes("attachments/") ? path.slice(path.indexOf("attachments/")) : path}`;
    } catch {
      /* ignore */
    }
    return `url:${url}`;
  }
  const name = String(att.name || "").trim().toLowerCase();
  return name ? `name:${name}` : null;
}

function normalizeRemovedKey(raw: string): string {
  const s = String(raw || "").trim();
  if (!s) return "";
  if (s.startsWith("s3:") || s.startsWith("url:") || s.startsWith("name:")) return s;
  if (s.includes("attachments/")) {
    const idx = s.indexOf("attachments/");
    return `s3:${s.slice(idx)}`;
  }
  if (/^https?:\/\//i.test(s)) {
    try {
      const u = new URL(s);
      const path = decodeURIComponent(u.pathname.replace(/^\/+/, ""));
      if (path.includes("attachments/")) {
        return `s3:${path.slice(path.indexOf("attachments/"))}`;
      }
    } catch {
      /* ignore */
    }
    return `url:${s}`;
  }
  return `s3:${s}`;
}

/**
 * Fusionne les PJ.
 * - Tout ce qui est sur le serveur et pas dans `removedKeys` est conservé.
 * - Les PJ client s’ajoutent / mettent à jour (même clé).
 * - Sans `removedKeys`, une liste client plus courte ne peut plus effacer le reste.
 */
export function mergeTravelAttachments(opts: {
  fromClient: unknown;
  fromServer: unknown;
  removedKeys?: unknown;
}): TravelAttachment[] {
  const server = Array.isArray(opts.fromServer)
    ? (opts.fromServer as TravelAttachment[])
    : [];
  const client = Array.isArray(opts.fromClient)
    ? (opts.fromClient as TravelAttachment[])
    : null;
  const removed = new Set<string>();
  if (Array.isArray(opts.removedKeys)) {
    for (const raw of opts.removedKeys) {
      const k = normalizeRemovedKey(String(raw || ""));
      if (k) removed.add(k);
    }
  }

  const map = new Map<string, TravelAttachment>();

  for (const att of server) {
    if (!att || typeof att !== "object") continue;
    const key = travelAttachmentIdentityKey(att);
    if (!key || removed.has(key)) continue;
    map.set(key, {
      name: String(att.name || "document"),
      url: String(att.url || ""),
      ...(att.s3Key ? { s3Key: String(att.s3Key) } : {}),
      ...(att.source ? { source: String(att.source) } : {}),
      ...(att.gmailMessageId ? { gmailMessageId: String(att.gmailMessageId) } : {}),
    });
  }

  if (client) {
    for (const att of client) {
      if (!att || typeof att !== "object") continue;
      const key = travelAttachmentIdentityKey(att);
      if (!key || removed.has(key)) continue;
      map.set(key, {
        name: String(att.name || "document"),
        url: String(att.url || ""),
        ...(att.s3Key ? { s3Key: String(att.s3Key) } : {}),
        ...(att.source ? { source: String(att.source) } : {}),
        ...(att.gmailMessageId ? { gmailMessageId: String(att.gmailMessageId) } : {}),
      });
    }
  }

  return [...map.values()].filter((a) => a.url || a.s3Key);
}
