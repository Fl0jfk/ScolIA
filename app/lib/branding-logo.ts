import "server-only";

import { getSignedReadUrl } from "@/app/lib/s3-storage";
import { s3Key } from "@/app/lib/s3-path";
import { parseTravelsS3KeyFromUrl } from "@/app/lib/travels-s3";

/** Hosts Object Storage (AWS / Scaleway) — pas d’URL « publique » brute vers le navigateur. */
function isObjectStorageHostname(host: string): boolean {
  const h = host.toLowerCase();
  return (
    h.endsWith(".amazonaws.com") ||
    h.endsWith(".scw.cloud") ||
    h === "s3.amazonaws.com"
  );
}

/**
 * Normalise une référence logo pour le stockage dans settings/site.json :
 * toujours une clé objet relative (ex. settings/branding/header-logo-….png),
 * jamais une URL absolue S3 (sinon migration de bucket → 403).
 */
export async function normalizeHeaderLogoRefForStorage(
  raw: string | null | undefined,
): Promise<string | undefined> {
  const trimmed = String(raw || "").trim();
  if (!trimmed) return undefined;

  if (!trimmed.startsWith("http://") && !trimmed.startsWith("https://")) {
    return s3Key(trimmed.split("?")[0].split("#")[0]) || undefined;
  }

  try {
    const host = new URL(trimmed).hostname;
    if (!isObjectStorageHostname(host)) {
      // URL externe volontaire (CDN hors S3) — conservée telle quelle.
      return trimmed;
    }
  } catch {
    return undefined;
  }

  const key = await parseTravelsS3KeyFromUrl(trimmed);
  return key ? s3Key(key) : undefined;
}

/**
 * Résout le logo header pour le navigateur : URL signée sur le dataBucket
 * du tenant courant. Jamais de repli sur une URL S3 absolue (privée / legacy).
 */
export async function resolveHeaderLogoDisplayUrl(
  rawRef: string | null | undefined,
): Promise<string | null> {
  const trimmed = String(rawRef || "").trim();
  if (!trimmed) return null;

  if (!trimmed.startsWith("http://") && !trimmed.startsWith("https://")) {
    return (await getSignedReadUrl(trimmed, 3600)) || null;
  }

  try {
    const host = new URL(trimmed).hostname;
    if (!isObjectStorageHostname(host)) {
      return trimmed;
    }
  } catch {
    return null;
  }

  const key = await parseTravelsS3KeyFromUrl(trimmed);
  if (!key) return null;
  return (await getSignedReadUrl(key, 3600)) || null;
}
